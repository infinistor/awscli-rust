//! `Test/CopyTest.cs`: 원본 오브젝트를 읽어 대상에 올린다(`RangeReadCopy` 메뉴).
//!
//! 원본과 같게 맞춘 동작
//!
//! - `Start(true)`: `HeadObject` → `CreateMultipartUpload` → 1GiB 범위마다 `GetObject(Range)` → `UploadPart` →
//!   `CompleteMultipartUpload`. 마지막에 `CopyTest End. {ms}ms`를 남긴다.
//! - 실패해도 멀티파트 업로드를 중단(`AbortMultipartUpload`)하지 않고, 예외는 잡지 않아 호출자(최상위)로 전파된다.
//! - 버킷·키가 비어 있으면(`null` 또는 빈 문자열) 요청을 만들 때 `ArgumentException`이라 요청을 보내지 않는다
//!   (`HeadObject`는 버킷 → 키, `CreateMultipartUpload`도 버킷 → 키 순서).
//!
//! 원본 버그(그대로 둔다)
//!
//! - `UploadPart(inputStream: part.ResponseStream)`의 입력은 응답 본문 스트림이라 탐색할 수 없다. 파트 크기를 정하지 않은 청크
//!   업로드는 .NET SDK가 `PartialWrapperStream`으로 감싸려다 `InvalidOperationException`(`Base stream of PartialWrapperStream
//!   must be seekable`)을 던진다. 그래서 크기가 0보다 큰 오브젝트는 첫 `GetObject(Range)` 응답을 받은 직후 이 예외로 끝나고
//!   (`UploadPart`·`CompleteMultipartUpload` 요청은 나가지 않는다), 크기가 0일 때만 파트 없는 `CompleteMultipartUpload`까지 간다.
//! - `Start(false)`(`GetObject` → `PutObject`)는 호출하는 곳이 없어 옮기지 않았다.

use std::time::Instant;

use awscli_rest_config::{CopyConfig, UserData};
use awscli_rest_s3::S3Client;
use awscli_rest_s3::s3_client::PartETag;
use tracing::info;

use crate::ScenarioError;

/// 원본 `Utility.GiB`.
const GIB: i64 = 1024 * 1024 * 1024;

/// 요청을 만들 때 필수 값이 비어 있으면 SDK가 던지는 `ArgumentException`.
fn required(value: &str, property: &str, request: &str) -> Result<(), ScenarioError> {
    if value.is_empty() {
        return Err(ScenarioError::new(
            "System.ArgumentException",
            format!(
                "{property} is a required property and must be set before making this call. (Parameter '{request}.{property}')"
            ),
        ));
    }
    Ok(())
}

/// 원본 `CopyTest`.
pub struct CopyTest {
    config: CopyConfig,
    main_user: UserData,
    /// 서로 다른 시스템 사이면 대상 사용자.
    alt_user: Option<UserData>,
}

impl CopyTest {
    /// 동일 시스템 안에서 복사한다.
    pub fn new(config: CopyConfig, main_user: UserData) -> Self {
        Self {
            config,
            main_user,
            alt_user: None,
        }
    }

    /// 서로 다른 시스템(계정) 사이를 복사한다.
    pub fn with_alt(config: CopyConfig, main_user: UserData, alt_user: UserData) -> Self {
        Self {
            config,
            main_user,
            alt_user: Some(alt_user),
        }
    }

    /// 원본 `Start(rangeRead: true)`.
    pub async fn start(&self) -> Result<(), ScenarioError> {
        let started = Instant::now();
        let config = &self.config;

        // 클라이언트 생성
        let source_client = S3Client::from_user(&self.main_user, false, 3, false);
        let target_client = match &self.alt_user {
            Some(alt) => S3Client::from_user(alt, false, 3, false),
            None => source_client.clone(),
        };

        // Metadata 가져오기
        required(
            &config.source_bucket,
            "BucketName",
            "GetObjectMetadataRequest",
        )?;
        required(&config.source_object, "Key", "GetObjectMetadataRequest")?;
        let metadata = source_client
            .head_object(&config.source_bucket, &config.source_object, None, None)
            .await
            .map_err(|e| ScenarioError::s3(e, &[]))?;
        let size = metadata.output.content_length().unwrap_or(0);

        let parts: Vec<PartETag> = Vec::new();

        required(
            &config.target_bucket,
            "BucketName",
            "InitiateMultipartUploadRequest",
        )?;
        required(
            &config.target_object,
            "Key",
            "InitiateMultipartUploadRequest",
        )?;
        let init = target_client
            .initiate_multipart_upload(&config.target_bucket, &config.target_object)
            .await
            .map_err(|e| ScenarioError::s3(e, &[]))?;
        let upload_id = init.output.upload_id().unwrap_or_default().to_string();

        let start = 0i64;
        let part_size = GIB;
        if start < size {
            let mut end = start + part_size;
            if end > size {
                end = size;
            }

            // `using var part`: 응답 본문은 읽지 않고 버린다.
            let _part = source_client
                .get_object(
                    &config.source_bucket,
                    &config.source_object,
                    None,
                    Some((start, end - 1)),
                )
                .await
                .map_err(|e| ScenarioError::s3(e, &["NoSuchKey", "InvalidObjectState"]))?;

            // `UploadPart(inputStream: part.ResponseStream)`: 응답 스트림은 탐색할 수 없어 SDK가 요청을 만들다 던진다.
            return Err(ScenarioError::new(
                "System.InvalidOperationException",
                "Base stream of PartialWrapperStream must be seekable",
            ));
        }
        target_client
            .complete_multipart_upload(
                &config.target_bucket,
                &config.target_object,
                &upload_id,
                &parts,
            )
            .await
            .map_err(|e| ScenarioError::s3(e, &[]))?;

        info!("CopyTest End. {}ms", started.elapsed().as_millis());
        Ok(())
    }
}
