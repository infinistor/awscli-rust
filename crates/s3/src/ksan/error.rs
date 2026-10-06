//! KsanClient 호출이 실패할 때의 오류. 원본에서 던지던 .NET 예외 형식을 그대로 구분한다.

use crate::signer::SignError;
use crate::xml_doc::XmlError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KsanError {
    /// `System.ArgumentException`. 입력 검증 실패와, 200/204가 아닌 응답(원본 `KsanException`이
    /// 응답 본문을 두 번 읽다가 `Stream was not readable.`로 실패하는 동작)이 여기에 해당한다.
    #[error("{0}")]
    Argument(String),
    /// `System.ArgumentNullException`. `DeleteBucketTagIndex`처럼 리전이 없는 서명.
    #[error("Value cannot be null. (Parameter '{0}')")]
    ArgumentNull(&'static str),
    /// `System.InvalidOperationException`. `XmlSerializer`가 응답을 읽지 못한 경우.
    #[error(transparent)]
    InvalidOperation(#[from] XmlError),
    /// `System.FormatException`. 접속 URL의 포트가 숫자가 아닌 경우.
    #[error("The input string '{0}' was not in a correct format.")]
    Format(String),
    /// `System.UriFormatException`.
    #[error("Invalid URI: {0}")]
    UriFormat(String),
    /// `System.Net.Http.HttpRequestException`. 연결 실패 등. 메시지는 .NET과 다르다.
    #[error("{0}")]
    Http(String),
}

/// 원본 `KsanException(HttpResponseMessage)`가 결국 던지는 메시지.
pub const STREAM_NOT_READABLE: &str = "Stream was not readable.";

impl KsanError {
    /// 원본에서 던지던 예외의 전체 형식 이름(`_log.Error(e)` 출력의 첫 부분).
    pub fn dotnet_type(&self) -> &'static str {
        match self {
            Self::Argument(_) => "System.ArgumentException",
            Self::ArgumentNull(_) => "System.ArgumentNullException",
            Self::InvalidOperation(_) => "System.InvalidOperationException",
            Self::Format(_) => "System.FormatException",
            Self::UriFormat(_) => "System.UriFormatException",
            Self::Http(_) => "System.Net.Http.HttpRequestException",
        }
    }
}

impl From<SignError> for KsanError {
    fn from(error: SignError) -> Self {
        match error {
            SignError::MissingValue(name) => Self::ArgumentNull(name),
            other => Self::Argument(other.to_string()),
        }
    }
}
