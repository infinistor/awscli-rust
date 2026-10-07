//! 메뉴별 도움말(원본 `if (help)` 분기). 예제 JSON은 .NET 출력(`tests/parity/baseline/cli-run/help.json`)을 그대로 쓴다.

use crate::menu::MenuList;
use crate::usage;

/// `DeleteObjects` 예제(`List<KeyVersion>`).
const DELETE_OBJECTS_EXAMPLE: &str = r#"[
  {
    "ETag": null,
    "Key": "Key1",
    "LastModifiedTime": null,
    "Size": null,
    "VersionId": "VersionId1"
  },
  {
    "ETag": null,
    "Key": "Key2",
    "LastModifiedTime": null,
    "Size": null,
    "VersionId": "VersionId2"
  }
]"#;

/// `PutObjectLegalHold` 예제(`ObjectLockLegalHold`).
const LEGAL_HOLD_EXAMPLE: &str = r#"{
  "Status": {
    "Value": "ON"
  }
}"#;

/// `PutObjectTagging` 예제(`Tagging`).
const TAGGING_EXAMPLE: &str = r#"{
  "TagSet": [
    {
      "Key": "0",
      "Value": "0"
    },
    {
      "Key": "1",
      "Value": "1"
    },
    {
      "Key": "2",
      "Value": "2"
    },
    {
      "Key": "3",
      "Value": "3"
    },
    {
      "Key": "4",
      "Value": "4"
    }
  ]
}"#;

/// 메뉴의 도움말 본문(끝 줄바꿈 없음).
pub(super) fn text(menu: MenuList) -> String {
    use MenuList::*;
    match menu {
        CopyObject => [
            usage::main_flag(usage::COPY_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::sub_flag(usage::SOURCE, "string", ""),
            usage::sub_flag(usage::SOURCE_KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
        ]
        .concat(),
        DeleteObject => [
            usage::main_flag(usage::DELETE_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", " : 삭제할 객체 버전"),
            usage::optional(usage::BYPASS, ""),
        ]
        .concat(),
        DeleteObjects => [
            usage::main_flag(usage::DELETE_OBJECTS, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::FILE, "string", ""),
            usage::optional(usage::QUIET, " : 무응답 모드"),
            usage::optional(usage::BYPASS, ""),
            format!("\nList\n{DELETE_OBJECTS_EXAMPLE}"),
        ]
        .concat(),
        DeleteObjectTagging => [
            usage::main_flag(usage::DELETE_OBJECT_TAGGING, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", " : 삭제할 객체 버전"),
            usage::optional(usage::BYPASS, ""),
        ]
        .concat(),
        GetObject => [
            usage::main_flag(usage::GET_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", " : 객체 버전"),
            usage::optional_value(usage::FILE, "string", " : 저장할 파일 경로"),
            usage::optional_value(usage::ENCRYPTION_KEY, "string", " : 암호화 키"),
            usage::optional_value(usage::SIZE, "string", " : 저장할 파일 크기(ex> 1KB, 10M)"),
            usage::optional_value(usage::START_BYTE, "string", " : 시작 바이트(ex> 1KB, 10M)"),
            usage::optional_value(usage::END_BYTE, "string", " : 끝 바이트(ex> 1KB, 10M)"),
        ]
        .concat(),
        GetObjectLegalHold => [
            usage::main_flag(usage::GET_OBJECT_LEGAL_HOLD, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
        ]
        .concat(),
        GetObjectLock => [
            usage::main_flag(usage::GET_OBJECT_LOCK, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
        ]
        .concat(),
        GetObjectRetention => [
            usage::main_flag(usage::GET_OBJECT_RETENTION, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
        ]
        .concat(),
        GetObjectTagging => [
            usage::main_flag(usage::GET_OBJECT_TAGGING, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
        ]
        .concat(),
        GetPresignedUrl => [
            usage::main_flag(usage::GET_PRESIGNED_URL, ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::DAYS, "int", " : 만료일"),
        ]
        .concat(),
        HeadObject => [
            usage::main_flag(usage::HEAD_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
            usage::optional_value(usage::ENCRYPTION_KEY, "string", ""),
        ]
        .concat(),
        RestoreObject => [
            usage::main_flag(usage::RESTORE_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
            usage::optional_value(usage::DAYS, "int", ""),
        ]
        .concat(),
        ListObjectVersions => [
            usage::main_flag(usage::LIST_OBJECT_VERSIONS, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::optional_value(usage::PREFIX, "string", ""),
            usage::optional_value(usage::DELIMITER, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", " : Next Version Id Marker"),
            usage::optional_value(usage::MARKER, "string", " : Next Key Marker"),
            usage::optional_value(usage::MAX_KEYS, "int", ""),
        ]
        .concat(),
        ListObjects => [
            usage::main_flag(usage::LIST_OBJECTS, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::optional_value(usage::PREFIX, "string", ""),
            usage::optional_value(usage::DELIMITER, "string", ""),
            usage::optional_value(usage::MARKER, "string", " : Next Key Marker"),
            usage::optional_value(usage::MAX_KEYS, "int", ""),
        ]
        .concat(),
        ListObjectsV2 => [
            usage::main_flag(usage::LIST_OBJECTS_V2, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::optional_value(usage::PREFIX, "string", ""),
            usage::optional_value(usage::DELIMITER, "string", ""),
            usage::optional_value(usage::MARKER, "string", " : Start After"),
            usage::optional_value(usage::MAX_KEYS, "int", ""),
        ]
        .concat(),
        PutObject => [
            usage::main_flag(usage::PUT_OBJECT, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::CHECKSUM_TYPE, "string", " : 체크섬 타입"),
            usage::optional_value(usage::ACL, "string", ""),
            usage::optional_value(usage::BODY, "string", ""),
            usage::optional_value(usage::FILE, "string", " : 파일 경로"),
            usage::optional(usage::MD5SUM, " : MD5 체크섬 계산 및 전송"),
            usage::optional(
                usage::LOCK_ENABLE,
                " : Lock 모드 업로드. 파일의 ContentMD5 추가 전달.",
            ),
            usage::optional_value(usage::STORAGE_CLASS, "string", ""),
            usage::optional_value(usage::ENCRYPTION_KEY, "string", ""),
            usage::optional_value(
                usage::TAG_SET,
                "string",
                " : 태그 설정 정보(ex>tag1=value1,tag2=value2)",
            ),
        ]
        .concat(),
        PutObjects => [
            usage::main_flag(usage::PUT_OBJECTS, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::FILE, "string", " : 폴더 경로"),
        ]
        .concat(),
        PutObjectLegalHold => [
            usage::main_flag(usage::PUT_OBJECT_LEGAL_HOLD, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::sub_flag(usage::FILE, "string", " : LegalHold 설정 파일 경로"),
            format!("\nLegalHold\n{LEGAL_HOLD_EXAMPLE}"),
        ]
        .concat(),
        PutObjectLock => [
            usage::main_flag(usage::PUT_OBJECT_LOCK, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::optional_value(usage::DAYS, "string", " : 보관 기간(일)"),
            usage::optional_value(usage::YEARS, "string", " : 보관 기간(년)"),
            usage::optional_value(
                usage::LOCK_MODE,
                "string",
                " : 버킷에 lock 모드 설정(Compliance/Governance). 기본값은 Compliance",
            ),
        ]
        .concat(),
        PutObjectRetention => [
            usage::main_flag(usage::PUT_OBJECT_RETENTION, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::sub_flag(usage::DATE, "string", " : 보관 만료 날짜(ex> 2025-01-01)"),
            usage::optional_value(
                usage::LOCK_MODE,
                "string",
                " : 버킷에 lock 모드 설정(Compliance/Governance). 기본값은 Compliance",
            ),
            usage::optional(usage::BYPASS, ""),
        ]
        .concat(),
        PutObjectTagging => [
            usage::main_flag(usage::PUT_OBJECT_TAGGING, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::sub_flag(usage::FILE, "string", " : 태그 설정 파일 경로"),
            format!("\nTagging\n{TAGGING_EXAMPLE}"),
        ]
        .concat(),
        StorageMove => [
            usage::main_flag(usage::STORAGE_MOVE, ""),
            usage::sub_flag(usage::BUCKET, "string", ""),
            usage::sub_flag(usage::KEY, "string", ""),
            usage::optional_value(usage::STORAGE_CLASS, "string", ""),
            usage::optional_value(usage::VERSION_ID, "string", ""),
        ]
        .concat(),
        _ => unreachable!("object 모듈의 메뉴가 아니다: {menu:?}"),
    }
}
