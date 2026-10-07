//! TESTCore `Mover/Data/*` 이식. 속성 이름은 원본 그대로 PascalCase(`Move_size` 포함)이다.

use serde::{Deserialize, Serialize};

use crate::json::{Deserializer, FromJson, JsonError, Token};

/// Mover 작업 상태
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MoverStatus {
    pub job_id: i32,
    pub status: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub error_desc: Option<String>,
    pub total_count: i64,
    pub total_size: Option<String>,
    pub moved_count: i64,
    pub moved_size: Option<String>,
    pub skipped_count: i64,
    pub skipped_size: Option<String>,
    pub failed_count: i64,
    pub failed_size: Option<String>,
    pub deleted_count: i64,
    pub deleted_size: Option<String>,
    pub progress: Option<String>,
}

impl FromJson for MoverStatus {
    fn type_name() -> String {
        "TestCore.Mover.Data.MoverStatus".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        d.read_object(tok, &Self::type_name(), Self::default(), |d, o, name| {
            match name {
                "JobId" => o.job_id = d.read_value()?,
                "Status" => o.status = d.read_nullable()?,
                "Source" => o.source = d.read_nullable()?,
                "Target" => o.target = d.read_nullable()?,
                "StartTime" => o.start_time = d.read_nullable()?,
                "EndTime" => o.end_time = d.read_nullable()?,
                "ErrorDesc" => o.error_desc = d.read_nullable()?,
                "TotalCount" => o.total_count = d.read_value()?,
                "TotalSize" => o.total_size = d.read_nullable()?,
                "MovedCount" => o.moved_count = d.read_value()?,
                "MovedSize" => o.moved_size = d.read_nullable()?,
                "SkippedCount" => o.skipped_count = d.read_value()?,
                "SkippedSize" => o.skipped_size = d.read_nullable()?,
                "FailedCount" => o.failed_count = d.read_value()?,
                "FailedSize" => o.failed_size = d.read_nullable()?,
                "DeletedCount" => o.deleted_count = d.read_value()?,
                "DeletedSize" => o.deleted_size = d.read_nullable()?,
                "Progress" => o.progress = d.read_nullable()?,
                _ => return Ok(false),
            }
            Ok(true)
        })
    }
}

/// Mover 원본(소스) 설정
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct SourceConfig {
    pub mount_point: Option<String>,
    pub endpoint: Option<String>,
    pub bucket: Option<String>,
    pub access: Option<String>,
    pub secret: Option<String>,
    pub prefix: Option<String>,
    #[serde(rename = "Move_size")]
    pub move_size: Option<String>,
}

/// Mover 대상(타깃) 설정
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct TargetConfig {
    pub endpoint: Option<String>,
    pub access: Option<String>,
    pub secret: Option<String>,
    pub bucket: Option<String>,
    pub prefix: Option<String>,
}
