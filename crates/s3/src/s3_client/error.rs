//! S3 호출 오류. 원본이 출력하던 `AmazonS3Exception`의 `StatusCode`, `ErrorCode`, `Message`를 보존한다.

use std::fmt;

use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use awscli_rest_common::dotnet_http::status_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Error {
    /// 서버가 오류 응답을 돌려준 경우(.NET `AmazonS3Exception`과 하위 예외).
    Service {
        status: u16,
        code: String,
        message: Option<String>,
        request_id: Option<String>,
    },
    /// 연결 실패·시간 초과 등 응답을 받지 못한 경우.
    Network(String),
    /// 요청을 만들 수 없는 경우(필수 값 누락 등).
    Request(String),
    /// 호출 인자가 잘못된 경우(.NET `ArgumentException`).
    Argument(String),
}

impl S3Error {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Service { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Service { code, .. } => Some(code),
            _ => None,
        }
    }

    /// .NET에서 던지던 예외 형식 이름. AWSSDK.S3가 별도 형식으로 모델링한 오류 코드만 구분한다.
    pub fn dotnet_type(&self) -> &'static str {
        match self {
            Self::Service { code, .. } => match code.as_str() {
                "NoSuchKey" => "Amazon.S3.Model.NoSuchKeyException",
                "NoSuchBucket" => "Amazon.S3.Model.NoSuchBucketException",
                "NoSuchUpload" => "Amazon.S3.Model.NoSuchUploadException",
                "BucketAlreadyExists" => "Amazon.S3.Model.BucketAlreadyExistsException",
                "BucketAlreadyOwnedByYou" => "Amazon.S3.Model.BucketAlreadyOwnedByYouException",
                "InvalidObjectState" => "Amazon.S3.Model.InvalidObjectStateException",
                "ObjectAlreadyInActiveTierError" => {
                    "Amazon.S3.Model.ObjectAlreadyInActiveTierErrorException"
                }
                "ObjectNotInActiveTierError" => {
                    "Amazon.S3.Model.ObjectNotInActiveTierErrorException"
                }
                _ => "Amazon.S3.AmazonS3Exception",
            },
            Self::Network(_) => "System.Net.Http.HttpRequestException",
            Self::Request(_) => "Amazon.Runtime.AmazonClientException",
            Self::Argument(_) => "System.ArgumentException",
        }
    }
}

impl fmt::Display for S3Error {
    /// `AmazonS3Exception.Message`: 서버 메시지가 있으면 그대로, 없으면 SDK가 만드는 문장.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service {
                status,
                code,
                message: Some(message),
                ..
            } if !message.is_empty() => {
                let _ = (status, code);
                f.write_str(message)
            }
            Self::Service { status, code, .. } => write!(
                f,
                "Error making request with Error Code {code} and Http Status Code {}. \
                 No further error information was returned by the service.",
                status_name(*status)
            ),
            Self::Network(message) | Self::Request(message) | Self::Argument(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for S3Error {}

impl<E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static> From<SdkError<E>>
    for S3Error
{
    fn from(error: SdkError<E>) -> Self {
        match &error {
            SdkError::ServiceError(service) => {
                let status = service.raw().status().as_u16();
                let inner = service.err();
                Self::Service {
                    status,
                    // 본문 없는 오류 응답이면 .NET은 상태 코드 이름을 오류 코드로 쓴다.
                    code: inner
                        .code()
                        .map(str::to_string)
                        .unwrap_or_else(|| status_name(status)),
                    message: inner.message().map(str::to_string),
                    request_id: inner.meta().extra("aws_request_id").map(str::to_string),
                }
            }
            SdkError::ConstructionFailure(_) => {
                Self::Request(aws_sdk_s3::error::DisplayErrorContext(&error).to_string())
            }
            _ => Self::Network(aws_sdk_s3::error::DisplayErrorContext(&error).to_string()),
        }
    }
}
