//! 원본 `#region ETC Function`의 `GeneratePresignedURL`.
//!
//! .NET `GetPreSignedURL`과 맞춘 점:
//!
//! - 프로토콜은 항상 HTTP(`Protocol = Protocol.HTTP`).
//! - 유효 시간(`X-Amz-Expires`)은 `(Expires - 현재 시각)`의 초 단위(소수 버림, 현재 시각은 초로 내림).
//! - `ContentType`, `ServerSideEncryptionMethod`는 서명 대상 헤더(`SignedHeaders`)로 들어간다.
//! - 관리자 모드의 `x-ifs-backend` 헤더는 서명에 넣지 않는다(.NET은 `BeforeRequestEvent`를 거치지 않는다).
//! - 유효 시간이 7일(604800초)을 넘으면 .NET은 서명 V4 대신 서명 V2 쿼리(`AWSAccessKeyId`, `Expires`,
//!   `Signature`)를 만든다. 같은 방식으로 직접 계산한다.
//!
//! 다른 점: .NET은 이미 지난 시각도 음수 `X-Amz-Expires`로 URL을 만들지만, `aws-sdk-s3`는 음수·0 유효 시간을
//! 만들 수 없어 오류로 돌려준다. 또 `DateTime`의 `Kind`가 `Unspecified`면 .NET은 로컬 시간으로 보지만,
//! 여기서는 시각이 항상 UTC(`chrono::DateTime<Utc>`)다.

use std::time::Duration;

use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::ServerSideEncryption;
use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

use super::{S3Client, S3Error};

/// V4는 최대 7일까지만 만들 수 있다.
const MAX_V4_SECONDS: i64 = 7 * 24 * 60 * 60;

/// 원본 `Amazon.S3.HttpVerb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVerb {
    Get,
    Put,
    Delete,
    Head,
}

impl HttpVerb {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
        }
    }
}

/// 서명 V2 URL을 만드는 데 필요한 값.
#[derive(Debug, Clone)]
pub(crate) struct PresignContext {
    pub access_key: String,
    pub secret_key: String,
    /// 사용자 지정 주소(스킴 포함). 없으면 AWS 기본 주소.
    pub endpoint: Option<String>,
}

/// 서명 대상 헤더(`Content-Type`, `x-amz-server-side-encryption`)를 서명 전에 넣는다.
fn headers(
    content_type: Option<String>,
    sse: Option<String>,
) -> impl Fn(&mut aws_sdk_s3::config::http::HttpRequest) + Send + Sync + 'static {
    move |request| {
        if let Some(value) = &content_type {
            request.headers_mut().insert("content-type", value.clone());
        }
        if let Some(value) = &sse {
            request
                .headers_mut()
                .insert("x-amz-server-side-encryption", value.clone());
        }
    }
}

impl S3Client {
    /// 원본 `GeneratePresignedURL(bucketName, key, DateTime Expires, HttpVerb Verb, ServerSideEncryptionMethod SSE_S3_Method = null, string contentType = null)`.
    pub async fn generate_presigned_url(
        &self,
        bucket_name: &str,
        key: &str,
        expires: DateTime<Utc>,
        verb: HttpVerb,
        sse_s3_method: Option<ServerSideEncryption>,
        content_type: Option<&str>,
    ) -> Result<String, S3Error> {
        let seconds = expires.timestamp() - Utc::now().timestamp();
        let sse = sse_s3_method.map(|m| m.as_str().to_string());
        if seconds > MAX_V4_SECONDS {
            return Ok(self.presign.sign_v2(
                bucket_name,
                key,
                expires.timestamp(),
                verb,
                sse.as_deref(),
                content_type,
            ));
        }
        let seconds = u64::try_from(seconds)
            .ok()
            .filter(|s| *s > 0)
            .ok_or_else(|| {
                S3Error::Request(format!(
                    "유효 시간이 0초 이하입니다(만료 시각이 이미 지났습니다): {expires}"
                ))
            })?;
        let config = PresigningConfig::expires_in(Duration::from_secs(seconds))
            .map_err(|e| S3Error::Request(e.to_string()))?;
        let mutate = headers(content_type.map(str::to_string), sse);

        let client = &self.plain_client;
        let presigned = match verb {
            HttpVerb::Get => client
                .get_object()
                .bucket(bucket_name)
                .key(key)
                .customize()
                .mutate_request(mutate)
                .presigned(config)
                .await
                .map(|p| p.uri().to_string())
                .map_err(S3Error::from),
            HttpVerb::Put => client
                .put_object()
                .bucket(bucket_name)
                .key(key)
                .customize()
                .mutate_request(mutate)
                .presigned(config)
                .await
                .map(|p| p.uri().to_string())
                .map_err(S3Error::from),
            HttpVerb::Delete => client
                .delete_object()
                .bucket(bucket_name)
                .key(key)
                .customize()
                .mutate_request(mutate)
                .presigned(config)
                .await
                .map(|p| p.uri().to_string())
                .map_err(S3Error::from),
            HttpVerb::Head => client
                .head_object()
                .bucket(bucket_name)
                .key(key)
                .customize()
                .mutate_request(mutate)
                .presigned(config)
                .await
                .map(|p| p.uri().to_string())
                .map_err(S3Error::from),
        }?;
        Ok(force_http(&presigned))
    }
}

/// `Protocol.HTTP`: 스킴을 `http`로 바꾼다(서명 V4는 스킴을 서명하지 않는다).
fn force_http(uri: &str) -> String {
    match uri.strip_prefix("https://") {
        Some(rest) => format!("http://{rest}"),
        None => uri.to_string(),
    }
}

impl PresignContext {
    /// 서명 V2 쿼리 인증 URL. 서명 대상: `VERB\n\n{Content-Type}\n{만료 시각(에폭 초)}\n{x-amz 헤더}{/버킷/키}`.
    fn sign_v2(
        &self,
        bucket: &str,
        key: &str,
        expires_epoch: i64,
        verb: HttpVerb,
        sse: Option<&str>,
        content_type: Option<&str>,
    ) -> String {
        let path = encode_path(key);
        let amz_headers = sse
            .map(|v| format!("x-amz-server-side-encryption:{v}\n"))
            .unwrap_or_default();
        let string_to_sign = format!(
            "{}\n\n{}\n{expires_epoch}\n{amz_headers}/{bucket}/{path}",
            verb.as_str(),
            content_type.unwrap_or("")
        );
        let mut mac = Hmac::<Sha1>::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC은 모든 키 길이를 받는다");
        mac.update(string_to_sign.as_bytes());
        let signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        let base = match &self.endpoint {
            Some(endpoint) => {
                let endpoint = endpoint.trim_end_matches('/');
                let rest = endpoint.split_once("://").map_or(endpoint, |(_, r)| r);
                format!("http://{rest}/{bucket}")
            }
            None => format!("http://{bucket}.s3.ap-northeast-2.amazonaws.com"),
        };
        format!(
            "{base}/{path}?AWSAccessKeyId={}&Expires={expires_epoch}&Signature={}",
            self.access_key,
            encode(&signature)
        )
    }
}

/// 키를 경로로 인코딩한다(`/`는 유지).
fn encode_path(key: &str) -> String {
    key.split('/').map(encode).collect::<Vec<_>>().join("/")
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
