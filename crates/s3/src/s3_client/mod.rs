//! TESTCore `Client/S3Client.cs` 이식. AWSSDK.S3 대신 `aws-sdk-s3`를 감싼다.
//!
//! .NET 설정과 맞춘 점:
//!
//! - 타임아웃 3600초, 재시도는 `retryCount`회(시도 횟수 = `retryCount + 1`, 표준 재시도 모드).
//! - 사용자 URL이 있으면 그 주소를 경로 방식(path-style)으로 쓰고 서명 리전은 `us-east-1`.
//!   URL이 없으면 AWS `ap-northeast-2`.
//! - 응답 체크섬 검증은 `WhenRequired`. 요청 체크섬은 `calculate_request_checksum`이면 `WhenSupported`,
//!   아니면 `WhenRequired`.
//! - 관리자 모드면 모든 요청에 `x-ifs-backend: NONE`, `x-ksan-backend: NONE`을 서명 전에 넣는다
//!   (원본 `BeforeRequestEvent`와 같이 서명 대상에 포함된다).
//! - 응답의 HTTP 상태 코드(.NET `response.HttpStatusCode`)를 `S3Response::status`로 돌려준다.
//!
//! .NET과 다른 점(설계 결정, `docs/design/s3-client.md`):
//!
//! - `UseChunkEncoding = true`인 업로드는 해당 요청만 요청 체크섬을 `WhenSupported`로 바꾸고 본문을
//!   스트림으로 보낸다. SDK는 이를 CRC32 트레일러가 붙은 청크 서명
//!   (`STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER`)으로 보낸다. .NET은 체크섬이 없으면 트레일러 없는
//!   청크 서명(`STREAMING-AWS4-HMAC-SHA256-PAYLOAD`)을 쓰므로 트레일러 두 줄만 다르다.
//!   `false`면 `WhenRequired`로 본문 전체를 서명해 한 번에 보낸다(.NET과 같음).
//! - Rust SDK가 붙이는 `x-id` 쿼리 매개변수는 서명 전에 지운다.

pub mod error;
mod interceptors;

use std::time::Duration;

use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{
    BehaviorVersion, Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation,
};
use awscli_rest_config::UserData;

pub use error::S3Error;
use interceptors::{AdminHeaders, StripOperationId};

/// 원본 `S3_TIMEOUT`(초).
pub const S3_TIMEOUT: u64 = 3600;
/// 원본 `S3_MAX_KEYS`.
pub const S3_MAX_KEYS: i32 = 1000;
/// 사용자 지정 주소를 쓸 때의 서명 리전(.NET SDK 기본값).
const DEFAULT_SIGNING_REGION: &str = "us-east-1";
/// URL이 없을 때의 리전(원본 `RegionEndpoint.APNortheast2`).
const DEFAULT_AWS_REGION: &str = "ap-northeast-2";

/// 응답과 HTTP 상태 코드.
#[derive(Debug, Clone)]
pub struct S3Response<T> {
    pub status: u16,
    pub output: T,
}

/// 원본 `S3Client`.
#[derive(Debug, Clone)]
pub struct S3Client {
    client: aws_sdk_s3::Client,
    is_admin: bool,
}

/// 요청을 보내고 상태 코드와 함께 돌려준다. `$checksum`을 주면 이 요청만 요청 체크섬 설정을 바꾼다.
macro_rules! send {
    ($builder:expr) => {{
        let capture = $crate::s3_client::interceptors::StatusCapture::default();
        let slot = capture.slot();
        let output = $builder
            .customize()
            .interceptor(capture)
            .send()
            .await
            .map_err($crate::s3_client::S3Error::from)?;
        Ok($crate::s3_client::S3Response {
            status: slot.get(),
            output,
        })
    }};
    ($builder:expr, checksum = $checksum:expr) => {{
        let capture = $crate::s3_client::interceptors::StatusCapture::default();
        let slot = capture.slot();
        let output = $builder
            .customize()
            .interceptor(capture)
            .config_override(
                aws_sdk_s3::config::Builder::default().request_checksum_calculation($checksum),
            )
            .send()
            .await
            .map_err($crate::s3_client::S3Error::from)?;
        Ok($crate::s3_client::S3Response {
            status: slot.get(),
            output,
        })
    }};
}

mod bucket;
mod object;

pub use object::PutBody;

impl S3Client {
    /// 원본 `S3Client(UserData user, bool isAdmin = false, int retryCount = 3, bool calculateRequestChecksum = false)`.
    pub fn from_user(
        user: &UserData,
        is_admin: bool,
        retry_count: i32,
        calculate_request_checksum: bool,
    ) -> Self {
        let checksum = if calculate_request_checksum {
            RequestChecksumCalculation::WhenSupported
        } else {
            RequestChecksumCalculation::WhenRequired
        };
        Self::build(
            Some(user.url.as_str()).filter(|u| !u.is_empty()),
            &user.access_key,
            &user.secret_key,
            is_admin,
            retry_count,
            checksum,
        )
    }

    /// 원본 `S3Client(string url, string accessKey, string secretKey, bool isAdmin = false, int retryCount = 2)`.
    pub fn new(
        url: &str,
        access_key: &str,
        secret_key: &str,
        is_admin: bool,
        retry_count: i32,
    ) -> Self {
        Self::build(
            Some(url),
            access_key,
            secret_key,
            is_admin,
            retry_count,
            RequestChecksumCalculation::WhenRequired,
        )
    }

    fn build(
        url: Option<&str>,
        access_key: &str,
        secret_key: &str,
        is_admin: bool,
        retry_count: i32,
        checksum: RequestChecksumCalculation,
    ) -> Self {
        let max_attempts = u32::try_from(retry_count.max(0)).unwrap_or(0) + 1;
        let mut config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(Credentials::new(
                access_key, secret_key, None, None, "TestCore",
            ))
            .retry_config(RetryConfig::standard().with_max_attempts(max_attempts))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_attempt_timeout(Duration::from_secs(S3_TIMEOUT))
                    .build(),
            )
            .request_checksum_calculation(checksum)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .interceptor(StripOperationId);
        config = match url {
            Some(url) => config
                .endpoint_url(with_scheme(url))
                .region(Region::new(DEFAULT_SIGNING_REGION))
                .force_path_style(true),
            None => config.region(Region::new(DEFAULT_AWS_REGION)),
        };
        if is_admin {
            config = config.interceptor(AdminHeaders);
        }
        Self {
            client: aws_sdk_s3::Client::from_conf(config.build()),
            is_admin,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    /// 감싼 SDK 클라이언트. 원본 `S3Client.Client`처럼 직접 호출이 필요한 곳에서 쓴다.
    pub fn inner(&self) -> &aws_sdk_s3::Client {
        &self.client
    }
}

/// 원본 `UseHttp = true`: 스킴이 없는 주소는 `http://`로 접속한다.
fn with_scheme(url: &str) -> String {
    if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{url}")
    }
}

/// `UseChunkEncoding` 값을 요청 체크섬 설정으로 바꾼다(모듈 문서 참고).
pub(crate) fn chunk_checksum(use_chunk_encoding: bool) -> RequestChecksumCalculation {
    if use_chunk_encoding {
        RequestChecksumCalculation::WhenSupported
    } else {
        RequestChecksumCalculation::WhenRequired
    }
}
