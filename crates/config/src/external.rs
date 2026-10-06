//! 외부 연동 설정 이식: `Data/AccessIps/AccessIpsConfig.cs`, `Mover/MoverConfig.cs`,
//! `Portal/PortalConfig.cs`, `Jenkins/JenkinsConfig.cs`.

use rand::Rng;
use serde::Serialize;

use crate::util::{UtilError, size_to_long};

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// `AccessIps` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AccessIpsConfig {
    /// 버킷 접두어
    pub bucket_prefix: String,
    /// 모든 테스트가 실패해야 하는지 여부
    pub all_failed: bool,
    /// 테스트 리스트 파일 경로
    pub test_list_path: String,
    /// Jenkins 테스트 이름
    pub jenkins_test_name: String,
    /// S3 URL 목록
    #[serde(rename = "S3URL")]
    pub s3_url: Vec<String>,
    /// 볼륨 이름
    pub volume_name: String,
    /// 메인 사용자 이름
    pub main_user_name: String,
    /// 서브 사용자 이름
    pub sub_user_name: String,
    /// 전체 공통 비밀번호
    pub password: String,
}

impl AccessIpsConfig {
    /// URL 목록을 직접 받는 생성자(원본의 `List<string>` 오버로드). 비밀번호 기본값은 적용하지 않는다.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bucket_prefix: &str,
        all_failed: bool,
        test_list_path: &str,
        jenkins_test_name: &str,
        s3_urls: Vec<String>,
        volume_name: &str,
        main_user_name: &str,
        sub_user_name: &str,
        password: &str,
    ) -> Self {
        Self {
            bucket_prefix: bucket_prefix.to_string(),
            all_failed,
            test_list_path: test_list_path.to_string(),
            jenkins_test_name: jenkins_test_name.to_string(),
            s3_url: s3_urls,
            volume_name: volume_name.to_string(),
            main_user_name: main_user_name.to_string(),
            sub_user_name: sub_user_name.to_string(),
            password: password.to_string(),
        }
    }

    /// 콤마로 구분된 URL 문자열을 받는 생성자(원본의 `string` 오버로드). 비밀번호가 비어 있으면 `qwe123`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_csv(
        bucket_prefix: &str,
        all_failed: bool,
        test_list_path: &str,
        jenkins_test_name: &str,
        s3_urls: &str,
        volume_name: &str,
        main_user_name: &str,
        sub_user_name: &str,
        password: &str,
    ) -> Self {
        let password = if password.is_empty() {
            "qwe123"
        } else {
            password
        };
        Self::new(
            bucket_prefix,
            all_failed,
            test_list_path,
            jenkins_test_name,
            split_urls(s3_urls),
            volume_name,
            main_user_name,
            sub_user_name,
            password,
        )
    }

    /// `IsEmpty`(JSON 제외). 여기서는 `IsNullOrEmpty`라 공백 문자열은 비어 있지 않은 것으로 본다.
    pub fn is_empty(&self) -> bool {
        self.bucket_prefix.is_empty()
            || self.test_list_path.is_empty()
            || self.jenkins_test_name.is_empty()
            || self.s3_url.is_empty()
            || self.volume_name.is_empty()
            || self.main_user_name.is_empty()
            || self.sub_user_name.is_empty()
    }
}

/// `Split(',', TrimEntries | RemoveEmptyEntries)`
fn split_urls(urls: &str) -> Vec<String> {
    urls.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// `Mover` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MoverConfig {
    #[serde(rename = "URL")]
    pub url: String,
    pub user_id: String,
    pub source_bucket: String,
    pub target_bucket: String,
    pub file_count: i32,
    pub max_file_size: i64,
}

impl MoverConfig {
    /// `max_file_size`는 크기 문자열이며 잘못되면 오류.
    pub fn new(
        url: &str,
        user_id: &str,
        source_bucket: &str,
        target_bucket: &str,
        file_count: i32,
        max_file_size: &str,
    ) -> Result<Self, UtilError> {
        Ok(Self {
            url: url.to_string(),
            user_id: user_id.to_string(),
            source_bucket: source_bucket.to_string(),
            target_bucket: target_bucket.to_string(),
            file_count,
            max_file_size: size_to_long(max_file_size)?,
        })
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        is_blank(&self.url)
            || is_blank(&self.user_id)
            || is_blank(&self.source_bucket)
            || is_blank(&self.target_bucket)
            || self.file_count < 1
            || self.max_file_size < 1
    }

    /// 최대 파일 크기 미만의 임의 크기(`Rand.NextInt64(MaxFileSize)`). 0이면 0, 음수면 오류.
    pub fn random_file_size(&self) -> Result<i64, UtilError> {
        match self.max_file_size {
            0 => Ok(0),
            n if n > 0 => Ok(rand::rng().random_range(0..n)),
            _ => Err(UtilError::OutOfRange("NextInt64")),
        }
    }
}

/// `Portal` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortalConfig {
    /// Portal URL(끝의 `/` 한 개는 제거)
    #[serde(rename = "URL")]
    pub url: String,
    /// Portal API Key
    pub api_key: String,
}

impl PortalConfig {
    pub fn new(url: &str, api_key: &str) -> Self {
        Self {
            url: url.strip_suffix('/').unwrap_or(url).to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        self.url.is_empty() || self.api_key.is_empty()
    }
}

/// `Jenkins` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct JenkinsConfig {
    pub host: String,
    pub jenkins_port: i32,
    #[serde(rename = "DBPort")]
    pub db_port: i32,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl JenkinsConfig {
    /// 인수 순서는 원본 생성자(`host, jenkinsPort, dbPort, user, password, database`)와 같다.
    pub fn new(
        host: &str,
        jenkins_port: i32,
        db_port: i32,
        user: &str,
        password: &str,
        database: &str,
    ) -> Self {
        Self {
            host: host.to_string(),
            jenkins_port,
            db_port,
            user: user.to_string(),
            password: password.to_string(),
            database: database.to_string(),
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        is_blank(&self.host)
            || is_blank(&self.user)
            || is_blank(&self.password)
            || is_blank(&self.database)
    }

    /// DB 접속 문자열.
    pub fn connection_string(&self) -> String {
        format!(
            "Server={};Port={};Database={};Uid={};Pwd={};",
            self.host, self.db_port, self.database, self.user, self.password
        )
    }

    /// Jenkins 서버 URL.
    pub fn jenkins_url(&self) -> String {
        format!("http://{}:{}/", self.host, self.jenkins_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_ips_csv() {
        let c = AccessIpsConfig::from_csv("p", false, "t", "j", " a , ,b,", "v", "m", "s", "");
        assert_eq!(c.s3_url, ["a", "b"]);
        assert_eq!(c.password, "qwe123");
        assert!(
            AccessIpsConfig::from_csv("", false, "", "", "", "", "", "", "")
                .s3_url
                .is_empty()
        );
    }

    #[test]
    fn portal_strips_one_slash() {
        assert_eq!(PortalConfig::new("http://x//", "k").url, "http://x/");
        assert_eq!(PortalConfig::new("", "k").url, "");
    }
}
