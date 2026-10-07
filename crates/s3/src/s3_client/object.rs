//! 원본 `#region Object Function`, `#region Multipart Function` 중 기본 연산.

use std::path::PathBuf;

use aws_sdk_s3::operation::delete_object::DeleteObjectOutput;
use aws_sdk_s3::operation::delete_objects::DeleteObjectsOutput;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use aws_sdk_s3::operation::list_objects::ListObjectsOutput;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use aws_sdk_s3::operation::put_object::PutObjectOutput;
use aws_sdk_s3::primitives::{ByteStream, Length, SdkBody};
use aws_sdk_s3::types::{ChecksumMode, Delete, ObjectIdentifier, Tag};
use bytes::Bytes;
use http_body_util::Full;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::mime::put_object_content_type;
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
    /// SDK 본문으로 바꾼다. `streaming`이면 데이터를 스트림 본문으로 만든다. SDK는 스트림 본문을
    /// 체크섬 트레일러가 붙은 청크 서명(`STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER`)으로 보내고,
    /// 메모리 본문은 체크섬을 헤더에 넣어 한 번에 보낸다. `streaming`이 아니면 .NET처럼 본문 전체의
    /// SHA256을 서명하므로 파일도 메모리로 읽는다.
    ///
    /// `position`/`size`는 원본 `UploadPart`의 `FilePosition`/`PartSize`: 파일은 `position`부터 `size`바이트
    /// (`size < 0`이면 끝까지), 메모리 데이터는 앞에서 `size`바이트만 보낸다.
    pub(crate) async fn into_stream(
        self,
        streaming: bool,
        position: i64,
        size: i64,
    ) -> Result<ByteStream, S3Error> {
        let memory = |mut bytes: Vec<u8>| {
            if let Ok(size) = usize::try_from(size) {
                bytes.truncate(size);
            }
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
            Self::Text(text) => memory(text.into_bytes()),
            Self::Bytes(bytes) => memory(bytes),
            Self::File(path) => {
                let io = |e: std::io::Error| S3Error::io(&path, &e);
                let length = tokio::fs::metadata(&path).await.map_err(io)?.len();
                let start = u64::try_from(position).unwrap_or(0).min(length);
                let remaining = length - start;
                let take = u64::try_from(size).map_or(remaining, |s| s.min(remaining));
                if streaming {
                    ByteStream::read_from()
                        .path(&path)
                        .offset(start)
                        .length(Length::Exact(take))
                        .build()
                        .await
                        .map_err(|e| S3Error::Request(e.to_string()))?
                } else {
                    let mut file = tokio::fs::File::open(&path).await.map_err(io)?;
                    file.seek(std::io::SeekFrom::Start(start))
                        .await
                        .map_err(io)?;
                    let mut buffer = Vec::new();
                    file.take(take).read_to_end(&mut buffer).await.map_err(io)?;
                    ByteStream::from(buffer)
                }
            }
        })
    }

    /// `PutObject`의 `Content-Type` 기본값(.NET은 파일이면 파일 경로, 아니면 키의 확장자로 정한다).
    fn put_content_type(&self, key: &str) -> &'static str {
        match self {
            Self::File(path) => put_object_content_type(&path.to_string_lossy()),
            _ => put_object_content_type(key),
        }
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
        let content_type = body.put_content_type(key);
        let stream = body.into_stream(use_chunk_encoding, 0, -1).await?;
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
                .content_type(content_type)
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
        let content_type = body.put_content_type(key);
        let stream = body.into_stream(use_chunk_encoding, 0, -1).await?;
        send!(
            self.client
                .put_object()
                .bucket(bucket_name)
                .key(key)
                .body(stream)
                .content_type(content_type)
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
