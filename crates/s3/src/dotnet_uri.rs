//! TESTCore가 `new Uri(...)`로 만든 주소에서 쓰는 값(`Host`, `Port`, `IsDefaultPort`, `AbsolutePath`)을
//! .NET `System.Uri`와 같게 계산한다. 서명(`CanonicalResourcePath`)과 실제 요청 경로가 이 값에 의존한다.
//!
//! `http(s)://호스트:포트/경로?쿼리` 형태(KsanClient가 만드는 주소)만 지원한다. 경로 처리 규칙은
//! `tools/dotnet-oracle`의 `uri` 명령으로 .NET 10 동작을 확인해 맞췄다.
//!
//! 1. `\`는 `/`로 바꾼다.
//! 2. `%XX`(16진수 두 자리)가 영숫자나 `-._~`이면 그 문자로 풀고, 아니면 적힌 그대로 둔다(대소문자 유지).
//! 3. 뒤에 16진수 두 자리가 오지 않는 `%`는 `%25`로 바꾼다. 이때 바로 뒤 두 글자 안에서 시작하는
//!    `%XX`는 풀지 않고 그대로 둔다(.NET 구현의 특성).
//! 4. 제어 문자, 공백, `"<>^`{|}`, 비 ASCII 문자는 UTF-8 바이트별 `%XX`(대문자)로 바꾼다.
//! 5. 마지막으로 RFC 3986 dot-segment(`.`, `..`)를 제거한다.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("지원하지 않는 URI 형식입니다: {0}")]
pub struct UriError(pub String);

/// .NET `System.Uri`에서 필요한 부분만 옮긴 값.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotnetUri {
    scheme: String,
    host: String,
    port: u16,
    absolute_path: String,
    query: String,
}

impl DotnetUri {
    pub fn parse(url: &str) -> Result<Self, UriError> {
        let err = || UriError(url.to_string());
        let (scheme, rest) = url.split_once("://").ok_or_else(err)?;
        let scheme = scheme.to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => return Err(err()),
        };
        // 권한(authority)은 첫 '/', '?', '#', '\'에서 끝난다.
        let authority_end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
        let (authority, rest) = rest.split_at(authority_end);
        let (host, port) = split_host_port(authority).ok_or_else(err)?;
        let port = port.unwrap_or(default_port);

        let (path_and_query, _fragment) = rest.split_once('#').unwrap_or((rest, ""));
        let (path, query) = path_and_query
            .split_once('?')
            .map(|(p, q)| (p, format!("?{q}")))
            .unwrap_or((path_and_query, String::new()));

        Ok(Self {
            scheme,
            host,
            port,
            absolute_path: normalize_path(path),
            query,
        })
    }

    /// `Uri.Host`: 소문자. IPv6는 대괄호를 포함한다.
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn is_default_port(&self) -> bool {
        matches!(
            (self.scheme.as_str(), self.port),
            ("http", 80) | ("https", 443)
        )
    }

    /// `Uri.AbsolutePath`: 이스케이프와 dot-segment 처리를 마친 경로. 항상 `/`로 시작한다.
    pub fn absolute_path(&self) -> &str {
        &self.absolute_path
    }

    /// 원본 쿼리 문자열(`?` 포함, 없으면 빈 문자열). 이스케이프는 아직 .NET과 맞추지 않았다.
    pub fn raw_query(&self) -> &str {
        &self.query
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }
}

impl fmt::Display for DotnetUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.host)?;
        if !self.is_default_port() {
            write!(f, ":{}", self.port)?;
        }
        write!(f, "{}{}", self.absolute_path, self.query)
    }
}

fn split_host_port(authority: &str) -> Option<(String, Option<u16>)> {
    // 사용자 정보는 지원하지 않는다.
    if authority.contains('@') {
        return None;
    }
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let end = rest.find(']')?;
        let host = format!("[{}]", &rest[..end]);
        match &rest[end + 1..] {
            "" => (host, None),
            port => (host, Some(port.strip_prefix(':')?)),
        }
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), Some(port)),
            None => (authority.to_string(), None),
        }
    };
    if host.is_empty() || host == "[]" {
        return None;
    }
    let port = match port {
        None | Some("") => None,
        Some(port) => Some(port.parse().ok()?),
    };
    Some((host.to_ascii_lowercase(), port))
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn needs_escape(c: char) -> bool {
    !c.is_ascii()
        || c.is_ascii_control()
        || matches!(c, ' ' | '"' | '<' | '>' | '^' | '`' | '{' | '|' | '}')
}

fn hex_pair(bytes: &[u8], at: usize) -> Option<u8> {
    let hi = (*bytes.get(at)? as char).to_digit(16)?;
    let lo = (*bytes.get(at + 1)? as char).to_digit(16)?;
    Some((hi * 16 + lo) as u8)
}

/// 위 규칙 1~5를 적용한다.
pub fn normalize_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len() + 8);
    // 이 위치 전까지 시작하는 `%XX`는 풀지 않는다(잘못된 `%` 바로 뒤 두 글자).
    let mut no_decode_until = 0;
    let mut i = 0;
    while i < path.len() {
        let c = path[i..].chars().next().expect("문자 경계");
        match c {
            '%' => match hex_pair(bytes, i + 1) {
                Some(b) if is_unreserved(b) && i >= no_decode_until => {
                    out.push(b as char);
                    i += 3;
                }
                Some(_) => {
                    out.push_str(&path[i..i + 3]);
                    i += 3;
                }
                None => {
                    out.push_str("%25");
                    no_decode_until = i + 3;
                    i += 1;
                }
            },
            '\\' => {
                out.push('/');
                i += 1;
            }
            c if needs_escape(c) => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
                i += c.len_utf8();
            }
            c => {
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    if !out.starts_with('/') {
        out.insert(0, '/');
    }
    remove_dot_segments(&out)
}

/// RFC 3986 5.2.4.
fn remove_dot_segments(path: &str) -> String {
    let mut input = path;
    let mut output = String::with_capacity(path.len());
    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix("../") {
            input = rest;
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest;
        } else if input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        } else if input.starts_with("/../") || input == "/.." {
            input = if input == "/.." { "/" } else { &input[3..] };
            match output.rfind('/') {
                Some(index) => output.truncate(index),
                None => output.clear(),
            }
        } else if input == "." || input == ".." {
            input = "";
        } else {
            let start = usize::from(input.starts_with('/'));
            let end = input[start..]
                .find('/')
                .map(|index| index + start)
                .unwrap_or(input.len());
            output.push_str(&input[..end]);
            input = &input[end..];
        }
    }
    if output.is_empty() {
        output.push('/');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_port() {
        let uri = DotnetUri::parse("http://LocalHost:80/b/k?q").unwrap();
        assert_eq!(uri.host(), "localhost");
        assert!(uri.is_default_port());
        let uri = DotnetUri::parse("http://[::1]:9000/b/k?q").unwrap();
        assert_eq!((uri.host(), uri.port()), ("[::1]", 9000));
        assert!(!uri.is_default_port());
        let uri = DotnetUri::parse("https://h:443/b/?tag-index").unwrap();
        assert!(uri.is_default_port());
        assert_eq!(uri.absolute_path(), "/b/");
        assert_eq!(uri.raw_query(), "?tag-index");
    }

    #[test]
    fn dot_segments() {
        assert_eq!(remove_dot_segments("/b/./x/../y"), "/b/y");
        assert_eq!(remove_dot_segments("/b/.."), "/");
        assert_eq!(remove_dot_segments("/b/a/."), "/b/a/");
        assert_eq!(remove_dot_segments("/b/a//b"), "/b/a//b");
    }
}
