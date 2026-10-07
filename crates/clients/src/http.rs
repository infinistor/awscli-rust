//! `KHttpClient`·`CurlClient`가 함께 쓰는 HTTP 전송. .NET `HttpClient`처럼 기본 헤더를 붙이지 않으려고
//! hyper를 직접 쓴다. 보내는 헤더는 `Host`와 호출자가 지정한 것뿐이다.

use std::sync::Arc;
use std::time::Duration;

use awscli_rest_s3::DotnetUri;
use bytes::Bytes;
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

/// `HttpClient.Timeout` 기본값(100초)
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(100);

/// 응답 상태 코드와 본문 문자열.
pub(crate) struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// 요청을 보내지 못했을 때의 원인.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransportError {
    /// 주소가 절대 URI가 아니거나 지원하지 않는 형식이다. 값은 .NET 예외의 (형식, 메시지)
    InvalidUri(&'static str, String),
    /// `HttpClient.Timeout` 초과
    Timeout,
    /// 연결·전송 실패
    Connect(String),
}

/// 인증서 검증 방식에 따라 TLS 설정을 만든다.
pub(crate) struct Transport {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
}

impl Transport {
    /// `connector`는 호출자가 TLS 설정을 끝낸 HTTPS 커넥터다.
    pub(crate) fn new(connector: HttpsConnector<HttpConnector>) -> Self {
        Self {
            // .NET은 Host, Content-Type처럼 단어 첫 글자를 대문자로 보낸다.
            client: Client::builder(TokioExecutor::new())
                .http1_title_case_headers(true)
                .build(connector),
        }
    }

    /// 운영체제 신뢰 저장소로 서버 인증서를 검증한다(`new HttpClient()` 기본 동작).
    pub(crate) fn verifying() -> Self {
        // 워크스페이스에는 `aws-lc-rs`와 `ring`이 함께 켜져 있어 기본 공급자를 고를 수 없다. `ring`을 지정한다.
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let builder =
            match HttpsConnectorBuilder::new().with_provider_and_native_roots(provider.clone()) {
                Ok(builder) => builder,
                // 신뢰 저장소를 읽지 못하면 HTTPS 요청은 인증서 검증에서 실패한다.
                Err(_) => HttpsConnectorBuilder::new().with_tls_config(
                    rustls::ClientConfig::builder_with_provider(provider)
                        .with_safe_default_protocol_versions()
                        .expect("기본 프로토콜 버전")
                        .with_root_certificates(rustls::RootCertStore::empty())
                        .with_no_client_auth(),
                ),
            };
        Self::new(builder.https_or_http().enable_http1().build())
    }
    /// 요청을 보내고 본문까지 받는다. 본문이 있으면(`Some`) `Content-Type`도 함께 보낸다.
    pub(crate) async fn send(
        &self,
        method: Method,
        url: &str,
        headers: &[(&str, String)],
        body: Option<(&str, Vec<u8>)>,
    ) -> Result<HttpResponse, TransportError> {
        let uri = DotnetUri::parse(url).map_err(|_| invalid_uri(url))?;
        let target = format!(
            "{}://{}:{}{}",
            uri.scheme(),
            uri.host(),
            uri.port(),
            uri.path_and_query()
        );
        // .NET은 기본 포트가 아닐 때만 `Host`에 포트를 붙인다.
        let host = if uri.is_default_port() {
            uri.host().to_string()
        } else {
            format!("{}:{}", uri.host(), uri.port())
        };
        let mut request = Request::builder()
            .method(method)
            .uri(target)
            .header("Host", host);
        for (name, value) in headers {
            request = request.header(*name, value.as_str());
        }
        let content = match body {
            Some((content_type, bytes)) => {
                request = request.header("Content-Type", content_type);
                // hyper는 빈 본문에 `Content-Length: 0`을 생략하지만 .NET은 보낸다.
                if bytes.is_empty() {
                    request = request.header("Content-Length", "0");
                }
                Full::new(Bytes::from(bytes))
            }
            None => Full::new(Bytes::new()),
        };
        let request = request
            .body(content)
            .map_err(|e| TransportError::Connect(e.to_string()))?;

        let work = async {
            let response = self
                .client
                .request(request)
                .await
                .map_err(|e| TransportError::Connect(error_chain(&e)))?;
            let status = response.status().as_u16();
            let bytes = response
                .into_body()
                .collect()
                .await
                .map_err(|e| TransportError::Connect(error_chain(&e)))?
                .to_bytes();
            Ok(HttpResponse {
                status,
                body: decode_body(&bytes),
            })
        };
        match tokio::time::timeout(HTTP_TIMEOUT, work).await {
            Ok(result) => result,
            Err(_) => Err(TransportError::Timeout),
        }
    }
}

/// 절대 URI가 아니면 `InvalidOperationException`, 그 밖에는 `UriFormatException`에 가깝게 구분한다.
fn invalid_uri(url: &str) -> TransportError {
    if url.contains("://") {
        TransportError::InvalidUri(
            "System.UriFormatException",
            "Invalid URI: The format of the URI could not be determined.".into(),
        )
    } else {
        TransportError::InvalidUri(
            "System.InvalidOperationException",
            "An invalid request URI was provided. Either the request URI must be an absolute URI or BaseAddress must be set.".into(),
        )
    }
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

/// `ReadAsStringAsync`: UTF-8 BOM은 건너뛰고 UTF-8로 읽는다.
fn decode_body(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}
