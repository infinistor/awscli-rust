//! 분산 실행 E2E용 상태 있는 가짜 S3(TESTCore `tests/DistributedChecks` `MockS3` 이식).
//!
//! 경로 방식(`/버킷/키`)만 다룬다. 버킷 생성·확인(`HEAD`, `?acl`), 객체 PUT·GET·HEAD·DELETE, 목록(V1·V2), 일괄 삭제.
//! 객체 ETag는 본문 MD5다. `aws-chunked`·HTTP 청크 본문은 풀어서 저장한다.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, Response, StatusCode};
use md5::{Digest, Md5};

#[derive(Default)]
struct State {
    buckets: BTreeSet<String>,
    objects: BTreeMap<String, Vec<u8>>,
}

/// 실행 중인 가짜 S3.
pub struct MockS3 {
    pub url: String,
    state: Arc<Mutex<State>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for MockS3 {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl MockS3 {
    pub async fn start() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        let app = axum::Router::new().fallback(move |request: Request<Body>| {
            let state = shared.clone();
            async move { handle(state, request).await }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self { url, state, task }
    }

    /// 저장된 객체 키(`버킷/키`)와 크기.
    pub fn objects(&self) -> BTreeMap<String, usize> {
        let state = self.state.lock().unwrap();
        state
            .objects
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }
}

fn etag(body: &[u8]) -> String {
    format!("\"{}\"", hex::encode(Md5::digest(body)))
}

fn xml(status: StatusCode, text: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/xml")
        .header("x-amz-request-id", "test-request")
        .body(Body::from(text))
        .unwrap()
}

fn empty(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("x-amz-request-id", "test-request")
        .body(Body::empty())
        .unwrap()
}

fn not_found(code: &str) -> Response<Body> {
    xml(
        StatusCode::NOT_FOUND,
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>{code}</Code><Message>{code}</Message></Error>"
        ),
    )
}

/// `aws-chunked`·HTTP 청크를 푼다(크기 0 뒤 트레일러는 버린다).
fn dechunk(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = data;
    while let Some(end) = rest.windows(2).position(|w| w == b"\r\n") {
        let header = String::from_utf8_lossy(&rest[..end]);
        let Ok(size) = usize::from_str_radix(header.split(';').next().unwrap_or("").trim(), 16)
        else {
            return data.to_vec();
        };
        if size == 0 {
            break;
        }
        let start = end + 2;
        if rest.len() < start + size {
            return data.to_vec();
        }
        out.extend_from_slice(&rest[start..start + size]);
        rest = rest.get(start + size + 2..).unwrap_or_default();
    }
    out
}

fn payload(headers: &HeaderMap, body: &[u8]) -> Vec<u8> {
    let has = |name: &str, needle: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.to_ascii_lowercase().contains(needle))
    };
    if has("content-encoding", "aws-chunked") || has("x-amz-content-sha256", "streaming-") {
        dechunk(body)
    } else {
        body.to_vec()
    }
}

fn query(uri: &axum::http::Uri) -> BTreeMap<String, String> {
    uri.query()
        .unwrap_or_default()
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

async fn handle(state: Arc<Mutex<State>>, request: Request<Body>) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap_or_default();
    let path = decode(parts.uri.path().trim_start_matches('/'));
    let (bucket, key) = path.split_once('/').unwrap_or((&path, ""));
    let query = query(&parts.uri);
    let mut s = state.lock().unwrap();
    let ns = "http://s3.amazonaws.com/doc/2006-03-01/";
    if key.is_empty() {
        if parts.method == Method::PUT {
            s.buckets.insert(bucket.to_string());
            return empty(StatusCode::OK);
        }
        if !s.buckets.contains(bucket) {
            return not_found("NoSuchBucket");
        }
        if parts.method == Method::HEAD {
            return empty(StatusCode::OK);
        }
        if query.contains_key("acl") {
            return xml(
                StatusCode::OK,
                format!(
                    "<AccessControlPolicy xmlns=\"{ns}\"><Owner><ID>owner</ID></Owner><AccessControlList/></AccessControlPolicy>"
                ),
            );
        }
        if parts.method == Method::POST && query.contains_key("delete") {
            let text = String::from_utf8_lossy(&body);
            let mut deleted = String::new();
            for chunk in text.split("<Key>").skip(1) {
                let k = chunk.split("</Key>").next().unwrap_or_default();
                let k = k
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&");
                s.objects.remove(&format!("{bucket}/{k}"));
                deleted.push_str(&format!("<Deleted><Key>{}</Key></Deleted>", escape(&k)));
            }
            return xml(
                StatusCode::OK,
                format!("<DeleteResult xmlns=\"{ns}\">{deleted}</DeleteResult>"),
            );
        }
        if parts.method == Method::GET {
            let prefix = query.get("prefix").cloned().unwrap_or_default();
            let after = query
                .get("start-after")
                .or_else(|| query.get("marker"))
                .cloned()
                .unwrap_or_default();
            let keys: Vec<(String, usize)> = s
                .objects
                .iter()
                .filter_map(|(k, v)| {
                    k.strip_prefix(&format!("{bucket}/"))
                        .map(|k| (k.to_string(), v.len()))
                })
                .filter(|(k, _)| k.starts_with(&prefix) && k.as_str() > after.as_str())
                .collect();
            let contents: String = keys
                .iter()
                .map(|(k, size)| {
                    format!(
                        "<Contents><Key>{}</Key><LastModified>2026-01-01T00:00:00.000Z</LastModified><ETag>\"test\"</ETag><Size>{size}</Size><StorageClass>STANDARD</StorageClass></Contents>",
                        escape(k)
                    )
                })
                .collect();
            return xml(
                StatusCode::OK,
                format!(
                    "<ListBucketResult xmlns=\"{ns}\"><Name>{bucket}</Name><Prefix>{}</Prefix><KeyCount>{}</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated>{contents}</ListBucketResult>",
                    escape(&prefix),
                    keys.len()
                ),
            );
        }
        return empty(StatusCode::METHOD_NOT_ALLOWED);
    }
    let full = format!("{bucket}/{key}");
    match parts.method {
        Method::PUT => {
            let data = payload(&parts.headers, &body);
            let tag = etag(&data);
            s.buckets.insert(bucket.to_string());
            s.objects.insert(full, data);
            Response::builder()
                .status(StatusCode::OK)
                .header("ETag", tag)
                .header("x-amz-request-id", "test-request")
                .body(Body::empty())
                .unwrap()
        }
        Method::DELETE => {
            s.objects.remove(&full);
            empty(StatusCode::NO_CONTENT)
        }
        Method::GET | Method::HEAD => {
            let Some(data) = s.objects.get(&full).cloned() else {
                return not_found("NoSuchKey");
            };
            let builder = Response::builder()
                .status(StatusCode::OK)
                .header("ETag", etag(&data))
                .header("Content-Length", data.len())
                .header("Last-Modified", "Thu, 01 Jan 2026 00:00:00 GMT")
                .header("x-amz-request-id", "test-request");
            if parts.method == Method::HEAD {
                builder.body(Body::empty()).unwrap()
            } else {
                builder.body(Body::from(data)).unwrap()
            }
        }
        _ => empty(StatusCode::METHOD_NOT_ALLOWED),
    }
}
