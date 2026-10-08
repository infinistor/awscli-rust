//! 원본 `#region Multipart Function`.

use aws_sdk_s3::operation::abort_multipart_upload::AbortMultipartUploadOutput;
use aws_sdk_s3::operation::complete_multipart_upload::CompleteMultipartUploadOutput;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::operation::list_multipart_uploads::ListMultipartUploadsOutput;
use aws_sdk_s3::operation::list_parts::ListPartsOutput;
use aws_sdk_s3::operation::upload_part::UploadPartOutput;
use aws_sdk_s3::operation::upload_part_copy::UploadPartCopyOutput;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};

use super::error::required_field;
use super::mime::initiate_content_type;
use super::object_config::copy_source;
use super::{
    PutBody, S3Client, S3Error, S3Response, UNSET, add_content_md5, chunk_checksum,
    set_content_type, strip_unset_query, zero_content_length,
};

/// .NET의 `UploadPart`·`CopyPart` 마샬러는 `UploadId`가 `null`이면 쿼리에서 뺀 채 요청을 보낸다. 호출 쪽은
/// 시작 응답에 `UploadId`가 없으면 빈 문자열을 넘기므로, 빈 값은 표식으로 바꿔 서명 전에 쿼리에서 지운다.
/// (`CompleteMultipartUpload`·`AbortMultipartUpload`·`ListParts`는 요청을 만들지 않고 `required_field` 예외.)
fn unset_if_empty(upload_id: &str) -> &str {
    if upload_id.is_empty() {
        UNSET
    } else {
        upload_id
    }
}

/// 원본 `PartETag`: 완료할 파트 번호와 ETag(와 파트 체크섬). 모두 비어 있을 수 있다(입력 파일에서 읽은 값).
///
/// .NET의 `ChecksumMD5`, `ChecksumSHA512`, `ChecksumXXHASH*`는 Rust SDK의 `CompletedPart`에 없어 옮기지 않았다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartETag {
    pub part_number: Option<i32>,
    pub e_tag: Option<String>,
    pub checksum_crc32: Option<String>,
    pub checksum_crc32_c: Option<String>,
    pub checksum_crc64_nvme: Option<String>,
    pub checksum_sha1: Option<String>,
    pub checksum_sha256: Option<String>,
}

impl PartETag {
    /// 원본 `new PartETag(partNumber, eTag)`. 응답에 ETag가 없으면(.NET `null`) 완료 요청 XML에서 `<ETag>`가 빠진다.
    pub fn new(part_number: i32, e_tag: Option<&str>) -> Self {
        Self {
            part_number: Some(part_number),
            e_tag: e_tag.map(str::to_string),
            ..Self::default()
        }
    }
}

impl S3Client {
    /// 원본 `InitiateMultipartUpload(bucketName, key)`. .NET은 키의 확장자로 `Content-Type`을 정한다
    /// (확장자가 없으면 보내지 않는다).
    pub async fn initiate_multipart_upload(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<S3Response<CreateMultipartUploadOutput>, S3Error> {
        self.initiate_multipart_upload_typed(bucket_name, key, initiate_content_type(key))
            .await
    }

    /// `ContentType`을 직접 정한 `InitiateMultipartUpload`(`TransferUtility`가 쓴다).
    pub(crate) async fn initiate_multipart_upload_typed(
        &self,
        bucket_name: &str,
        key: &str,
        content_type: Option<&str>,
    ) -> Result<S3Response<CreateMultipartUploadOutput>, S3Error> {
        send!(
            self.client
                .create_multipart_upload()
                .bucket(bucket_name)
                .key(key)
                .set_content_type(content_type.map(str::to_string)),
            mutate = zero_content_length
        )
    }

    /// 원본 `UploadPart(bucketName, key, uploadId, PartNumber, filePath, filePosition, inputStream, partSize, useChunkEncoding = true)`.
    /// 파일 본문은 `file_position`부터 `part_size`바이트(`part_size < 0`이면 끝까지)를 보낸다.
    /// .NET은 파트 요청에 항상 `Content-Type: text/plain`을 붙인다.
    #[allow(clippy::too_many_arguments)]
    pub async fn upload_part(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        body: PutBody,
        file_position: i64,
        part_size: i64,
        use_chunk_encoding: bool,
    ) -> Result<S3Response<UploadPartOutput>, S3Error> {
        let stream = body
            .into_stream(use_chunk_encoding, file_position, part_size)
            .await?;
        send!(
            self.client
                .upload_part()
                .bucket(bucket_name)
                .key(key)
                .upload_id(unset_if_empty(upload_id))
                .part_number(part_number)
                .body(stream),
            checksum = chunk_checksum(use_chunk_encoding),
            mutate = |request| {
                set_content_type(request, "text/plain");
                strip_unset_query(request);
            }
        )
    }

    /// 원본 `CopyPart(sourceBucket, sourceKey, destinationBucket, destinationKey, uploadId, partNumber, start, end, versionId = null)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn copy_part(
        &self,
        source_bucket: &str,
        source_key: &str,
        destination_bucket: &str,
        destination_key: &str,
        upload_id: &str,
        part_number: i32,
        start: i64,
        end: i64,
        version_id: Option<&str>,
    ) -> Result<S3Response<UploadPartCopyOutput>, S3Error> {
        send!(
            self.client
                .upload_part_copy()
                .bucket(destination_bucket)
                .key(destination_key)
                .copy_source(copy_source(source_bucket, source_key, version_id))
                .upload_id(unset_if_empty(upload_id))
                .part_number(part_number)
                .copy_source_range(format!("bytes={start}-{end}")),
            mutate = |request| {
                zero_content_length(request);
                strip_unset_query(request);
            }
        )
    }

    /// 원본 `CompleteMultipartUpload(bucketName, key, uploadId, List<PartETag> Parts)`.
    /// .NET SDK는 `Content-MD5`를 붙이므로 같게 맞춘다.
    pub async fn complete_multipart_upload(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
        parts: &[PartETag],
    ) -> Result<S3Response<CompleteMultipartUploadOutput>, S3Error> {
        required_field(upload_id, "UploadId")?;
        let completed = CompletedMultipartUpload::builder()
            .set_parts(Some(
                parts
                    .iter()
                    .map(|p| {
                        CompletedPart::builder()
                            .set_part_number(p.part_number)
                            .set_e_tag(p.e_tag.clone())
                            .set_checksum_crc32(p.checksum_crc32.clone())
                            .set_checksum_crc32_c(p.checksum_crc32_c.clone())
                            .set_checksum_crc64_nvme(p.checksum_crc64_nvme.clone())
                            .set_checksum_sha1(p.checksum_sha1.clone())
                            .set_checksum_sha256(p.checksum_sha256.clone())
                            .build()
                    })
                    .collect(),
            ))
            .build();
        send!(
            self.client
                .complete_multipart_upload()
                .bucket(bucket_name)
                .key(key)
                .upload_id(upload_id)
                .multipart_upload(completed),
            mutate = add_content_md5
        )
    }

    /// 원본 `AbortMultipartUpload(bucketName, key, uploadId)`.
    pub async fn abort_multipart_upload(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<S3Response<AbortMultipartUploadOutput>, S3Error> {
        required_field(upload_id, "UploadId")?;
        send!(
            self.client
                .abort_multipart_upload()
                .bucket(bucket_name)
                .key(key)
                .upload_id(upload_id)
        )
    }

    /// 원본 `ListMultipartUploads(bucketName, prefix = null, uploadIdMarker = null, keyMarker = null, maxKeys = 1000, delimiter = null)`.
    pub async fn list_multipart_uploads(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
        upload_id_marker: Option<&str>,
        key_marker: Option<&str>,
        max_keys: i32,
        delimiter: Option<&str>,
    ) -> Result<S3Response<ListMultipartUploadsOutput>, S3Error> {
        send!(
            self.client
                .list_multipart_uploads()
                .bucket(bucket_name)
                .max_uploads(max_keys)
                .set_prefix(prefix.map(str::to_string))
                .set_delimiter(delimiter.map(str::to_string))
                .set_upload_id_marker(upload_id_marker.map(str::to_string))
                .set_key_marker(key_marker.map(str::to_string))
        )
    }

    /// 원본 `ListParts(bucketName, key, uploadId, partNumberMarker = 0, maxKeys = 1000)`.
    /// `part_number_marker`는 0보다 클 때만 보낸다.
    pub async fn list_parts(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
        part_number_marker: i32,
        max_keys: i32,
    ) -> Result<S3Response<ListPartsOutput>, S3Error> {
        required_field(upload_id, "UploadId")?;
        send!(
            self.client
                .list_parts()
                .bucket(bucket_name)
                .key(key)
                .upload_id(upload_id)
                .max_parts(max_keys)
                .set_part_number_marker(
                    (part_number_marker > 0).then(|| part_number_marker.to_string())
                )
        )
    }
}
