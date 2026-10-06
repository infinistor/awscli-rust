//! SDK 요청 처리 단계에 끼워 넣는 인터셉터.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use aws_sdk_s3::config::interceptors::{
    BeforeDeserializationInterceptorContextRef, BeforeTransmitInterceptorContextMut,
};
use aws_sdk_s3::config::{ConfigBag, Intercept, RuntimeComponents};
use aws_sdk_s3::error::BoxError;

/// 원본 `S3Headers.HEADER_BACKEND` 등.
pub const HEADER_BACKEND: &str = "x-ifs-backend";
pub const HEADER_KSAN_BACKEND: &str = "x-ksan-backend";
pub const HEADER_DATA: &str = "NONE";

/// 관리자 모드 헤더를 서명 전에 넣는다(원본 `RegisterAdminHeaders`).
#[derive(Debug)]
pub struct AdminHeaders;

impl Intercept for AdminHeaders {
    fn name(&self) -> &'static str {
        "AdminHeaders"
    }

    fn modify_before_signing(
        &self,
        context: &mut BeforeTransmitInterceptorContextMut<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let headers = context.request_mut().headers_mut();
        headers.insert(HEADER_BACKEND, HEADER_DATA);
        headers.insert(HEADER_KSAN_BACKEND, HEADER_DATA);
        Ok(())
    }
}

/// Rust SDK가 붙이는 `x-id=<연산 이름>` 쿼리 매개변수를 서명 전에 지운다. .NET SDK는 보내지 않는다.
#[derive(Debug)]
pub struct StripOperationId;

impl Intercept for StripOperationId {
    fn name(&self) -> &'static str {
        "StripOperationId"
    }

    fn modify_before_signing(
        &self,
        context: &mut BeforeTransmitInterceptorContextMut<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let request = context.request_mut();
        if let Some(uri) = strip_x_id(request.uri()) {
            request.set_uri(uri)?;
        }
        Ok(())
    }
}

/// 쿼리에서 `x-id=...`를 지운 주소. 없으면 `None`.
fn strip_x_id(uri: &str) -> Option<String> {
    let (base, query) = uri.split_once('?')?;
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.starts_with("x-id="))
        .collect();
    if kept.len() == query.split('&').count() {
        return None;
    }
    Some(if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept.join("&"))
    })
}

/// 마지막으로 받은 응답의 HTTP 상태 코드를 기록한다(재시도하면 마지막 시도 값).
#[derive(Debug, Default)]
pub struct StatusCapture {
    status: Arc<AtomicU16>,
}

/// `StatusCapture`가 기록한 값을 읽는 쪽.
#[derive(Debug, Clone)]
pub struct StatusSlot(Arc<AtomicU16>);

impl StatusSlot {
    pub fn get(&self) -> u16 {
        self.0.load(Ordering::Relaxed)
    }
}

impl StatusCapture {
    pub fn slot(&self) -> StatusSlot {
        StatusSlot(self.status.clone())
    }
}

impl Intercept for StatusCapture {
    fn name(&self) -> &'static str {
        "StatusCapture"
    }

    fn read_after_transmit(
        &self,
        context: &BeforeDeserializationInterceptorContextRef<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        self.status
            .store(context.response().status().as_u16(), Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::strip_x_id;

    #[test]
    fn strips_operation_id() {
        assert_eq!(
            strip_x_id("http://h/b/k?x-id=GetObject").as_deref(),
            Some("http://h/b/k")
        );
        assert_eq!(
            strip_x_id("http://h/?max-buckets=10&x-id=ListBuckets").as_deref(),
            Some("http://h/?max-buckets=10")
        );
        assert_eq!(strip_x_id("http://h/b?delete"), None);
    }
}
