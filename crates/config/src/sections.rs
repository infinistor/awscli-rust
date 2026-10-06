//! TESTCore `Data/Config/{MainConfig,ListObjConfig,UpDownConfig,CompareConfig,UsedSizeConfig,
//! DuplicateConfig,DBConfig,MultiSystemConfig}.cs` 이식.
//!
//! JSON 속성 이름과 구성은 `JsonExtensions.ToJsonString`(이름 변환 없음, 들여쓰기만 적용)과 같다.
//! `[JsonIgnore]` 속성(`IsEmpty`, `TotalFileCount`)은 직렬화하지 않고 메서드로만 제공한다.

use serde::Serialize;

use crate::enum_bucket_types::EnumBucketTypes;
use crate::user_data::UserData;
use crate::util::{UtilError, size_to_long};

const DEFAULT_PART_SIZE: i64 = 5 * 1024 * 1024; // 5MB

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// `Default` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MainConfig {
    /// 버킷명
    pub bucket_name: String,
    /// 스레드 접두사
    pub thread_prefix: String,
    /// 오브젝트 접두사
    pub object_prefix: String,
    /// 파일경로
    pub file_path: String,
    /// 파일크기
    pub file_size: i64,
    /// 멀티파트크기
    pub part_size: i64,
    /// 타겟 경로
    pub target_path: String,
    /// 관리자 모드 활성화 여부
    pub is_admin: bool,
    /// 재시도 횟수
    pub retry_count: i32,
}

impl MainConfig {
    /// 기본값 규칙: 스레드 접두사 `TH`, 오브젝트 접두사 `FILE`, 파일경로는 현재 디렉터리의 `/test`,
    /// 멀티파트 크기 0이면 5MB, 재시도 횟수가 음수면 3. 크기 문자열이 잘못되면 오류.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bucket_name: &str,
        thread_prefix: &str,
        object_prefix: &str,
        file_path: &str,
        file_size: &str,
        part_size: &str,
        retry_count: i32,
        target_path: &str,
        is_admin: bool,
    ) -> Result<Self, UtilError> {
        let or_default = |value: &str, default: &str| {
            if is_blank(value) {
                default.to_string()
            } else {
                value.to_string()
            }
        };
        let file_path = if is_blank(file_path) {
            // Directory.GetCurrentDirectory() + "/test"
            let cwd = std::env::current_dir().map(|p| p.display().to_string());
            format!("{}/test", cwd.unwrap_or_default())
        } else {
            file_path.to_string()
        };
        let file_size = size_to_long(file_size)?;
        let part_size = match size_to_long(part_size)? {
            0 => DEFAULT_PART_SIZE,
            n => n,
        };
        Ok(Self {
            bucket_name: bucket_name.to_string(),
            thread_prefix: or_default(thread_prefix, "TH"),
            object_prefix: or_default(object_prefix, "FILE"),
            file_path,
            file_size,
            part_size,
            target_path: target_path.to_string(),
            is_admin,
            retry_count: if retry_count < 0 { 3 } else { retry_count },
        })
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        is_blank(&self.bucket_name) || self.file_size < 1
    }

    pub fn set_bucket_name(&mut self, bucket_name: impl Into<String>) {
        self.bucket_name = bucket_name.into();
    }
    pub fn set_thread_prefix(&mut self, thread_prefix: impl Into<String>) {
        self.thread_prefix = thread_prefix.into();
    }
    pub fn set_object_prefix(&mut self, object_prefix: impl Into<String>) {
        self.object_prefix = object_prefix.into();
    }
    pub fn set_file_path(&mut self, file_path: impl Into<String>) {
        self.file_path = file_path.into();
    }
    /// `SetFileSize(string)`: 크기 문자열을 변환해 설정한다.
    pub fn set_file_size_str(&mut self, file_size: &str) -> Result<(), UtilError> {
        self.file_size = size_to_long(file_size)?;
        Ok(())
    }
    /// `SetFileSize(long)`
    pub fn set_file_size(&mut self, file_size: i64) {
        self.file_size = file_size;
    }
    /// `SetPartSize(string)`: 변환 결과가 0이어도 기본값으로 바꾸지 않는다(원본 동작).
    pub fn set_part_size_str(&mut self, part_size: &str) -> Result<(), UtilError> {
        self.part_size = size_to_long(part_size)?;
        Ok(())
    }
    /// `SetPartSize(long)`
    pub fn set_part_size(&mut self, part_size: i64) {
        self.part_size = part_size;
    }
    pub fn set_target_path(&mut self, path: impl Into<String>) {
        self.target_path = path.into();
    }
    pub fn set_is_admin(&mut self, is_admin: bool) {
        self.is_admin = is_admin;
    }
}

/// `ListObj` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListObjConfig {
    /// 오브젝트 Prefix
    pub prefix: String,
    /// 테스트 전처리 여부
    pub prepare: bool,
    /// 오브젝트 개수
    pub file_count: i32,
    /// 테스트 시간
    pub times: i32,
    /// 스레드 개수
    pub thread_count: i32,
}

impl ListObjConfig {
    pub fn new(
        prefix: &str,
        prepare: bool,
        file_count: i32,
        times: i32,
        thread_count: i32,
    ) -> Self {
        Self {
            prefix: prefix.to_string(),
            prepare,
            file_count,
            times,
            thread_count,
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        self.file_count < 1 || self.times < 1
    }
}

/// `UpDown` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpDownConfig {
    /// 읽기 비율
    pub read_ratio: i32,
    /// 쓰기 비율
    pub write_ratio: i32,
    /// 삭제 비율
    pub delete_ratio: i32,
    /// 쓰레드 수
    pub thread_count: i32,
    /// 쓰레드별 파일 갯수
    pub file_count: i32,
    /// 폴더 분할 수
    pub division_count: i32,
    /// 반복할 시간(sec)
    pub times: i32,
    /// 버킷 생성 방식(JSON에서는 숫자)
    pub bucket_type: EnumBucketTypes,
    /// ETag 비교여부
    #[serde(rename = "ETagCheck")]
    pub etag_check: bool,
    /// Chunk Encoding 사용 여부
    pub use_chunk_encoding: bool,
    /// 결과 저장 경로(설정 전에는 JSON `null`)
    pub save: Option<String>,
}

impl UpDownConfig {
    /// 기본값 규칙: `division_count < 1`이면 1000, `Empty` 버킷 방식은 `Prefix`,
    /// 읽기/쓰기 비율이 음수면 5, 삭제 비율이 음수면 0.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        read_ratio: i32,
        write_ratio: i32,
        delete_ratio: i32,
        thread_count: i32,
        file_count: i32,
        division_count: i32,
        times: i32,
        bucket_type: EnumBucketTypes,
        etag_check: bool,
        use_chunk_encoding: bool,
    ) -> Self {
        let division_count = if division_count < 1 {
            1000
        } else {
            division_count
        };
        let bucket_type = if bucket_type == EnumBucketTypes::Empty {
            EnumBucketTypes::Prefix
        } else {
            bucket_type
        };
        Self {
            read_ratio: if read_ratio < 0 { 5 } else { read_ratio },
            write_ratio: if write_ratio < 0 { 5 } else { write_ratio },
            delete_ratio: if delete_ratio < 0 { 0 } else { delete_ratio },
            thread_count,
            file_count,
            division_count,
            times,
            bucket_type,
            etag_check,
            use_chunk_encoding,
            save: None,
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        self.thread_count < 1 || (self.times < 1 && self.file_count < 1)
    }

    /// `TotalFileCount`(JSON 제외). 원본은 `int` 곱셈(오버플로 시 감김).
    pub fn total_file_count(&self) -> i32 {
        self.thread_count.wrapping_mul(self.file_count)
    }

    pub fn set_thread_count(&mut self, thread_count: i32) {
        self.thread_count = thread_count;
    }
    pub fn set_file_count(&mut self, file_count: i32) {
        self.file_count = file_count;
    }
    pub fn set_division_count(&mut self, division_count: i32) {
        self.division_count = division_count;
    }
    pub fn set_times(&mut self, times: i32) {
        self.times = times;
    }
    pub fn set_bucket_type(&mut self, bucket_type: EnumBucketTypes) {
        self.bucket_type = bucket_type;
    }
    pub fn set_etag_check(&mut self, etag_check: bool) {
        self.etag_check = etag_check;
    }
    pub fn set_read(&mut self, read: i32) {
        self.read_ratio = read;
    }
    pub fn set_write(&mut self, write: i32) {
        self.write_ratio = write;
    }
    pub fn set_delete(&mut self, delete: i32) {
        self.delete_ratio = delete;
    }
    pub fn set_save(&mut self, save: impl Into<String>) {
        self.save = Some(save.into());
    }
    pub fn set_use_chunk_encoding(&mut self, use_chunk_encoding: bool) {
        self.use_chunk_encoding = use_chunk_encoding;
    }
}

/// `Compare` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CompareConfig {
    /// 원본 버킷명
    pub source_bucket: String,
    /// 대상 버킷명
    pub target_bucket: String,
    /// DeleteMarker 비교 여부
    pub is_delete_marker: bool,
    /// ETag 비교 여부
    #[serde(rename = "ETagCheck")]
    pub etag_check: bool,
    /// checksum 비교 여부
    pub checksum_check: bool,
    /// Metadata 비교 여부. false면 List 결과만 비교하고 HeadObject를 생략한다.
    pub metadata_check: bool,
    /// Version 비교 여부
    pub version_check: bool,
    /// Replication 상태 확인 여부
    pub replication_check: bool,
    /// Tag 비교 여부
    pub tag_check: bool,
}

impl CompareConfig {
    /// 인수 순서는 원본 생성자와 같다.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_bucket: &str,
        target_bucket: &str,
        is_delete_marker: bool,
        etag_check: bool,
        checksum_check: bool,
        version_check: bool,
        metadata_check: bool,
        replication_check: bool,
        tag_check: bool,
    ) -> Self {
        Self {
            source_bucket: source_bucket.to_string(),
            target_bucket: target_bucket.to_string(),
            is_delete_marker,
            etag_check,
            checksum_check,
            metadata_check,
            version_check,
            replication_check,
            tag_check,
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        is_blank(&self.source_bucket) || is_blank(&self.target_bucket)
    }
}

/// `UsedSize` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UsedSizeConfig {
    pub bucket_prefix: String,
    pub file_size: i64,
    pub multi_part_size: i64,
}

impl UsedSizeConfig {
    /// 크기 문자열이 잘못되면 오류.
    pub fn new(
        bucket_prefix: &str,
        file_size: &str,
        multi_part_size: &str,
    ) -> Result<Self, UtilError> {
        Ok(Self {
            bucket_prefix: bucket_prefix.to_string(),
            file_size: size_to_long(file_size)?,
            multi_part_size: size_to_long(multi_part_size)?,
        })
    }
}

/// `Duplicate` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DuplicateConfig {
    /// 오브젝트 경로
    pub object_path: String,
    /// 반복 횟수
    pub loop_count: i32,
    /// 오브젝트 개수
    pub object_count: i32,
}

impl DuplicateConfig {
    pub fn new(object_path: &str, loop_count: i32, object_count: i32) -> Self {
        Self {
            object_path: object_path.to_string(),
            loop_count,
            object_count,
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        is_blank(&self.object_path) || self.loop_count < 1 || self.object_count < 1
    }
}

/// `DB` 섹션.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DbConfig {
    /// DB Host
    pub host: String,
    /// DB Port
    pub port: i32,
    /// DB User
    pub user: String,
    /// DB Password
    pub password: String,
    /// DB Name
    pub database: String,
}

impl DbConfig {
    pub fn new(host: &str, port: i32, user: &str, password: &str, database: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
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

    /// DB 연결 문자열.
    pub fn connection_string(&self) -> String {
        format!(
            "Server={};Port={};Database={};Uid={};Pwd={};",
            self.host, self.port, self.database, self.user, self.password
        )
    }
}

/// `MultiSystem` 섹션.
///
/// 원본은 `MainUser`와 **같은 `UserData` 객체**를 들고 있어, 이후 `MainUser.SetURL(...)` 같은 변경이
/// `MultiGateway`/`OldSystem`/`NewSystem`(호출 때마다 새로 만든다)에 반영된다. 이 동작을 유지하려고
/// 사용자 정보를 복사해 두지 않고, 해당 접근자와 직렬화에 `&UserData`를 넘겨받는다.
/// `Config`가 `main_user`를 넘겨 직렬화한다([`MultiSystemConfig::view`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiSystemConfig {
    /// 버킷 생성 방식
    pub bucket_type: EnumBucketTypes,
    /// 쓰레드 수
    pub thread_count: i32,
    /// 쓰레드별 파일 갯수
    pub file_count: i32,
    multi_gateway_url: String,
    old_system_url: String,
    new_system_url: String,
}

impl MultiSystemConfig {
    pub fn new(
        bucket_type: EnumBucketTypes,
        thread_count: i32,
        file_count: i32,
        multi_gateway_url: &str,
        old_system_url: &str,
        new_system_url: &str,
    ) -> Self {
        Self {
            bucket_type,
            thread_count,
            file_count,
            multi_gateway_url: multi_gateway_url.to_string(),
            old_system_url: old_system_url.to_string(),
            new_system_url: new_system_url.to_string(),
        }
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        self.thread_count < 1 || self.file_count < 1
    }

    /// `TotalFileCount`(JSON 제외). 원본은 `int * int`를 `long`으로 넓히므로 곱셈은 `int` 범위에서 감긴다.
    pub fn total_file_count(&self) -> i64 {
        i64::from(self.thread_count.wrapping_mul(self.file_count))
    }

    fn with_url(user: &UserData, url: &str) -> UserData {
        UserData::new(url, &user.region_name, &user.access_key, &user.secret_key)
    }

    pub fn multi_gateway(&self, user: &UserData) -> UserData {
        Self::with_url(user, &self.multi_gateway_url)
    }

    pub fn old_system(&self, user: &UserData) -> UserData {
        Self::with_url(user, &self.old_system_url)
    }

    pub fn new_system(&self, user: &UserData) -> UserData {
        Self::with_url(user, &self.new_system_url)
    }

    /// 사용자 정보를 묶어 JSON으로 내보낼 수 있는 보기.
    pub fn view<'a>(&'a self, user: &'a UserData) -> MultiSystemView<'a> {
        MultiSystemView { config: self, user }
    }
}

/// `MultiSystemConfig` + 사용자 정보. 원본 JSON(`BucketType`, `ThreadCount`, `FileCount`,
/// `MultiGateway`, `OldSystem`, `NewSystem`)과 같은 모양으로 직렬화한다.
#[derive(Debug, Clone, Copy)]
pub struct MultiSystemView<'a> {
    config: &'a MultiSystemConfig,
    user: &'a UserData,
}

impl Serialize for MultiSystemView<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("MultiSystemConfig", 6)?;
        s.serialize_field("BucketType", &self.config.bucket_type)?;
        s.serialize_field("ThreadCount", &self.config.thread_count)?;
        s.serialize_field("FileCount", &self.config.file_count)?;
        s.serialize_field("MultiGateway", &self.config.multi_gateway(self.user))?;
        s.serialize_field("OldSystem", &self.config.old_system(self.user))?;
        s.serialize_field("NewSystem", &self.config.new_system(self.user))?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_defaults() {
        let c = MainConfig::new("b", "", " ", "", "1K", "", -1, "", false).unwrap();
        assert_eq!(c.thread_prefix, "TH");
        assert_eq!(c.object_prefix, "FILE");
        assert!(c.file_path.ends_with("/test"));
        assert_eq!(
            (c.file_size, c.part_size, c.retry_count),
            (1000, 5 << 20, 3)
        );
        assert!(MainConfig::new("b", "", "", "", "zz", "", 0, "", false).is_err());
    }

    #[test]
    fn up_down_defaults() {
        let c = UpDownConfig::new(-1, -1, -1, 1, 1, 0, 1, EnumBucketTypes::Empty, false, false);
        assert_eq!((c.read_ratio, c.write_ratio, c.delete_ratio), (5, 5, 0));
        assert_eq!(c.division_count, 1000);
        assert_eq!(c.bucket_type, EnumBucketTypes::Prefix);
    }
}
