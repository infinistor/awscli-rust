//! TESTCore `Mover/MoverClient.cs` 이식: Mover 서비스의 작업 시작·상태 조회.
//!
//! 원본과 같게 맞춘 동작:
//!
//! - 시작은 `POST {URL}/api/Start`(본문은 `RequestMoverStart.ToString()`), 상태는 `GET {URL}/api/Status/{userId}`.
//! - 시작 응답이 JSON `null`이면 `-1`을 돌려준다. 상태 응답이 JSON `null`이면 `None`.
//! - 상태 응답의 `Items`가 `null`이거나 목록에 `null` 요소가 있으면(찾기 전에 만나면)
//!   `NullReferenceException`이다.
//! - 응답 코드가 `200 OK`가 아니면 `HttpRequestException`(`CurlClient` 참고).

pub mod data;
pub mod request;
pub mod response;

pub use data::{MoverStatus, SourceConfig, TargetConfig};
pub use request::RequestMoverStart;
pub use response::{ResponseMover, ResponseMoverStart, ResponseMoverStatus};

use crate::curl::{CurlClient, CurlError};

/// Mover 호출이 실패할 때의 오류.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoverError {
    /// 상태 응답의 `Items`나 요소가 `null`이라 발생하는 `NullReferenceException`
    #[error("Object reference not set to an instance of an object.")]
    NullReference,
    /// `CurlClient` 단계의 오류
    #[error(transparent)]
    Curl(#[from] CurlError),
}

impl MoverError {
    /// 원본에서 던지던 예외의 전체 형식 이름.
    pub fn dotnet_type(&self) -> &'static str {
        match self {
            Self::NullReference => "System.NullReferenceException",
            Self::Curl(error) => error.dotnet_type(),
        }
    }
}

/// 원본 `MoverClient`.
pub struct MoverClient {
    pub url: String,
    curl: CurlClient,
}

impl MoverClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            curl: CurlClient::new(),
        }
    }

    /// Mover 작업을 시작하고 생성된 Job Id를 돌려준다(응답이 `null`이면 -1).
    pub async fn mover_start(&self, request: &RequestMoverStart) -> Result<i32, MoverError> {
        let response = self
            .curl
            .post::<ResponseMoverStart>(
                &format!("{}/api/Start", self.url),
                &request.to_json_string(),
            )
            .await?;
        Ok(response.map_or(-1, |r| r.job_id))
    }

    /// 사용자의 Mover 작업 목록에서 지정한 Job Id의 상태를 조회한다(없으면 `None`).
    pub async fn mover_status(
        &self,
        user_id: &str,
        job_id: i32,
    ) -> Result<Option<MoverStatus>, MoverError> {
        let response = self
            .curl
            .get::<ResponseMoverStatus>(&format!("{}/api/Status/{user_id}", self.url))
            .await?;
        let Some(response) = response else {
            return Ok(None);
        };
        let items = response.items.ok_or(MoverError::NullReference)?;
        for item in items {
            let item = item.ok_or(MoverError::NullReference)?;
            if item.job_id == job_id {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }
}
