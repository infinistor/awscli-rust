//! 원본 `#region TransferUtility Function`: `Upload`, `Download`.
//!
//! .NET `TransferUtility`가 보내는 요청을 캡처해 맞췄다.
//!
//! 업로드:
//!
//! - 크기가 `part_size`보다 작으면 `PutObject` 한 번(청크 서명), 같거나 크면 멀티파트다
//!   (`MinSizeBeforePartUpload = partSize`이므로 `partSize`와 같은 크기도 파트 1개짜리 멀티파트).
//! - 멀티파트는 `CreateMultipartUpload` → `UploadPart`(파트 크기 `max(partSize, ceil(크기 / 10000))`,
//!   동시 `threadCount`개) → `CompleteMultipartUpload`. 파트가 실패하면 `AbortMultipartUpload`를 보내고 오류를 돌려준다.
//! - 빈 파일은 청크 서명 없이 한 번에 보낸다.
//! - `Content-Type`: 지정하지 않으면 파일은 파일 확장자(`MimeTypeFromExtension`), 스트림은 키의 확장자로 정한다.
//!   파트 요청은 항상 `text/plain`이다.
//! - 파일 경로와 스트림을 함께 주면 .NET처럼 `ArgumentException`이다.
//!
//! 다운로드: `GetObject` 한 번(버전 ID가 있으면 포함)을 파일로 쓴다. 파일이 있으면 덮어쓰고 없는 디렉터리는
//! 만든다. 오류 응답이면 기존 파일은 그대로 둔다. ETag가 단일 파트 MD5(32자리 16진수)이고 서버 측 KMS·고객 키
//! 암호화가 아니면 받은 내용의 MD5와 비교한다(다르면 파일은 남기고 오류).

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use aws_sdk_s3::operation::put_object::PutObjectOutput;
use aws_sdk_s3::types::ServerSideEncryption;
use md5::{Digest, Md5};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::mime::{
    initiate_content_type, mime_type_from_extension, path_extension, put_object_content_type,
};
use super::{PartETag, PutBody, S3Client, S3Error, S3Response, chunk_checksum};

/// 멀티파트의 최대 파트 수.
const MAX_PARTS: i64 = 10_000;
/// 파일 경로와 스트림이 모두 있거나 둘 다 없을 때 .NET이 던지는 메시지.
const SOURCE_MESSAGE: &str =
    "Please specify one of either an InputStream or a FilePath to be PUT as an S3 object.";

/// 업로드 원본.
enum Source<'a> {
    File(&'a Path),
    Memory(Arc<Vec<u8>>),
}

/// 파트 하나의 원본. 메모리 데이터는 공유한 채 범위만 들고 있다.
enum PartSource {
    File(std::path::PathBuf),
    Memory(Arc<Vec<u8>>, std::ops::Range<usize>),
}

impl S3Client {
    /// 원본 `Upload(bucketName, key, filePath, partSize = 5MiB, threadCount = 10, Stream body = null, byte[] byteBody = null, contentType = null)`.
    ///
    /// `body`(스트림)는 호출하는 쪽에서 읽은 내용을 넘긴다. `byte_body`가 있으면 `body`보다 우선한다.
    #[allow(clippy::too_many_arguments)]
    pub async fn upload(
        &self,
        bucket_name: &str,
        key: &str,
        file_path: Option<&Path>,
        part_size: i64,
        thread_count: usize,
        body: Option<Vec<u8>>,
        byte_body: Option<Vec<u8>>,
        content_type: Option<&str>,
    ) -> Result<(), S3Error> {
        let stream = byte_body.or(body);
        let (source, size, default_single, default_initiate) = match (file_path, stream) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(S3Error::Argument(SOURCE_MESSAGE.to_string()));
            }
            (Some(path), None) => {
                let size = tokio::fs::metadata(path)
                    .await
                    .map_err(|e| S3Error::io(path, &e))?
                    .len();
                let mime = mime_type_from_extension(path_extension(&path.to_string_lossy()));
                (Source::File(path), size, Some(mime), Some(mime))
            }
            (None, Some(bytes)) => (
                Source::Memory(Arc::new(bytes)),
                0,
                Some(put_object_content_type(key)),
                initiate_content_type(key),
            ),
        };
        let size = match &source {
            Source::Memory(bytes) => bytes.len(),
            Source::File(_) => usize::try_from(size).unwrap_or(usize::MAX),
        };
        let size = i64::try_from(size).unwrap_or(i64::MAX);
        let part_size = part_size.max(1);

        if size < part_size {
            let content_type = content_type.or(default_single);
            return self
                .upload_single(bucket_name, key, &source, size, content_type)
                .await;
        }

        // .NET은 파트 수가 10000을 넘지 않도록 파트 크기를 키운다.
        let part_size = effective_part_size(size, part_size);
        let initiate_type = content_type.or(default_initiate);
        let upload_id = self
            .initiate_multipart_upload_typed(bucket_name, key, initiate_type)
            .await?
            .output
            .upload_id()
            .unwrap_or_default()
            .to_string();

        match self
            .upload_parts(
                bucket_name,
                key,
                &upload_id,
                &source,
                size,
                part_size,
                thread_count.max(1),
            )
            .await
        {
            Ok(parts) => {
                self.complete_multipart_upload(bucket_name, key, &upload_id, &parts)
                    .await?;
                Ok(())
            }
            Err(error) => {
                // 중단 요청의 실패는 원래 오류를 가리지 않는다.
                let _ = self
                    .abort_multipart_upload(bucket_name, key, &upload_id)
                    .await;
                Err(error)
            }
        }
    }

    async fn upload_single(
        &self,
        bucket_name: &str,
        key: &str,
        source: &Source<'_>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<(), S3Error> {
        // .NET은 빈 본문에는 청크 서명을 쓰지 않는다.
        let chunked = size > 0;
        let body = match source {
            Source::File(path) => PutBody::File(path.to_path_buf()),
            Source::Memory(bytes) => PutBody::Bytes(bytes.as_ref().clone()),
        };
        let stream = body.into_stream(chunked, 0, -1).await?;
        let response: Result<S3Response<PutObjectOutput>, S3Error> = send!(
            self.client
                .put_object()
                .bucket(bucket_name)
                .key(key)
                .body(stream)
                .set_content_type(content_type.map(str::to_string)),
            checksum = chunk_checksum(chunked)
        );
        response.map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    async fn upload_parts(
        &self,
        bucket_name: &str,
        key: &str,
        upload_id: &str,
        source: &Source<'_>,
        size: i64,
        part_size: i64,
        thread_count: usize,
    ) -> Result<Vec<PartETag>, S3Error> {
        let semaphore = Arc::new(Semaphore::new(thread_count));
        // 한 파트가 실패하면 아직 시작하지 않은 파트는 보내지 않는다(.NET은 취소 토큰으로 대기 중인 파트를 취소한다).
        let failed = Arc::new(AtomicBool::new(false));
        let mut tasks = JoinSet::new();
        let mut number = 0;
        let mut position = 0;
        while position < size {
            number += 1;
            let length = part_size.min(size - position);
            // 메모리 데이터는 작업이 세마포어를 얻은 뒤에 잘라 복사한다(동시 요청 수만큼만 복사본이 생긴다).
            let part = match source {
                Source::File(path) => PartSource::File(path.to_path_buf()),
                Source::Memory(bytes) => {
                    let start = usize::try_from(position).unwrap_or(0);
                    let end = start + usize::try_from(length).unwrap_or(0);
                    PartSource::Memory(bytes.clone(), start..end)
                }
            };
            let (file_position, part_length) = match source {
                Source::File(_) => (position, length),
                Source::Memory(_) => (0, -1),
            };
            let client = self.clone();
            let (bucket, key, upload_id) = (
                bucket_name.to_string(),
                key.to_string(),
                upload_id.to_string(),
            );
            let semaphore = semaphore.clone();
            let failed = failed.clone();
            tasks.spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("세마포어는 닫지 않는다");
                if failed.load(Ordering::SeqCst) {
                    return Ok(None);
                }
                let body = match part {
                    PartSource::File(path) => PutBody::File(path),
                    PartSource::Memory(bytes, range) => PutBody::Bytes(bytes[range].to_vec()),
                };
                let response = client
                    .upload_part(
                        &bucket,
                        &key,
                        &upload_id,
                        number,
                        body,
                        file_position,
                        part_length,
                        true,
                    )
                    .await
                    .inspect_err(|_| failed.store(true, Ordering::SeqCst))?;
                Ok::<_, S3Error>(Some(PartETag::new(number, response.output.e_tag())))
            });
            position += length;
        }

        // .NET `TransferUtility`는 파트 하나가 실패해도 이미 보낸 파트가 끝날 때까지 기다린 뒤(`Task.WhenAll`)
        // 업로드를 중단한다. 첫 오류를 기억해 두고 남은 작업을 모두 끝낸다.
        let mut parts = Vec::new();
        let mut first_error = None;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(Some(part))) => parts.push(part),
                Ok(Ok(None)) => {}
                Ok(Err(error)) => {
                    first_error.get_or_insert(error);
                }
                Err(error) => {
                    first_error.get_or_insert(S3Error::Request(error.to_string()));
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        parts.sort_by_key(|p| p.part_number);
        Ok(parts)
    }

    /// 원본 `Download(bucketName, key, filePath, versionId = null)`.
    pub async fn download(
        &self,
        bucket_name: &str,
        key: &str,
        file_path: &Path,
        version_id: Option<&str>,
    ) -> Result<(), S3Error> {
        let io = |e: std::io::Error| S3Error::io(file_path, &e);
        let response = self.get_object(bucket_name, key, version_id, None).await?;
        let mut output = response.output;

        let verify = expected_md5(
            output.e_tag(),
            output.server_side_encryption(),
            output.sse_customer_algorithm(),
        );
        if let Some(parent) = file_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(parent).await.map_err(io)?;
        }
        let mut file = tokio::fs::File::create(file_path).await.map_err(io)?;
        let mut hasher = Md5::new();
        while let Some(chunk) = output
            .body
            .try_next()
            .await
            .map_err(|e| S3Error::Network(e.to_string()))?
        {
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(io)?;
        }
        file.flush().await.map_err(io)?;

        if verify.is_some_and(|expected| hex::encode(hasher.finalize()) != expected) {
            return Err(S3Error::Request(
                "Expected hash not equal to calculated hash".to_string(),
            ));
        }
        Ok(())
    }
}

/// 내용의 MD5와 비교할 ETag(따옴표 제거, 소문자). 멀티파트·KMS·고객 키 암호화 객체는 비교하지 않는다.
fn expected_md5(
    e_tag: Option<&str>,
    sse: Option<&ServerSideEncryption>,
    customer_algorithm: Option<&str>,
) -> Option<String> {
    if customer_algorithm.is_some()
        || matches!(
            sse,
            Some(ServerSideEncryption::AwsKms | ServerSideEncryption::AwsKmsDsse)
        )
    {
        return None;
    }
    let value = e_tag?.trim_matches('"').to_ascii_lowercase();
    (value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())).then_some(value)
}

/// 실제 파트 크기. .NET은 파트 수가 10000을 넘지 않도록 `ceil(크기 / 10000)`보다 작게 나누지 않는다.
fn effective_part_size(size: i64, part_size: i64) -> i64 {
    part_size.max((size + MAX_PARTS - 1) / MAX_PARTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_size_grows_to_keep_10000_parts() {
        // .NET 캡처: 크기 100010, partSize 10 -> 파트 11바이트 9091개 + 9바이트 1개(9092개).
        let part = effective_part_size(100_010, 10);
        assert_eq!(part, 11);
        assert_eq!((100_010 + part - 1) / part, 9092);
        assert_eq!(100_010 - 9091 * part, 9);
        // 10000개 이하면 그대로.
        assert_eq!(effective_part_size(100_000, 10), 10);
        assert_eq!(effective_part_size(25, 10), 10);
    }

    #[test]
    fn md5_check_applies_to_plain_etags_only() {
        let md5 = "5eb63bbbe01eeed093cb22bb8f5acdc3";
        assert_eq!(
            expected_md5(Some(&format!("\"{md5}\"")), None, None).as_deref(),
            Some(md5)
        );
        assert_eq!(expected_md5(Some("\"abc-3\""), None, None), None);
        assert_eq!(
            expected_md5(Some(md5), Some(&ServerSideEncryption::AwsKms), None),
            None
        );
        assert_eq!(expected_md5(Some(md5), None, Some("AES256")), None);
        assert_eq!(expected_md5(None, None, None), None);
    }
}
