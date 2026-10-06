//! .NET `System.Net.HttpStatusCode` 열거형 이름. 원본은 상태 코드를 `{response.StatusCode}`처럼
//! 이름으로 출력하므로(예: `NotFound`) 같은 이름을 쓴다. 정의되지 않은 코드는 숫자로 출력된다.

/// `HttpStatusCode.ToString()`. 같은 값에 이름이 여러 개면 .NET이 고르는 이름을 쓴다.
pub fn status_name(code: u16) -> String {
    let name = match code {
        100 => "Continue",
        101 => "SwitchingProtocols",
        102 => "Processing",
        103 => "EarlyHints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "NonAuthoritativeInformation",
        204 => "NoContent",
        205 => "ResetContent",
        206 => "PartialContent",
        207 => "MultiStatus",
        208 => "AlreadyReported",
        226 => "IMUsed",
        300 => "Ambiguous",
        301 => "Moved",
        302 => "Redirect",
        303 => "RedirectMethod",
        304 => "NotModified",
        305 => "UseProxy",
        306 => "Unused",
        307 => "TemporaryRedirect",
        308 => "PermanentRedirect",
        400 => "BadRequest",
        401 => "Unauthorized",
        402 => "PaymentRequired",
        403 => "Forbidden",
        404 => "NotFound",
        405 => "MethodNotAllowed",
        406 => "NotAcceptable",
        407 => "ProxyAuthenticationRequired",
        408 => "RequestTimeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "LengthRequired",
        412 => "PreconditionFailed",
        413 => "RequestEntityTooLarge",
        414 => "RequestUriTooLong",
        415 => "UnsupportedMediaType",
        416 => "RequestedRangeNotSatisfiable",
        417 => "ExpectationFailed",
        421 => "MisdirectedRequest",
        422 => "UnprocessableEntity",
        423 => "Locked",
        424 => "FailedDependency",
        426 => "UpgradeRequired",
        428 => "PreconditionRequired",
        429 => "TooManyRequests",
        431 => "RequestHeaderFieldsTooLarge",
        451 => "UnavailableForLegalReasons",
        500 => "InternalServerError",
        501 => "NotImplemented",
        502 => "BadGateway",
        503 => "ServiceUnavailable",
        504 => "GatewayTimeout",
        505 => "HttpVersionNotSupported",
        506 => "VariantAlsoNegotiates",
        507 => "InsufficientStorage",
        508 => "LoopDetected",
        510 => "NotExtended",
        511 => "NetworkAuthenticationRequired",
        _ => return code.to_string(),
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(status_name(404), "NotFound");
        assert_eq!(status_name(500), "InternalServerError");
        assert_eq!(status_name(599), "599");
    }
}
