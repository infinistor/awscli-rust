//! 원본 `#region Object Function`, `#region Multipart Function` 중 기본 연산.

use std::path::PathBuf;

use aws_sdk_s3::operation::delete_object::DeleteObjectOutput;
use aws_sdk_s3::operation::delete_objects::DeleteObjectsOutput;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use aws_sdk_s3::operation::list_objects::ListObjectsOutput;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use aws_sdk_s3::operation::put_object::PutObjectOutput;
use aws_sdk_s3::operation::upload_part::UploadPartOutput;
use aws_sdk_s3::primitives::{ByteStream, SdkBody};
use aws_sdk_s3::types::{ChecksumMode, Delete, ObjectIdentifier, Tag};
use bytes::Bytes;
use http_body_util::Full;

use super::{S3Client, S3Error, S3Response, chunk_checksum};

/// 업로드 본문. 원본 `PutObjectRequest`의 `ContentBody`, `FilePath`, `InputStream`에 대응한다.
#[derive(Debug, Clone)]
pub enum PutBody {
    /// `ContentBody`: 문자열(UTF-8). .NET은 `Content-Type: text/plain`을 붙인다.
    Text(String),
    /// `FilePath`.
    File(PathBuf),
    /// `InputStream`(메모리).
    Bytes(Vec<u8>),
}

impl PutBody {
    /// SDK 본문으로 바꾼다. `streaming`이면 메모리 데이터도 스트림 본문으로 만든다. SDK는 스트림 본문을
    /// 체크섬 트레일러가 붙은 청크 서명(`STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER`)으로 보내고,
    /// 메모리 본문은 체크섬을 헤더에 넣어 한 번에 보낸다. 파일은 항상 스트림이다.
    async fn into_stream(
        self,
        streaming: bool,
    ) -> Result<(ByteStream, Option<&'static str>), S3Error> {
        let memory = |bytes: Vec<u8>| {
            if streaming {
                let bytes = Bytes::from(bytes);
                ByteStream::new(SdkBody::retryable(move || {
                    SdkBody::from_body_1_x(Full::new(bytes.clone()))
                }))
            } else {
                ByteStream::from(bytes)
            }
        };
        Ok(match self {
            Self::Text(text) => (memory(text.into_bytes()), Some("text/plain")),
            Self::Bytes(bytes) => (memory(bytes), None),
            Self::File(path) => (
                ByteStream::from_path(&path)
                    .await
                    .map_err(|e| S3Error::Request(e.to_string()))?,
                None,
            ),
        })
    }
}

/// 원본 `ByteRange(start, end)`를 `Range` 헤더 값으로.
pub fn byte_range(start: i64, end: i64) -> String {
    format!("bytes={start}-{end}")
}

impl S3Client {
    /// 원본 `PutObject(bucketName, key, filePath, body, inputStream, useChunkEncoding = true, tagSet)`.
    pub async fn put_object(
        &self,
        bucket_name: &str,
        key: &str,
        body: PutBody,
        use_chunk_encoding: bool,
        tag_set: Option<Vec<Tag>>,
    ) -> Result<S3Response<PutObjectOutput>, S3Error> {
        let (stream, content_type) = body.into_stream(use_chunk_encoding).await?;
        let tagging = tag_set.map(|tags| {
            tags.iter()
                .map(|t| format!("{}={}", url_encode(t.key()), url_encode(t.value())))
                .collect::<Vec<_>>()
                .join("&")
        });
        send!(
            self.client
                .put_object()
                .bucket(bucket_name)
                .key(key)
                .body(stream)
                .set_content_type(content_type.map(str::to_string))
                .set_tagging(tagging),
            checksum = chunk_checksum(use_chunk_encoding)
        )
    }

    /// `PutObjectRequest.ChecksumAlgorithm`을 지정한 업로드(원본 `--checksum-type` 경로).
    pub async fn put_object_with_checksum(
        &self,
        bucket_name: &str,
        key: &str,
        body: PutBody,
        use_chunk_encoding: bool,
        algorithm: aws_sdk_s3::types::ChecksumAlgorithm,
    ) -> Result<S3Response<PutObjectOutput>, S3Error> {
        let (stream, content_type) = body.into_stream(use_chunk_encoding).await?;
        send!(
            self.client
                .put_object()
                .bucket(bucket_name)
                .key(key)
                .body(stream)
                .set_content_type(content_type.map(str::to_string))
                .checksum_algorithm(algorithm),
            // .NET은 알고리즘을 지정하면 항상 체크섬을 계산한다(청크면 트레일러, 아니면 헤더).
            checksum = aws_sdk_s3::config::RequestChecksumCalculation::WhenSupported
        )
    }

    /// 원본 `GetObject(bucketName, key, versionId = null, ByteRange range = null)`.
    pub async fn get_object(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
        range: Option<(i64, i64)>,
    ) -> Result<S3Response<GetObjectOutput>, S3Error> {
        send!(
            self.client
                .get_object()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
                .set_range(range.map(|(s, e)| byte_range(s, e)))
        )
    }

    /// 원본 `HeadObject(bucketName, key, versionId = null, ChecksumMode checksumMode = null)`.
    pub async fn head_object(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
        checksum_mode: Option<ChecksumMode>,
    ) -> Result<S3Response<HeadObjectOutput>, S3Error> {
        send!(
            self.client
                .head_object()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
                .set_checksum_mode(checksum_mode)
        )
    }

    /// 원본 `ListObjects(bucketName, prefix, marker, maxKeys = 1000, delimiter)`.
    pub async fn list_objects(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
        marker: Option<&str>,
        max_keys: i32,
        delimiter: Option<&str>,
    ) -> Result<S3Response<ListObjectsOutput>, S3Error> {
        send!(
            self.client
                .list_objects()
                .bucket(bucket_name)
                .max_keys(max_keys)
                .set_delimiter(delimiter.map(str::to_string))
                .set_marker(marker.map(str::to_string))
                .set_prefix(prefix.map(str::to_string))
        )
    }

    /// 원본 `ListObjectsV2(bucketName, prefix, startAfter, maxKeys = 1000, delimiter, continuationToken)`.
    pub async fn list_objects_v2(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
        start_after: Option<&str>,
        max_keys: i32,
        delimiter: Option<&str>,
        continuation_token: Option<&str>,
    ) -> Result<S3Response<ListObjectsV2Output>, S3Error> {
        send!(
            self.client
                .list_objects_v2()
                .bucket(bucket_name)
                .max_keys(max_keys)
                .set_delimiter(delimiter.map(str::to_string))
                .set_continuation_token(continuation_token.map(str::to_string))
                .set_prefix(prefix.map(str::to_string))
                .set_start_after(start_after.map(str::to_string))
        )
    }

    /// 원본 `DeleteObject(bucketName, key, versionId = null, bool? bypass = null)`.
    /// `bypass`가 `true`일 때만 헤더를 보낸다(false여도 보내면 AWS가 거부한다).
    pub async fn delete_object(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
        bypass: Option<bool>,
    ) -> Result<S3Response<DeleteObjectOutput>, S3Error> {
        send!(
            self.client
                .delete_object()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
                .set_bypass_governance_retention((bypass == Some(true)).then_some(true))
        )
    }

    /// 원본 `DeleteObjects(bucketName, keyList, bool? bypass = null, bool? Quiet = null)`.
    /// `key_list`는 (키, 버전 ID).
    pub async fn delete_objects(
        &self,
        bucket_name: &str,
        key_list: &[(String, Option<String>)],
        bypass: Option<bool>,
        quiet: Option<bool>,
    ) -> Result<S3Response<DeleteObjectsOutput>, S3Error> {
        let objects = key_list
            .iter()
            .map(|(key, version)| {
                ObjectIdentifier::builder()
                    .key(key)
                    .set_version_id(version.clone())
                    .build()
                    .map_err(|e| S3Error::Request(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let delete = Delete::builder()
            .set_objects(Some(objects))
            .set_quiet(quiet)
            .build()
            .map_err(|e| S3Error::Request(e.to_string()))?;
        send!(
            self.client
                .delete_objects()
                .bucket(bucket_name)
                .delete(delete)
                .set_bypass_governance_retention((bypass == Some(true)).then_some(true))
        )
    }

    /// 원본 `UploadPart(bucketName, key, uploadId, PartNumber, filePath, filePosition, inputStream, partSize, useChunkEncoding = true)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn upload_part(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        body: PutBody,
        use_chunk_encoding: bool,
    ) -> Result<S3Response<UploadPartOutput>, S3Error> {
        let (stream, _) = body.into_stream(use_chunk_encoding).await?;
        send!(
            self.client
                .upload_part()
                .bucket(bucket_name)
                .key(key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(stream),
            checksum = chunk_checksum(use_chunk_encoding)
        )
    }
}

/// `Tagging` 헤더 값 인코딩(RFC 3986).
fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
