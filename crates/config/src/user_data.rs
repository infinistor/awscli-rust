//! TESTCore `Data/Config/UserData.cs` 이식.

use serde::Serialize;

use crate::util::{UtilError, try_parse_i32};

/// S3 접속 정보. JSON 속성 이름은 원본 그대로 `URL`, `RegionName`, `AccessKey`, `SecretKey`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct UserData {
    /// S3 URL
    #[serde(rename = "URL")]
    pub url: String,
    /// S3 Region
    #[serde(rename = "RegionName")]
    pub region_name: String,
    /// S3 AccessKey
    #[serde(rename = "AccessKey")]
    pub access_key: String,
    /// S3 SecretKey
    #[serde(rename = "SecretKey")]
    pub secret_key: String,
}

impl UserData {
    pub fn new(
        url: impl Into<String>,
        region_name: impl Into<String>,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            region_name: region_name.into(),
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }

    /// `IsEmpty`(JSON 제외): AccessKey, SecretKey, URL 중 하나라도 공백이면 true.
    pub fn is_empty(&self) -> bool {
        self.access_key.trim().is_empty()
            || self.secret_key.trim().is_empty()
            || self.url.trim().is_empty()
    }

    /// `IsRegion`(JSON 제외): RegionName이 비어 있지 않으면 true.
    pub fn is_region(&self) -> bool {
        !self.region_name.trim().is_empty()
    }

    pub fn set_url(&mut self, url: impl Into<String>) {
        self.url = url.into();
    }

    pub fn set_region(&mut self, region_name: impl Into<String>) {
        self.region_name = region_name.into();
    }

    pub fn set_access_key(&mut self, access_key: impl Into<String>) {
        self.access_key = access_key.into();
    }

    pub fn set_secret_key(&mut self, secret_key: impl Into<String>) {
        self.secret_key = secret_key.into();
    }

    /// URL에서 스킴을 제외한 호스트 부분(`//` 다음부터 마지막 `:` 앞까지)을 돌려준다. 추출할 수 없으면 빈 문자열.
    ///
    /// 원본은 UTF-16 인덱스를 쓰므로 같은 방식으로 계산한다. `//`가 없으면 `IndexOf`가 -1이라
    /// 시작 위치가 1이 되는 점도 그대로다.
    pub fn host(&self) -> String {
        let units: Vec<u16> = self.url.encode_utf16().collect();
        let start = find_slashes(&units).map_or(-1, |i| i as i64) + 2;
        let end = units
            .iter()
            .rposition(|&c| c == u16::from(b':'))
            .map_or(-1, |i| i as i64);
        if start < 0 || start > end {
            return String::new();
        }
        String::from_utf16_lossy(&units[start as usize..end as usize])
    }

    /// URL의 마지막 `:` 뒤를 포트 번호로 해석한다. `:`가 없으면 -1, 숫자가 아니면 오류(`int.Parse` 예외).
    pub fn port(&self) -> Result<i32, UtilError> {
        let Some(colon) = self.url.rfind(':') else {
            return Ok(-1);
        };
        let text = self.url[colon + 1..].replace('/', "");
        // int.Parse는 앞뒤 공백과 부호를 허용한다.
        try_parse_i32(&text).ok_or(UtilError::InvalidNumber(text))
    }
}

fn find_slashes(units: &[u16]) -> Option<usize> {
    units.windows(2).position(|w| w == [b'/' as u16; 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_port() {
        let u = UserData::new("http://localhost:8080/", "", "a", "b");
        assert_eq!(u.host(), "localhost");
        assert_eq!(u.port().unwrap(), 8080);
        let u = UserData::new("localhost", "", "a", "b");
        assert_eq!(u.host(), "");
        assert_eq!(u.port().unwrap(), -1);
        assert!(UserData::new("http://x:abc", "", "", "").port().is_err());
        assert!(UserData::default().is_empty());
    }
}
