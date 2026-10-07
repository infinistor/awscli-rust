//! TESTCore `Client/KHttpClient.cs` 이식: 응답 본문을 JSON으로 읽어 지정한 형식으로 돌려주는 `HttpClient` 래퍼.
//!
//! 원본과 같게 맞춘 동작:
//!
//! - `Authorization` 헤더에는 스킴 자리에 API 키만 넣는다(`AuthenticationHeaderValue(apiKey)`).
//!   공백이 아닌 값이어야 설정하며, HTTP 토큰이 아닌 문자(공백, 구분자, 비 ASCII)가 있으면 생성 시
//!   `FormatException`이다.
//! - 서버 인증서는 검증하지 않는다(`ServerCertificateCustomValidationCallback = true`).
//! - 상태 코드는 확인하지 않고 본문을 항상 JSON으로 읽는다. 기본 `System.Text.Json` 옵션이라 속성 이름은
//!   대소문자를 구분하고 열거형은 숫자다. 본문이 비어 있으면 `JsonException`이다(`null`/`default`로 보지 않는다).
//! - `Get`·`Post`는 `.Result`를 쓰므로 네트워크 오류가 `AggregateException`으로 감싸이고, `Delete`는
//!   `GetAwaiter().GetResult()`라 그대로 나온다.
//!
//! 이식하지 않은 것: `params JsonConverter[]`를 받는 `Get`·`Post`·`Put` 오버로드와 `ToQueryString`.
//! 호출하는 곳이 없고(PortalManager는 문자열 본문 `Post`와 `Get`·`Delete`만 쓴다), 호출하더라도
//! `Converters.ToString()`이 형식 이름 문자열(`System.Text.Json.Serialization.JsonConverter[]`)이라
//! 의미 있는 요청이 되지 않는다.
//!
//! 다르게 동작하는 점: 3xx 응답을 따라가지 않는다(.NET 기본값은 자동 리디렉션). 응답 본문의
//! `charset`은 무시하고 UTF-8로 읽는다. 네트워크 오류 메시지는 운영체제·런타임마다 다르다.

use std::sync::Arc;

use http::Method;
use hyper_rustls::HttpsConnectorBuilder;
use rustls::DigitallySignedStruct;
use rustls::SignatureScheme;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

use crate::http::{HTTP_TIMEOUT, HttpResponse, Transport, TransportError};
use crate::json::{FromJson, JsonError, ReadOptions, deserialize};

/// `POST`·`PUT`에서 `StringContent`가 붙이는 `Content-Type`
const STRING_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

/// KHttpClient 호출이 실패할 때의 오류. 원본에서 던지던 .NET 예외 형식을 그대로 구분한다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KHttpError {
    /// `System.FormatException`. `Authorization` 값이 HTTP 토큰이 아닌 경우.
    #[error("{0}")]
    Format(String),
    /// `System.Text.Json.JsonException`. 응답 본문을 읽지 못한 경우.
    #[error(transparent)]
    Json(#[from] JsonError),
    /// 요청을 보내지 못한 경우. `dotnet_type`은 원본이 던지던 예외 형식 이름이다.
    #[error("{message}")]
    Http {
        dotnet_type: &'static str,
        message: String,
    },
}

impl KHttpError {
    /// 원본에서 던지던 예외의 전체 형식 이름.
    pub fn dotnet_type(&self) -> &'static str {
        match self {
            Self::Format(_) => "System.FormatException",
            Self::Json(_) => "System.Text.Json.JsonException",
            Self::Http { dotnet_type, .. } => dotnet_type,
        }
    }

    /// `.Result`가 감싸는 `AggregateException`(`aggregate`) 또는 그대로 나오는 예외로 만든다.
    fn from_transport(error: TransportError, aggregate: bool) -> Self {
        let (inner_type, inner_message) = match error {
            TransportError::InvalidUri(dotnet_type, message) => (dotnet_type, message),
            TransportError::Timeout => (
                "System.Threading.Tasks.TaskCanceledException",
                format!(
                    "The request was canceled due to the configured HttpClient.Timeout of {} seconds elapsing.",
                    HTTP_TIMEOUT.as_secs()
                ),
            ),
            TransportError::Connect(message) => ("System.Net.Http.HttpRequestException", message),
        };
        if aggregate {
            Self::Http {
                dotnet_type: "System.AggregateException",
                message: format!("One or more errors occurred. ({inner_message})"),
            }
        } else {
            Self::Http {
                dotnet_type: inner_type,
                message: inner_message,
            }
        }
    }
}

/// 원본 `KHttpClient`.
pub struct KHttpClient {
    transport: Transport,
    authorization: Option<String>,
}

impl KHttpClient {
    /// `Authorization`이 비어 있지 않으면(공백만 있어도 비어 있는 것으로 본다) 모든 요청에 붙인다.
    pub fn new(authorization: Option<&str>) -> Result<Self, KHttpError> {
        let authorization = match authorization {
            Some(value) if !value.trim().is_empty() => {
                if !is_token(value) {
                    return Err(KHttpError::Format(format!(
                        "The format of value '{value}' is invalid."
                    )));
                }
                Some(value.to_string())
            }
            _ => None,
        };
        Ok(Self {
            transport: Transport::new(no_verify_connector()),
            authorization,
        })
    }

    /// 원본 `Get<T>(url)`
    pub async fn get<T: FromJson>(&self, url: &str) -> Result<Option<T>, KHttpError> {
        let response = self.send(Method::GET, url, None, true).await?;
        read_json(&response)
    }

    /// 원본 `Post<T>(url, string)`
    pub async fn post<T: FromJson>(&self, url: &str, body: &str) -> Result<Option<T>, KHttpError> {
        let response = self.send(Method::POST, url, Some(body), true).await?;
        read_json(&response)
    }

    /// 원본 `Delete<T>(url)`
    pub async fn delete<T: FromJson>(&self, url: &str) -> Result<Option<T>, KHttpError> {
        let response = self.send(Method::DELETE, url, None, false).await?;
        read_json(&response)
    }

    async fn send(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
        aggregate: bool,
    ) -> Result<HttpResponse, KHttpError> {
        let headers: Vec<(&str, String)> = self
            .authorization
            .iter()
            .map(|value| ("Authorization", value.clone()))
            .collect();
        let body = body.map(|text| (STRING_CONTENT_TYPE, text.as_bytes().to_vec()));
        self.transport
            .send(method, url, &headers, body)
            .await
            .map_err(|e| KHttpError::from_transport(e, aggregate))
    }
}

fn read_json<T: FromJson>(response: &HttpResponse) -> Result<Option<T>, KHttpError> {
    Ok(deserialize::<T>(&response.body, ReadOptions::default())?)
}

/// RFC 7230 토큰 문자만 허용한다(`HttpRuleParser.GetTokenLength`).
fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// 서버 인증서를 검증하지 않는 TLS 설정(원본의 `ServerCertificateCustomValidationCallback = true`).
/// 이 모듈 밖에서는 쓰지 않는다.
fn no_verify_connector()
-> hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("기본 프로토콜 버전")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify(provider)))
        .with_no_client_auth();
    HttpsConnectorBuilder::new()
        .with_tls_config(config)
        .https_or_http()
        .enable_http1()
        .build()
}

#[derive(Debug)]
struct NoVerify(Arc<CryptoProvider>);

impl ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_check() {
        assert!(is_token("tok.en-1_2~"));
        assert!(!is_token("Bearer abc"));
        assert!(!is_token("a/b"));
        assert!(!is_token("한글"));
        assert!(!is_token("k="));
    }

    #[test]
    fn blank_authorization_is_skipped() {
        assert!(KHttpClient::new(None).unwrap().authorization.is_none());
        assert!(
            KHttpClient::new(Some("  "))
                .unwrap()
                .authorization
                .is_none()
        );
        assert_eq!(
            KHttpClient::new(Some("a b")).err().unwrap().to_string(),
            "The format of value 'a b' is invalid."
        );
    }
}
