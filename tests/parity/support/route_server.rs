//! 요청 줄에 맞는 경로(route)의 응답을 돌려주는 캡처 서버. 오라클의 `ServeUpDown`과 같은 방식이다.
//! 요청을 받을 때마다(응답하기 전) `hook`을 부르므로, 끝없이 도는 부하 루프를 정해진 요청 수에서 멈출 수 있다.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Debug)]
pub struct Route {
    pub contains: String,
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

/// 사례 JSON의 `routes` 배열을 읽는다.
pub fn routes(value: &Value) -> Vec<Route> {
    value
        .as_array()
        .map(|routes| {
            routes
                .iter()
                .map(|r| Route {
                    contains: r["contains"].as_str().unwrap().to_string(),
                    status: r["status"].as_u64().unwrap() as u16,
                    body: r
                        .get("responseBody")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    headers: r
                        .get("responseHeaders")
                        .and_then(Value::as_object)
                        .map(|h| {
                            h.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub type Hook = Arc<dyn Fn(&str) + Send + Sync>;

pub struct RouteServer {
    pub port: u16,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for RouteServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl RouteServer {
    /// 서버를 띄운다. 받은 요청 줄은 `lines`에 넣고 `hook`을 부른다.
    pub async fn start(routes: Vec<Route>, lines: Arc<Mutex<Vec<String>>>, hook: Hook) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let routes = Arc::new(routes);
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                tokio::spawn(serve(socket, routes.clone(), lines.clone(), hook.clone()));
            }
        });
        Self { port, task }
    }
}

async fn read_line(socket: &mut TcpStream) -> Option<String> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while !line.ends_with(b"\r\n") {
        if socket.read(&mut byte).await.ok()? == 0 {
            return None;
        }
        line.push(byte[0]);
    }
    String::from_utf8(line).ok()
}

async fn serve(
    mut socket: TcpStream,
    routes: Arc<Vec<Route>>,
    lines: Arc<Mutex<Vec<String>>>,
    hook: Hook,
) -> Option<()> {
    loop {
        let mut head = Vec::new();
        loop {
            let line = read_line(&mut socket).await?;
            if line == "\r\n" {
                break;
            }
            head.push(line.trim_end().to_string());
        }
        let header = |name: &str| {
            head.iter().skip(1).find_map(|h| {
                let (n, v) = h.split_once(':')?;
                n.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
            })
        };
        if header("Expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
            socket
                .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
                .await
                .ok()?;
        }
        if header("Transfer-Encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
            loop {
                let size_line = read_line(&mut socket).await?;
                let size = usize::from_str_radix(size_line.trim().split(';').next()?, 16).ok()?;
                if size == 0 {
                    while read_line(&mut socket).await? != "\r\n" {}
                    break;
                }
                let mut chunk = vec![0u8; size + 2];
                socket.read_exact(&mut chunk).await.ok()?;
            }
        } else {
            let length: usize = header("Content-Length")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0u8; length];
            socket.read_exact(&mut body).await.ok()?;
        }
        let line = head[0].clone();
        lines.lock().unwrap().push(line.clone());
        hook(&line);

        let route = routes
            .iter()
            .find(|r| line.contains(&r.contains))
            .cloned()
            .unwrap_or_else(|| panic!("맞는 경로가 없다: {line}"));
        let extra: String = route
            .headers
            .iter()
            .map(|(n, v)| format!("{n}: {v}\r\n"))
            .collect();
        let has_length = route
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Content-Length"));
        let length = if has_length {
            String::new()
        } else {
            format!("Content-Length: {}\r\n", route.body.len())
        };
        let response = format!("HTTP/1.1 {} Status\r\n{extra}{length}\r\n", route.status);
        socket.write_all(response.as_bytes()).await.ok()?;
        if !line.starts_with("HEAD ") {
            socket.write_all(route.body.as_bytes()).await.ok()?;
        }
        socket.flush().await.ok()?;
    }
}
