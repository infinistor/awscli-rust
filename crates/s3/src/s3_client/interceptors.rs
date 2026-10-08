//! SDK 요청 처리 단계에 끼워 넣는 인터셉터.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use aws_sdk_s3::config::interceptors::{
    BeforeDeserializationInterceptorContextMut, BeforeDeserializationInterceptorContextRef,
    BeforeTransmitInterceptorContextMut,
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

/// 키가 `/`로 시작하면 .NET SDK는 요청 경로에서 그 `/` 하나를 뺀다(`/버킷//a` → `/버킷/a`, `//a` 키는 `/버킷//a`).
/// Rust SDK는 키를 그대로 붙이므로 서명 전에 같은 모양으로 바꾼다. `path_style`이면 첫 경로 조각이 버킷이다.
#[derive(Debug)]
pub struct TrimKeySlash {
    pub path_style: bool,
}

impl Intercept for TrimKeySlash {
    fn name(&self) -> &'static str {
        "TrimKeySlash"
    }

    fn modify_before_signing(
        &self,
        context: &mut BeforeTransmitInterceptorContextMut<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let request = context.request_mut();
        if let Some(uri) = trim_key_slash(request.uri(), self.path_style) {
            request.set_uri(uri)?;
        }
        Ok(())
    }
}

/// 키 앞의 `/` 하나를 뺀 주소. 바꿀 것이 없으면 `None`.
fn trim_key_slash(uri: &str, path_style: bool) -> Option<String> {
    // 스킴과 호스트 뒤의 경로 시작 위치
    let after_scheme = uri.find("://").map_or(0, |i| i + 3);
    let path_start = after_scheme + uri[after_scheme..].find('/')?;
    let path_end = uri[path_start..]
        .find(['?', '#'])
        .map_or(uri.len(), |i| path_start + i);
    let path = &uri[path_start..path_end];
    // 키가 시작하는 위치(경로 방식이면 `/버킷/` 다음, 아니면 `/` 다음)
    let key_start = if path_style {
        let bucket_end = path[1..].find('/')? + 1;
        bucket_end + 1
    } else {
        1
    };
    if !path[key_start..].starts_with('/') {
        return None;
    }
    let at = path_start + key_start;
    Some(format!("{}{}", &uri[..at], &uri[at + 1..]))
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

/// 성공 응답의 본문이 비어 있으면 `<root/>`로 바꾼다. .NET SDK는 본문이 빈 목록 응답을 빈 결과로 읽지만
/// Rust SDK는 XML 해석 오류로 본다.
#[derive(Debug)]
pub struct EmptyBodyAsRoot(pub &'static str);

impl Intercept for EmptyBodyAsRoot {
    fn name(&self) -> &'static str {
        "EmptyBodyAsRoot"
    }

    fn modify_before_deserialization(
        &self,
        context: &mut BeforeDeserializationInterceptorContextMut<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let response = context.response_mut();
        let empty = response.status().is_success()
            && (response.body().bytes().is_some_and(<[u8]>::is_empty)
                || response.headers().get("content-length") == Some("0"));
        if empty {
            *response.body_mut() = aws_sdk_s3::primitives::SdkBody::from(format!("<{}/>", self.0));
        }
        Ok(())
    }
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

/// 응답의 HTTP 날짜 헤더(`Last-Modified`, `Expires`, `Date`)에서 한 자리 일·시·분·초를 두 자리로 맞춘다.
///
/// KSAN은 `Thu, 8 Oct 2026 03:26:06 GMT`처럼 일을 한 자리로 보낸다. .NET SDK는 그대로 읽지만 Rust SDK의 HTTP 날짜
/// 해석은 IMF-fixdate(두 자리 일)만 받아 GetObject·HeadObject 응답 전체를 오류로 본다(매달 1~9일에만 드러난다).
#[derive(Debug)]
pub struct NormalizeHttpDates;

/// `Wdy, D Mon YYYY H:M:S GMT` → `Wdy, DD Mon YYYY HH:MM:SS GMT`. 형식이 다르면 그대로 둔다.
pub fn normalize_http_date(value: &str) -> Option<String> {
    let (weekday, rest) = value.split_once(", ")?;
    let parts: Vec<&str> = rest.split(' ').collect();
    let [day, month, year, time, zone] = parts.as_slice() else {
        return None;
    };
    let pad = |text: &str| -> Option<String> {
        (!text.is_empty() && text.len() <= 2 && text.bytes().all(|b| b.is_ascii_digit()))
            .then(|| format!("{text:0>2}"))
    };
    let clock: Vec<String> = time.split(':').map(pad).collect::<Option<_>>()?;
    if clock.len() != 3 {
        return None;
    }
    let normalized = format!(
        "{weekday}, {} {month} {year} {} {zone}",
        pad(day)?,
        clock.join(":")
    );
    (normalized != value).then_some(normalized)
}

impl Intercept for NormalizeHttpDates {
    fn name(&self) -> &'static str {
        "NormalizeHttpDates"
    }

    fn modify_before_deserialization(
        &self,
        context: &mut BeforeDeserializationInterceptorContextMut<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let headers = context.response_mut().headers_mut();
        for name in ["last-modified", "expires", "date"] {
            if let Some(fixed) = headers.get(name).and_then(normalize_http_date) {
                headers.insert(name, fixed);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_http_date, strip_x_id, trim_key_slash};

    #[test]
    fn pads_single_digit_http_dates() {
        assert_eq!(
            normalize_http_date("Thu, 8 Oct 2026 03:26:06 GMT").as_deref(),
            Some("Thu, 08 Oct 2026 03:26:06 GMT")
        );
        assert_eq!(
            normalize_http_date("Thu, 8 Oct 2026 3:6:6 GMT").as_deref(),
            Some("Thu, 08 Oct 2026 03:06:06 GMT")
        );
        assert_eq!(normalize_http_date("Thu, 08 Oct 2026 03:26:06 GMT"), None);
        assert_eq!(normalize_http_date("garbage"), None);
    }

    #[test]
    fn trims_one_leading_key_slash() {
        assert_eq!(
            trim_key_slash("http://h:1/b//a.txt?tagging", true).as_deref(),
            Some("http://h:1/b/a.txt?tagging")
        );
        assert_eq!(
            trim_key_slash("http://h/b///a.txt", true).as_deref(),
            Some("http://h/b//a.txt")
        );
        assert_eq!(trim_key_slash("http://h/b/dir//a.txt", true), None);
        assert_eq!(trim_key_slash("http://h/b/", true), None);
        assert_eq!(trim_key_slash("http://h/b", true), None);
        assert_eq!(trim_key_slash("http://h/", true), None);
        assert_eq!(
            trim_key_slash("https://b.s3.amazonaws.com//a", false).as_deref(),
            Some("https://b.s3.amazonaws.com/a")
        );
        assert_eq!(trim_key_slash("https://b.s3.amazonaws.com/a", false), None);
    }

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
