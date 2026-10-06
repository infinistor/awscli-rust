//! 클라이언트 쪽 설정 DTO 이식: `Data/Config/{CopyConfig,UpDownClientConfig,MultiSystemClientconfig}.cs`.
//! `Config`가 직접 만들지는 않고 CLI/시나리오가 쓴다.

use serde::Serialize;

use crate::enum_bucket_types::EnumBucketTypes;

const DEFAULT_THREAD_PREFIX: &str = "TH";

fn thread_prefix_or_default(prefix: &str) -> String {
    if prefix.trim().is_empty() {
        DEFAULT_THREAD_PREFIX.to_string()
    } else {
        prefix.to_string()
    }
}

/// `CopyConfig`: 원본/대상 버킷·오브젝트 이름.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CopyConfig {
    /// 원본 버킷명
    pub source_bucket: String,
    /// 원본 오브젝트명
    pub source_object: String,
    /// 대상 버킷명
    pub target_bucket: String,
    /// 대상 오브젝트명
    pub target_object: String,
}

impl CopyConfig {
    pub fn new(
        source_bucket: &str,
        source_object: &str,
        target_bucket: &str,
        target_object: &str,
    ) -> Self {
        Self {
            source_bucket: source_bucket.to_string(),
            source_object: source_object.to_string(),
            target_bucket: target_bucket.to_string(),
            target_object: target_object.to_string(),
        }
    }

    /// 각 항목을 빈 문자열로 초기화한다.
    pub fn init(&mut self) {
        *self = Self::default();
    }

    /// `IsEmpty`(JSON 제외)
    pub fn is_empty(&self) -> bool {
        [
            &self.source_bucket,
            &self.target_bucket,
            &self.source_object,
            &self.target_object,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
    }
}

/// `UpDownClientConfig`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpDownClientConfig {
    pub thread_prefix: String,
    pub object_prefix: String,
    pub read_ratio: i32,
    pub write_ratio: i32,
    pub delete_ratio: i32,
    pub file_size: i64,
    pub bucket_type: EnumBucketTypes,
    #[serde(rename = "ETagCheck")]
    pub etag_check: bool,
    pub division_count: i32,
    pub retry_count: i32,
    pub is_admin: bool,
    pub use_chunk_encoding: bool,
    /// 분산 실행에서 조회·삭제 범위를 스레드 prefix로 제한한다.
    pub distributed: bool,
}

impl UpDownClientConfig {
    /// 스레드 접두사가 비어 있으면 `TH`, `division_count < 1`이면 1000. `distributed`는 false로 시작한다.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_prefix: &str,
        object_prefix: &str,
        read_ratio: i32,
        write_ratio: i32,
        delete_ratio: i32,
        file_size: i64,
        bucket_type: EnumBucketTypes,
        etag_check: bool,
        division_count: i32,
        retry_count: i32,
        is_admin: bool,
        use_chunk_encoding: bool,
    ) -> Self {
        Self {
            thread_prefix: thread_prefix_or_default(thread_prefix),
            object_prefix: object_prefix.to_string(),
            read_ratio,
            write_ratio,
            delete_ratio,
            file_size,
            bucket_type,
            etag_check,
            division_count: if division_count < 1 {
                1000
            } else {
                division_count
            },
            retry_count,
            is_admin,
            use_chunk_encoding,
            distributed: false,
        }
    }
}

/// `MultiSystemClientConfig`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MultiSystemClientConfig {
    pub thread_prefix: String,
    pub object_prefix: String,
    pub file_count: i32,
    pub file_size: i64,
    pub part_size: i64,
    pub bucket_type: EnumBucketTypes,
}

impl MultiSystemClientConfig {
    pub fn new(
        thread_prefix: &str,
        object_prefix: &str,
        file_count: i32,
        file_size: i64,
        part_size: i64,
        bucket_type: EnumBucketTypes,
    ) -> Self {
        Self {
            thread_prefix: thread_prefix_or_default(thread_prefix),
            object_prefix: object_prefix.to_string(),
            file_count,
            file_size,
            part_size,
            bucket_type,
        }
    }
}
