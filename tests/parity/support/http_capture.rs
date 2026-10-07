//! parity 테스트용 HTTP 캡처 서버. 오라클(`tools/dotnet-oracle`)의 캡처 서버와 같은 방식으로
//! 요청을 원문 그대로 기록하고, 연결마다 같은 응답을 돌려준다(재시도 확인용).

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    pub line: String,
    pub headers: Vec<(String, String)>,
    /// 본문 원문. `Transfer-Encoding: chunked`면 HTTP 청크 구분자까지 포함한다.
    pub body: String,
}

impl CapturedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CannedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub struct CaptureServer {
    pub port: u16,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    task: tokio::task::JoinHandle<()>,
}

impl CaptureServer {
    pub async fn start(response: CannedResponse) -> Self {
        Self::start_with_routes(response, Vec::new()).await
    }

    /// 요청 줄에 `contains`가 들어 있으면 해당 응답을, 아니면 `response`를 돌려준다(오라클의 `Routes`와 같다).
    pub async fn start_with_routes(
        response: CannedResponse,
        routes: Vec<(String, CannedResponse)>,
    ) -> Self {
        Self::start_with_delays(response, routes, Vec::new()).await
    }

    /// `start_with_routes`에 더해, 요청 줄에 `contains`가 들어 있으면 응답을 `delay`만큼 늦춘다(시간 제한 시나리오에서
    /// 요청 횟수를 정하는 용도).
    pub async fn start_with_delays(
        response: CannedResponse,
        routes: Vec<(String, CannedResponse)>,
        delays: Vec<(String, std::time::Duration)>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let shared = requests.clone();
        let routes = Arc::new(routes);
        let delays = Arc::new(delays);
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let requests = shared.clone();
                let response = response.clone();
                let routes = routes.clone();
                let delays = delays.clone();
                tokio::spawn(async move {
                    let _ = serve(socket, &response, &routes, &delays, &requests).await;
                });
            }
        });
        Self {
            port,
            requests,
            task,
        }
    }

    /// 지금까지 받은 요청. 서버는 계속 동작한다.
    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for CaptureServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_line(socket: &mut TcpStream) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while !line.ends_with(b"\r\n") {
        if socket.read(&mut byte).await.ok()? == 0 {
            return None;
        }
        line.push(byte[0]);
    }
    Some(line)
}

async fn serve(
    mut socket: TcpStream,
    response: &CannedResponse,
    routes: &[(String, CannedResponse)],
    delays: &[(String, std::time::Duration)],
    requests: &Mutex<Vec<CapturedRequest>>,
) -> Option<()> {
    loop {
        let mut head = Vec::new();
        loop {
            let line = read_line(&mut socket).await?;
            let end = line == b"\r\n";
            head.extend_from_slice(&line);
            if end {
                break;
            }
        }
        let head = String::from_utf8(head).ok()?;
        let mut lines = head.split("\r\n").filter(|l| !l.is_empty());
        let line = lines.next()?.to_string();
        let headers: Vec<(String, String)> = lines
            .filter_map(|h| h.split_once(':'))
            .map(|(n, v)| (n.to_string(), v.trim().to_string()))
            .collect();
        let request = CapturedRequest {
            line,
            headers,
            body: String::new(),
        };
        if request
            .header("Expect")
            .is_some_and(|v| v.eq_ignore_ascii_case("100-continue"))
        {
            socket
                .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
                .await
                .ok()?;
        }
        let mut body = Vec::new();
        if request
            .header("Transfer-Encoding")
            .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
        {
            loop {
                let size_line = read_line(&mut socket).await?;
                body.extend_from_slice(&size_line);
                let text = String::from_utf8_lossy(&size_line);
                let size = usize::from_str_radix(text.trim().split(';').next()?, 16).ok()?;
                if size == 0 {
                    loop {
                        let trailer = read_line(&mut socket).await?;
                        body.extend_from_slice(&trailer);
                        if trailer == b"\r\n" {
                            break;
                        }
                    }
                    break;
                }
                let mut chunk = vec![0u8; size + 2];
                socket.read_exact(&mut chunk).await.ok()?;
                body.extend_from_slice(&chunk);
            }
        } else {
            let length: usize = request
                .header("Content-Length")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            body.resize(length, 0);
            socket.read_exact(&mut body).await.ok()?;
        }
        let is_head = request.line.starts_with("HEAD ");
        let response = routes
            .iter()
            .find(|(contains, _)| request.line.contains(contains.as_str()))
            .map_or(response, |(_, r)| r);
        let delay = delays
            .iter()
            .find(|(contains, _)| request.line.contains(contains.as_str()))
            .map(|(_, delay)| *delay);
        requests.lock().unwrap().push(CapturedRequest {
            body: String::from_utf8_lossy(&body).into_owned(),
            ..request
        });
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }

        let extra: String = response
            .headers
            .iter()
            .map(|(n, v)| format!("{n}: {v}\r\n"))
            .collect();
        let head = format!(
            "HTTP/1.1 {} Status\r\n{extra}Content-Length: {}\r\n\r\n",
            response.status,
            response.body.len()
        );
        socket.write_all(head.as_bytes()).await.ok()?;
        if !is_head {
            socket.write_all(response.body.as_bytes()).await.ok()?;
        }
        socket.flush().await.ok()?;
    }
}
