//! 실행 비교용 최소 MySQL 서버(와이어 프로토콜). `UsedSizeTest`가 보내는 사용량 조회에 정해 둔 행을 순서대로 돌려준다.
//!
//! - 인증은 검사하지 않고 항상 통과시킨다(`mysql_native_password`를 알리고, 클라이언트 응답은 읽고 버린다). TLS는 알리지 않는다.
//! - `SELECT filecount, used FROM bucket WHERE bucket = '...'`은 `rows`에서 하나씩 꺼내 돌려준다(`None`이면 행 없음, 다 쓰면 행 없음).
//! - 그 밖의 질의(연결 직후 드라이버가 보내는 `SET`·`SELECT @@...` 등)는 모두 기록하고 빈 OK로 답한다.
//!   `SELECT @@...`·`SHOW ...`처럼 결과 집합을 기대하는 질의는 열 이름을 질의에서 읽어 한 행짜리 값으로 답한다.
//! - 받은 질의는 `log`에 쌓는다(사용량 조회만 기준 출력에 남긴다).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct FakeMysql {
    pub port: u16,
    pub queries: Arc<Mutex<Vec<String>>>,
    all: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for FakeMysql {
    fn drop(&mut self) {
        self.task.abort();
    }
}

type Rows = Arc<Mutex<VecDeque<Option<(i64, i64)>>>>;

impl FakeMysql {
    pub async fn start(rows: Vec<Option<(i64, i64)>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let rows: Rows = Arc::new(Mutex::new(rows.into()));
        let queries = Arc::new(Mutex::new(Vec::new()));
        let all = Arc::new(Mutex::new(Vec::new()));
        let task = {
            let (rows, queries, all) = (rows.clone(), queries.clone(), all.clone());
            tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let (rows, queries, all) = (rows.clone(), queries.clone(), all.clone());
                    tokio::spawn(async move {
                        let _ = serve(stream, rows, queries, all).await;
                    });
                }
            })
        };
        Self {
            port,
            queries,
            all,
            task,
        }
    }

    /// 받은 모든 질의(디버깅용).
    #[allow(dead_code)]
    pub fn all_queries(&self) -> Vec<String> {
        self.all.lock().unwrap().clone()
    }
}

async fn read_packet(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    let len = usize::from(header[0]) | usize::from(header[1]) << 8 | usize::from(header[2]) << 16;
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    Ok((header[3], payload))
}

async fn write_packet(stream: &mut TcpStream, seq: u8, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len();
    let mut packet = vec![len as u8, (len >> 8) as u8, (len >> 16) as u8, seq];
    packet.extend_from_slice(payload);
    stream.write_all(&packet).await
}

fn lenenc(out: &mut Vec<u8>, value: &[u8]) {
    // 250 미만의 길이만 쓴다.
    out.push(value.len() as u8);
    out.extend_from_slice(value);
}

fn ok_packet() -> Vec<u8> {
    // OK, 영향 행 0, 마지막 삽입 ID 0, 상태 AUTOCOMMIT, 경고 0
    vec![0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00]
}

fn eof_packet() -> Vec<u8> {
    vec![0xfe, 0x00, 0x00, 0x02, 0x00]
}

fn column_definition(name: &str, kind: u8) -> Vec<u8> {
    let mut out = Vec::new();
    lenenc(&mut out, b"def");
    lenenc(&mut out, b"");
    lenenc(&mut out, b"");
    lenenc(&mut out, b"");
    lenenc(&mut out, name.as_bytes());
    lenenc(&mut out, name.as_bytes());
    out.push(0x0c);
    // 문자 집합: 숫자는 binary(63), 문자열은 utf8mb4(45)
    let charset: u16 = if kind == 0x08 { 63 } else { 45 };
    out.extend_from_slice(&charset.to_le_bytes());
    out.extend_from_slice(&255u32.to_le_bytes());
    out.push(kind);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.push(0);
    out.extend_from_slice(&[0, 0]);
    out
}

/// 열 이름과 행(문자열 값)으로 결과 집합을 보낸다.
async fn send_result_set(
    stream: &mut TcpStream,
    columns: &[(&str, u8)],
    rows: &[Vec<String>],
) -> std::io::Result<()> {
    let mut seq = 1u8;
    write_packet(stream, seq, &[columns.len() as u8]).await?;
    for (name, kind) in columns {
        seq += 1;
        write_packet(stream, seq, &column_definition(name, *kind)).await?;
    }
    seq += 1;
    write_packet(stream, seq, &eof_packet()).await?;
    for row in rows {
        let mut payload = Vec::new();
        for value in row {
            lenenc(&mut payload, value.as_bytes());
        }
        seq += 1;
        write_packet(stream, seq, &payload).await?;
    }
    seq += 1;
    write_packet(stream, seq, &eof_packet()).await
}

/// 드라이버가 연결 직후 읽는 서버 변수·함수 값.
fn system_value(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.trim_start_matches('@') {
        "max_allowed_packet" => "67108864",
        "character_set_client" | "character_set_connection" => "utf8mb4",
        "license" => "GPL",
        "sql_mode" => "",
        "lower_case_table_names" => "0",
        "autocommit" | "auto_increment_increment" => "1",
        "wait_timeout" => "28800",
        n if n.starts_with("timediff") => "00:00:00",
        _ => "0",
    }
    .to_string()
}

async fn serve(
    mut stream: TcpStream,
    rows: Rows,
    queries: Arc<Mutex<Vec<String>>>,
    all: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    // 핸드셰이크
    let capabilities: u32 = 0x1
        | 0x2
        | 0x4
        | 0x8
        | 0x200
        | 0x2000
        | 0x8000
        | 0x1_0000
        | 0x2_0000
        | 0x4_0000
        | 0x8_0000
        | 0x10_0000
        | 0x20_0000;
    let mut hello = vec![10u8];
    hello.extend_from_slice(b"5.7.99-fake\0");
    hello.extend_from_slice(&1u32.to_le_bytes());
    hello.extend_from_slice(b"abcdefgh");
    hello.push(0);
    hello.extend_from_slice(&((capabilities & 0xffff) as u16).to_le_bytes());
    hello.push(33);
    hello.extend_from_slice(&2u16.to_le_bytes());
    hello.extend_from_slice(&((capabilities >> 16) as u16).to_le_bytes());
    hello.push(21);
    hello.extend_from_slice(&[0u8; 10]);
    hello.extend_from_slice(b"ijklmnopqrst\0");
    hello.extend_from_slice(b"mysql_native_password\0");
    write_packet(&mut stream, 0, &hello).await?;
    let (seq, _response) = read_packet(&mut stream).await?;
    write_packet(&mut stream, seq.wrapping_add(1), &ok_packet()).await?;

    // 명령
    loop {
        let (_, payload) = read_packet(&mut stream).await?;
        let Some((&command, rest)) = payload.split_first() else {
            continue;
        };
        match command {
            // COM_QUIT
            0x01 => return Ok(()),
            // COM_QUERY
            0x03 => {
                let sql = String::from_utf8_lossy(rest).into_owned();
                all.lock().unwrap().push(sql.clone());
                let lower = sql.trim().to_ascii_lowercase();
                if lower.starts_with("select filecount, used from bucket") {
                    queries.lock().unwrap().push(sql.clone());
                    let row = rows.lock().unwrap().pop_front().flatten();
                    let data: Vec<Vec<String>> = row
                        .map(|(count, used)| vec![vec![count.to_string(), used.to_string()]])
                        .unwrap_or_default();
                    send_result_set(&mut stream, &[("filecount", 0x08), ("used", 0x08)], &data)
                        .await?;
                } else if lower == "show collation" {
                    // MySql.Data가 연결할 때 문자 집합 번호를 읽는다.
                    let columns = [
                        ("Collation", 0xfd),
                        ("Charset", 0xfd),
                        ("Id", 0x08),
                        ("Default", 0xfd),
                        ("Compiled", 0xfd),
                        ("Sortlen", 0x08),
                    ];
                    let rows = [
                        ["utf8_general_ci", "utf8", "33", "Yes", "Yes", "1"],
                        ["utf8mb4_general_ci", "utf8mb4", "45", "Yes", "Yes", "1"],
                        ["binary", "binary", "63", "Yes", "Yes", "1"],
                    ]
                    .map(|row| row.map(str::to_string).to_vec());
                    send_result_set(&mut stream, &columns, &rows).await?;
                } else if lower.starts_with("select") || lower.starts_with("show") {
                    // 드라이버의 준비 질의(`SELECT @@변수, ...`·`SELECT 함수(...)`): 열마다 값 하나
                    let names: Vec<String> = sql
                        .trim()
                        .get(6..)
                        .unwrap_or_default()
                        .split(',')
                        .map(|c| c.trim().to_string())
                        .collect();
                    let columns: Vec<(&str, u8)> =
                        names.iter().map(|n| (n.as_str(), 0xfd)).collect();
                    let row = names.iter().map(|n| system_value(n)).collect();
                    send_result_set(&mut stream, &columns, &[row]).await?;
                } else {
                    write_packet(&mut stream, 1, &ok_packet()).await?;
                }
            }
            // COM_PING·COM_INIT_DB·COM_RESET_CONNECTION
            _ => write_packet(&mut stream, 1, &ok_packet()).await?,
        }
    }
}
