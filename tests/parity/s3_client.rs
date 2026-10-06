//! S3Client가 TESTCore `S3Client`(AWSSDK.S3)와 같은 요청을 보내고 같은 결과를 내는지 확인한다.
//! 기준 출력은 `tools/dotnet-oracle`의 `s3` 명령으로 만든다.
//!
//! SDK가 다르므로 다음은 비교하지 않는다: `User-Agent`, `amz-sdk-*`, `x-amz-user-agent`,
//! `Expect`, `X-Amz-Date`, 서명 값(청크·트레일러 서명 포함), `Host`의 포트, 쿼리 매개변수 순서,
//! .NET만 보내는 `x-amz-api-version`, 서명 대상 목록의 `content-length`(Rust SDK만 서명에 넣는다).
//!
//! 체크섬 없이 `UseChunkEncoding = true`로 올리는 경우는 형식이 다르다. .NET은 트레일러 없는 청크 서명
//! (`STREAMING-AWS4-HMAC-SHA256-PAYLOAD`), Rust SDK는 CRC32 트레일러가 붙은 청크 서명
//! (`STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER`)으로 보낸다. 이 사례는 `CHUNKED_CASES`에서 따로 확인한다.

#[path = "support/http_capture.rs"]
mod http_capture;

use std::collections::BTreeSet;
use std::path::Path;

use awscli_rest_s3::S3Client;
use awscli_rest_s3::s3_client::{PutBody, S3_MAX_KEYS};
use http_capture::{CannedResponse, CaptureServer, CapturedRequest};
use serde_json::Value;

const CASES: &[&str] = &[
    "list-buckets",
    "put-object-plain",
    "put-object-checksum-plain",
    "put-object-checksum-chunked",
    "put-object-admin",
    "get-object",
    "get-object-range",
    "head-object",
    "list-objects-v2",
    "delete-objects",
    "put-bucket-versioning",
    "head-bucket-exists",
    "head-bucket-missing",
    "error-500-retry",
    "error-404",
];

const CHUNKED_CASES: &[&str] = &["put-object-chunked", "upload-part-chunked"];

const IGNORED_HEADERS: &[&str] = &[
    "user-agent",
    "x-amz-user-agent",
    "amz-sdk-invocation-id",
    "amz-sdk-request",
    "expect",
    "x-amz-date",
    "x-amz-api-version",
];

/// 서명 대상 목록에서 빼고 비교하는 헤더(`x-amz-date`는 둘 다 서명하므로 남긴다).
fn ignored_in_signature(header: &str) -> bool {
    header != "x-amz-date" && (IGNORED_HEADERS.contains(&header) || header == "content-length")
}

/// 청크 서명과 트레일러 서명 값을 자리표시자로 바꾼다.
fn normalize_body(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(index) = rest.find("signature") {
        let (head, tail) = rest.split_at(index + "signature".len());
        out.push_str(head);
        // `chunk-signature=` 또는 `x-amz-trailer-signature:` 뒤 64자리 16진수
        let separator = &tail[..1];
        let value: String = tail[1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        if value.len() == 64 {
            out.push_str(separator);
            out.push_str("<SIG>");
            rest = &tail[1 + 64..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

#[derive(Debug, PartialEq, Eq)]
struct Normalized {
    method: String,
    path: String,
    query: BTreeSet<String>,
    headers: BTreeSet<(String, String)>,
    body: String,
}

fn normalize(request: &CapturedRequest) -> Normalized {
    let mut parts = request.line.split(' ');
    let method = parts.next().unwrap().to_string();
    let target = parts.next().unwrap();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter(|q| !q.is_empty())
        // `?acl`과 `?acl=`은 같은 뜻이다.
        .map(|q| q.trim_end_matches('=').to_string())
        .collect();
    let headers = request
        .headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.to_ascii_lowercase();
            if IGNORED_HEADERS.contains(&name.as_str()) {
                return None;
            }
            let value = match name.as_str() {
                "host" => value.split(':').next().unwrap().to_string(),
                "authorization" => {
                    let signed = value
                        .split("SignedHeaders=")
                        .nth(1)
                        .and_then(|s| s.split(',').next())
                        .unwrap_or("");
                    let signed: Vec<&str> = signed
                        .split(';')
                        .filter(|h| !ignored_in_signature(h))
                        .collect();
                    format!("SignedHeaders={}", signed.join(";"))
                }
                _ => value.clone(),
            };
            Some((name, value))
        })
        .collect();
    Normalized {
        method,
        path: path.to_string(),
        query,
        headers,
        body: normalize_body(&request.body),
    }
}

fn expected_requests(baseline: &Value) -> Vec<CapturedRequest> {
    baseline["requests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| CapturedRequest {
            line: r["line"].as_str().unwrap().to_string(),
            headers: r["headers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| (h[0].as_str().unwrap().into(), h[1].as_str().unwrap().into()))
                .collect(),
            body: r["body"].as_str().unwrap().to_string(),
        })
        .collect()
}

/// (상태 코드, 오류) — 오류는 (.NET 형식 이름, 메시지, 상태 코드, 오류 코드).
type Outcome = (
    Option<u16>,
    Option<(String, String, Option<u16>, Option<String>)>,
);

async fn run(spec: &Value, port: u16) -> Outcome {
    let text = |key: &str, default: &'static str| {
        spec.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| default.to_string())
    };
    let flag = |key: &str, default: bool| spec.get(key).and_then(Value::as_bool).unwrap_or(default);
    let retry = spec.get("retry").and_then(Value::as_i64).unwrap_or(3) as i32;
    let user = awscli_rest_config::UserData::new(
        format!("http://127.0.0.1:{port}"),
        "",
        "AKIAEXAMPLE",
        "secretExample",
    );
    let client = S3Client::from_user(&user, flag("admin", false), retry, flag("checksum", false));
    let bucket = text("bucket", "my-bucket");
    let key = text("key", "dir/key.txt");
    let body = text("body", "hello world");
    let chunked = flag("chunked", true);

    macro_rules! status {
        ($call:expr) => {
            match $call.await {
                Ok(response) => (Some(response.status), None),
                Err(e) => (
                    None,
                    Some((
                        e.dotnet_type().to_string(),
                        e.to_string(),
                        e.status(),
                        e.code().map(str::to_string),
                    )),
                ),
            }
        };
    }
    match text("op", "").as_str() {
        "list-buckets" => status!(client.list_buckets(None, 10000, None)),
        "put-bucket" => status!(client.put_bucket(&bucket, None, None, None)),
        "head-bucket-exists" => {
            let exists = client.does_s3_bucket_exist(&bucket).await;
            (Some(u16::from(exists)), None)
        }
        "put-object" => {
            status!(client.put_object(&bucket, &key, PutBody::Text(body), chunked, None))
        }
        "put-object-checksum" => status!(client.put_object_with_checksum(
            &bucket,
            &key,
            PutBody::Text(body),
            chunked,
            aws_sdk_s3::types::ChecksumAlgorithm::Crc32
        )),
        "get-object" => status!(client.get_object(&bucket, &key, None, None)),
        "get-object-range" => status!(client.get_object(&bucket, &key, None, Some((0, 4)))),
        "head-object" => status!(client.head_object(&bucket, &key, None, None)),
        "list-objects-v2" => status!(client.list_objects_v2(
            &bucket,
            Some("dir/"),
            None,
            S3_MAX_KEYS,
            Some("/"),
            None
        )),
        "list-objects" => {
            status!(client.list_objects(&bucket, Some("dir/"), None, S3_MAX_KEYS, None))
        }
        "delete-object" => status!(client.delete_object(&bucket, &key, None, None)),
        "delete-objects" => status!(client.delete_objects(
            &bucket,
            &[("a".into(), None), ("b".into(), Some("v1".into()))],
            None,
            Some(true)
        )),
        "upload-part" => status!(client.upload_part(
            &bucket,
            &key,
            "upload-1",
            1,
            PutBody::Bytes(body.into_bytes()),
            chunked
        )),
        "put-bucket-versioning" => status!(client.put_bucket_versioning(
            &bucket,
            Some(aws_sdk_s3::types::BucketVersioningStatus::Enabled)
        )),
        op => panic!("알 수 없는 op: {op}"),
    }
}

fn canned(spec: &Value) -> CannedResponse {
    CannedResponse {
        status: spec.get("status").and_then(Value::as_u64).unwrap_or(200) as u16,
        headers: spec
            .get("responseHeaders")
            .and_then(Value::as_object)
            .map(|h| {
                h.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        body: spec
            .get("responseBody")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

fn read(path: String) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity");
    serde_json::from_str(&std::fs::read_to_string(root.join(path)).unwrap()).unwrap()
}

/// .NET 결과를 같은 모양으로 바꾼다. `DoesS3BucketExist`는 bool을 0/1로.
fn expected_outcome(baseline: &Value) -> Outcome {
    let status = match &baseline["result"] {
        Value::Bool(b) => Some(u16::from(*b)),
        Value::Object(o) => o.get("status").and_then(Value::as_u64).map(|s| s as u16),
        _ => None,
    };
    let error = baseline["error"].as_object().map(|e| {
        (
            e["type"].as_str().unwrap().to_string(),
            e["message"].as_str().unwrap().to_string(),
            e.get("status").and_then(Value::as_u64).map(|s| s as u16),
            e.get("ErrorCode")
                .and_then(Value::as_str)
                .map(str::to_string),
        )
    });
    (status, error)
}

#[tokio::test]
async fn s3_client_matches_dotnet() {
    let mut failures = Vec::new();
    for name in CASES {
        let spec = read(format!("s3/{name}.json"));
        let baseline = read(format!("baseline/s3/{name}.json"));
        let server = CaptureServer::start(canned(&spec)).await;
        let outcome = run(&spec, server.port).await;

        let expected: Vec<Normalized> =
            expected_requests(&baseline).iter().map(normalize).collect();
        let actual: Vec<Normalized> = server.requests().iter().map(normalize).collect();
        if actual != expected {
            failures.push(format!(
                "{name} 요청\n  expected {expected:#?}\n  actual   {actual:#?}"
            ));
        }
        let expected_outcome = expected_outcome(&baseline);
        if outcome != expected_outcome {
            failures.push(format!(
                "{name} 결과\n  expected {expected_outcome:?}\n  actual   {outcome:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// 체크섬 없는 청크 업로드: .NET은 트레일러 없는 청크 서명, Rust는 CRC32 트레일러가 붙은 청크 서명.
/// 청크 본문(서명 자리 제외)과 디코딩 길이는 같고, Rust 쪽에만 트레일러가 더 붙는지 확인한다.
#[tokio::test]
async fn chunked_uploads_add_crc32_trailer() {
    for name in CHUNKED_CASES {
        let spec = read(format!("s3/{name}.json"));
        let baseline = read(format!("baseline/s3/{name}.json"));
        let expected = &expected_requests(&baseline)[0];
        assert_eq!(
            expected.header("X-Amz-Content-SHA256"),
            Some("STREAMING-AWS4-HMAC-SHA256-PAYLOAD"),
            "{name}"
        );

        let server = CaptureServer::start(canned(&spec)).await;
        let (status, error) = run(&spec, server.port).await;
        assert_eq!((status, error), (Some(200), None), "{name}");
        let requests = server.requests();
        assert_eq!(requests.len(), 1, "{name}");
        let actual = &requests[0];
        assert_eq!(
            actual.line.split('?').next(),
            expected.line.split('?').next(),
            "{name}"
        );
        assert_eq!(
            actual.header("Content-Encoding"),
            Some("aws-chunked"),
            "{name}"
        );
        assert_eq!(
            actual.header("X-Amz-Content-SHA256"),
            Some("STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER"),
            "{name}"
        );
        assert_eq!(
            actual.header("X-Amz-Trailer"),
            Some("x-amz-checksum-crc32"),
            "{name}"
        );
        assert_eq!(
            actual.header("X-Amz-Decoded-Content-Length"),
            expected.header("X-Amz-Decoded-Content-Length"),
            "{name}"
        );
        // .NET 본문의 마지막 빈 줄(\r\n) 앞에 트레일러 두 줄이 더 붙는다.
        let expected_body = normalize_body(&expected.body);
        let actual_body = normalize_body(&actual.body);
        let prefix = expected_body.strip_suffix("\r\n").unwrap();
        assert!(
            actual_body.starts_with(prefix),
            "{name}\n{expected_body:?}\n{actual_body:?}"
        );
        assert_eq!(
            &actual_body[prefix.len()..],
            "x-amz-checksum-crc32:DUoRhQ==\r\nx-amz-trailer-signature:<SIG>\r\n\r\n",
            "{name}"
        );
    }
}
