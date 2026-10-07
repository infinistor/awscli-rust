//! TESTCore `Portal/PortalException.cs` 이식과 PortalManager 호출 실패 오류.

use super::response::PortalResponse;
use crate::khttp::KHttpError;

/// PortalManager 호출이 실패할 때의 오류. 원본에서 던지던 .NET 예외 형식을 그대로 구분한다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortalError {
    /// `PortalVolumeException`: 볼륨 관련 Portal API 호출 실패
    #[error("{0}")]
    Volume(String),
    /// `PortalUserException`: 사용자 관련 Portal API 호출 실패
    #[error("{0}")]
    User(String),
    /// `PortalAccessIpsException`: AccessIps 관련 Portal API 호출 실패
    #[error("{0}")]
    AccessIps(String),
    /// `InvalidAccessIpsTestDataException`: AccessIps 테스트 데이터가 유효하지 않음
    #[error("{0}")]
    InvalidAccessIpsTestData(String),
    /// 응답 본문이 JSON `null`이라 `result.Result`에서 발생하는 `NullReferenceException`
    #[error("Object reference not set to an instance of an object.")]
    NullReference,
    /// KHttpClient 단계의 오류(HTTP 전송, JSON 읽기, `Authorization` 형식)
    #[error(transparent)]
    Client(#[from] KHttpError),
}

impl PortalError {
    /// 원본에서 던지던 예외의 전체 형식 이름.
    pub fn dotnet_type(&self) -> &'static str {
        match self {
            Self::Volume(_) => "TestCore.Portal.PortalVolumeException",
            Self::User(_) => "TestCore.Portal.PortalUserException",
            Self::AccessIps(_) => "TestCore.Portal.PortalAccessIpsException",
            Self::InvalidAccessIpsTestData(_) => {
                "TestCore.Portal.InvalidAccessIpsTestDataException"
            }
            Self::NullReference => "System.NullReferenceException",
            Self::Client(error) => error.dotnet_type(),
        }
    }

    pub(super) fn volume(response: &PortalResponse) -> Self {
        Self::Volume(error_message(response))
    }

    pub(super) fn user(response: &PortalResponse) -> Self {
        Self::User(error_message(response))
    }

    pub(super) fn access_ips(response: &PortalResponse) -> Self {
        Self::AccessIps(error_message(response))
    }
}

/// `ERROR({Code}) : {Message}`. 값이 없으면(`null`) 빈 문자열이다.
fn error_message(response: &PortalResponse) -> String {
    format!(
        "ERROR({}) : {}",
        response.code.as_deref().unwrap_or_default(),
        response.message.as_deref().unwrap_or_default()
    )
}
