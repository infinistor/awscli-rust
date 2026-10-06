//! TESTCore `Signers/AWS4SignerBase.cs`, `Signers/AWS4SignerForAuthorizationHeader.cs` 이식.
//!
//! TESTCore에서 실제로 쓰는 서명기는 KsanClient의 `Aws4SignerForAuthorizationHeader`뿐이다.
//! `AWS4SignerForChunkedUpload`, `AWS4SignerForPOST`, `AWS4SignerForQueryParameterAuth`는 호출하는 곳이 없어
//! 옮기지 않았다.
//!
//! 원본 동작을 그대로 따른 점:
//!
//! - 정규 경로는 `Uri.AbsolutePath`(이미 퍼센트 인코딩됨)를 다시 `UrlEncode(path, isPath: true)` 한다.
//!   따라서 `%`가 `%25`로 한 번 더 인코딩된다(예: 공백이 든 키 `a b` → `/b/a%2520b`).
//! - 쿼리는 호출자가 넘긴 문자열을 `&`로 나누고 `=` 기준 두 번째 조각만 값으로 쓴다(`a=b=c`의 값은 `b`).
//!   같은 이름이 두 번 나오면 오류다(원본 `ToDictionary` 예외).
//! - 헤더 이름은 소문자로 바꿔 정렬한다. 소문자로 바꾼 이름이 겹치면 오류다(원본 `SortedDictionary.Add` 예외).
//!   원본은 이름 목록을 `OrdinalIgnoreCase`, 헤더 블록을 현재 문화권 비교로 정렬하는데,
//!   여기서는 둘 다 서수 비교로 정렬한다. 영문 소문자·숫자·`-`로 된 헤더 이름에서는 결과가 같다.
//! - 헤더 값의 연속 공백은 한 칸으로 줄이고 앞뒤 공백을 지운다.
//! - `region`이나 `service`가 없으면(.NET의 `null`) 서명 키를 만들 때 오류가 난다
//!   (원본 `Encoding.UTF8.GetBytes(null)`의 `ArgumentNullException`).

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::dotnet_uri::DotnetUri;

/// 빈 본문의 SHA256.
pub const EMPTY_BODY_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

pub const SCHEME: &str = "AWS4";
pub const ALGORITHM: &str = "HMAC-SHA256";
pub const TERMINATOR: &str = "aws4_request";

pub const ISO8601_BASIC_FORMAT: &str = "%Y%m%dT%H%M%SZ";
pub const DATE_STRING_FORMAT: &str = "%Y%m%d";

pub const X_AMZ_ALGORITHM: &str = "X-Amz-Algorithm";
pub const X_AMZ_CREDENTIAL: &str = "X-Amz-Credential";
pub const X_AMZ_SIGNED_HEADERS: &str = "X-Amz-SignedHeaders";
pub const X_AMZ_DATE: &str = "X-Amz-Date";
pub const X_AMZ_SIGNATURE: &str = "X-Amz-Signature";
pub const X_AMZ_EXPIRES: &str = "X-Amz-Expires";
pub const X_AMZ_CONTENT_SHA256: &str = "X-Amz-Content-SHA256";
pub const X_AMZ_DECODED_CONTENT_LENGTH: &str = "X-Amz-Decoded-Content-Length";
pub const X_AMZ_META_UUID: &str = "X-Amz-Meta-UUID";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignError {
    #[error("An item with the same key has already been added. Key: {0}")]
    DuplicateHeader(String),
    #[error("An item with the same key has already been added. Key: {0}")]
    DuplicateQueryParameter(String),
    #[error("Value cannot be null. (Parameter '{0}')")]
    MissingValue(&'static str),
}

/// 헤더 목록. 원본 `Dictionary<string, string>`처럼 넣은 순서를 유지하고 이름은 대소문자를 구분한다.
pub type Headers = Vec<(String, String)>;

/// 원본 `Dictionary.Add`: 같은 이름(대소문자 구분)이 있으면 오류.
pub fn add_header(
    headers: &mut Headers,
    name: impl Into<String>,
    value: impl Into<String>,
) -> Result<(), SignError> {
    let name = name.into();
    if headers.iter().any(|(n, _)| *n == name) {
        return Err(SignError::DuplicateHeader(name));
    }
    headers.push((name, value.into()));
    Ok(())
}

/// 원본 `Aws4SignerForAuthorizationHeader`.
#[derive(Debug, Clone)]
pub struct AuthorizationHeaderSigner<'a> {
    pub endpoint: &'a DotnetUri,
    pub http_method: &'a str,
    pub service: Option<&'a str>,
    pub region: Option<&'a str>,
}

impl AuthorizationHeaderSigner<'_> {
    /// 원본 `ComputeSignature`. `headers`에 `X-Amz-Date`, `Host`를 추가하고 `Authorization` 값을 돌려준다.
    /// 원본은 현재 시각(`DateTime.UtcNow`)을 쓰므로 호출자가 `Utc::now()`를 넘긴다.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_signature(
        &self,
        headers: &mut Headers,
        query_parameters: &str,
        body_hash: &str,
        access_key: &str,
        secret_key: &str,
        debug: bool,
        now: DateTime<Utc>,
    ) -> Result<String, SignError> {
        let date_time_stamp = now.format(ISO8601_BASIC_FORMAT).to_string();
        add_header(headers, X_AMZ_DATE, date_time_stamp.as_str())?;

        let mut host_header = self.endpoint.host().to_string();
        if !self.endpoint.is_default_port() {
            host_header.push_str(&format!(":{}", self.endpoint.port()));
        }
        add_header(headers, "Host", host_header)?;

        let canonicalized_header_names = canonicalize_header_names(headers);
        let canonicalized_headers = canonicalize_headers(headers)?;
        let canonicalized_query = canonicalize_query(query_parameters)?;

        let canonical_request = canonicalize_request(
            self.endpoint,
            self.http_method,
            &canonicalized_query,
            &canonicalized_header_names,
            &canonicalized_headers,
            body_hash,
        );
        if debug {
            info!("\nCanonicalRequest:\n{canonical_request}");
        }

        let canonical_request_hash = Sha256::digest(canonical_request.as_bytes());
        let date_stamp = now.format(DATE_STRING_FORMAT).to_string();
        let region = self.region.unwrap_or("");
        let service = self.service.unwrap_or("");
        let scope = format!("{date_stamp}/{region}/{service}/{TERMINATOR}");
        let string_to_sign = format!(
            "{SCHEME}-{ALGORITHM}\n{date_time_stamp}\n{scope}\n{}",
            to_hex_string(&canonical_request_hash, true)
        );
        if debug {
            info!("\nStringToSign:\n{string_to_sign}");
        }

        let signing_key = derive_signing_key(secret_key, self.region, &date_stamp, self.service)?;
        let signature = to_hex_string(&hmac_sha256(&signing_key, string_to_sign.as_bytes()), true);
        if debug {
            info!("\nSignature:\n{signature}");
        }

        let authorization = format!(
            "{SCHEME}-{ALGORITHM} Credential={access_key}/{scope}, SignedHeaders={canonicalized_header_names}, Signature={signature}"
        );
        if debug {
            info!("\nAuthorization:\n{authorization}");
        }
        Ok(authorization)
    }
}

/// 원본 `CanonicalizeHeaderNames`.
pub fn canonicalize_header_names(headers: &Headers) -> String {
    let mut names: Vec<&str> = headers.iter().map(|(n, _)| n.as_str()).collect();
    // OrdinalIgnoreCase: 대문자로 바꿔 서수 비교. 안정 정렬이 아닌 원본과 달리 같은 키는 없다.
    names.sort_by_key(|n| n.to_uppercase());
    names
        .iter()
        .map(|n| n.to_lowercase())
        .collect::<Vec<_>>()
        .join(";")
}

/// 원본 `CanonicalizeHeaders`.
pub fn canonicalize_headers(headers: &Headers) -> Result<String, SignError> {
    if headers.is_empty() {
        return Ok(String::new());
    }
    let mut sorted: Vec<(String, &str)> = Vec::with_capacity(headers.len());
    for (name, value) in headers {
        let lower = name.to_lowercase();
        if sorted.iter().any(|(n, _)| *n == lower) {
            return Err(SignError::DuplicateHeader(lower));
        }
        sorted.push((lower, value));
    }
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(sorted
        .iter()
        .map(|(name, value)| format!("{name}:{}\n", compress_whitespace(value)))
        .collect())
}

/// 정규식 `\s+`를 한 칸 공백으로 바꾸고 앞뒤 공백을 지운다.
fn compress_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 원본 `ComputeSignature`의 쿼리 정규화.
pub fn canonicalize_query(query_parameters: &str) -> Result<String, SignError> {
    if query_parameters.is_empty() {
        return Ok(String::new());
    }
    let mut params: Vec<(&str, &str)> = Vec::new();
    for pair in query_parameters.split('&') {
        let mut parts = pair.split('=');
        let name = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if params.iter().any(|(n, _)| *n == name) {
            return Err(SignError::DuplicateQueryParameter(name.to_string()));
        }
        params.push((name, value));
    }
    params.sort_by(|a, b| a.0.cmp(b.0));
    Ok(params
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join("&"))
}

/// 원본 `CanonicalizeRequest`.
pub fn canonicalize_request(
    endpoint: &DotnetUri,
    http_method: &str,
    query_parameters: &str,
    canonicalized_header_names: &str,
    canonicalized_headers: &str,
    body_hash: &str,
) -> String {
    format!(
        "{http_method}\n{}\n{query_parameters}\n{canonicalized_headers}\n{canonicalized_header_names}\n{body_hash}",
        canonical_resource_path(endpoint)
    )
}

/// 원본 `CanonicalResourcePath`.
pub fn canonical_resource_path(endpoint: &DotnetUri) -> String {
    match endpoint.absolute_path() {
        "" => "/".to_string(),
        path => url_encode(path, true),
    }
}

/// 원본 `DeriveSigningKey`.
pub fn derive_signing_key(
    secret_key: &str,
    region: Option<&str>,
    date: &str,
    service: Option<&str>,
) -> Result<Vec<u8>, SignError> {
    let region = region.ok_or(SignError::MissingValue("s"))?;
    let service = service.ok_or(SignError::MissingValue("s"))?;
    let k_secret = format!("{SCHEME}{secret_key}");
    let hash_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes());
    let hash_region = hmac_sha256(&hash_date, region.as_bytes());
    let hash_service = hmac_sha256(&hash_region, service.as_bytes());
    Ok(hmac_sha256(&hash_service, TERMINATOR.as_bytes()))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC은 모든 키 길이를 받는다");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// 원본 `ToHexString`.
pub fn to_hex_string(data: &[u8], lowercase: bool) -> String {
    if lowercase {
        hex::encode(data)
    } else {
        hex::encode_upper(data)
    }
}

/// 원본 `UrlEncode`: RFC 3986 비예약 문자(경로면 `/`, `:` 포함)를 뺀 나머지를 `%XX`로 바꾼다.
/// 원본은 UTF-16 코드 단위 값을 그대로 16진수로 쓰므로(예: `한` → `%D55C`) 그 동작을 따른다.
pub fn url_encode(data: &str, is_path: bool) -> String {
    let mut encoded = String::with_capacity(data.len() * 2);
    for unit in data.encode_utf16() {
        let keep = char::from_u32(u32::from(unit)).is_some_and(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '.' | '~')
                || (is_path && matches!(c, '/' | ':'))
        });
        if keep {
            encoded.push(char::from_u32(u32::from(unit)).expect("ASCII"));
        } else {
            encoded.push_str(&format!("%{unit:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn url_encode_matches_original() {
        assert_eq!(url_encode("/b/a%20b", true), "/b/a%2520b");
        assert_eq!(url_encode("a/b:c d", false), "a%2Fb%3Ac%20d");
        assert_eq!(url_encode("한", false), "%D55C");
    }

    #[test]
    fn query_keeps_only_second_part() {
        assert_eq!(
            canonicalize_query("tag-index").unwrap(),
            "tag-index=".to_string()
        );
        assert_eq!(canonicalize_query("b=1&a=x=y").unwrap(), "a=x&b=1");
        assert!(canonicalize_query("a=1&a=2").is_err());
    }

    /// AWS SigV4 문서의 GET Object 예제(`examplebucket`, 2013-05-24)와 같은 서명이 나와야 한다.
    #[test]
    fn aws_documentation_example() {
        let uri = DotnetUri::parse("https://examplebucket.s3.amazonaws.com:443/test.txt").unwrap();
        let signer = AuthorizationHeaderSigner {
            endpoint: &uri,
            http_method: "GET",
            service: Some("s3"),
            region: Some("us-east-1"),
        };
        let mut headers: Headers = vec![
            ("Range".into(), "bytes=0-9".into()),
            ("x-amz-content-sha256".into(), EMPTY_BODY_SHA256.into()),
        ];
        let now = Utc.with_ymd_and_hms(2013, 5, 24, 0, 0, 0).unwrap();
        let authorization = signer
            .compute_signature(
                &mut headers,
                "",
                EMPTY_BODY_SHA256,
                "AKIAIOSFODNN7EXAMPLE",
                "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                false,
                now,
            )
            .unwrap();
        assert_eq!(
            authorization,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }
}
