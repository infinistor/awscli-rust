//! 원본 `#region Object Function` 중 `object.rs`에 없는 연산(복사, ACL, 태그, 보존, 법적 보존, 복제, 복원, 버전 목록).
//!
//! `GetObjectAttributes`는 사용처가 없어 옮기지 않았다.

use aws_sdk_s3::operation::copy_object::CopyObjectOutput;
use aws_sdk_s3::operation::delete_bucket_replication::DeleteBucketReplicationOutput;
use aws_sdk_s3::operation::delete_object_tagging::DeleteObjectTaggingOutput;
use aws_sdk_s3::operation::get_bucket_replication::GetBucketReplicationOutput;
use aws_sdk_s3::operation::get_object_acl::GetObjectAclOutput;
use aws_sdk_s3::operation::get_object_legal_hold::GetObjectLegalHoldOutput;
use aws_sdk_s3::operation::get_object_retention::GetObjectRetentionOutput;
use aws_sdk_s3::operation::get_object_tagging::GetObjectTaggingOutput;
use aws_sdk_s3::operation::list_object_versions::ListObjectVersionsOutput;
use aws_sdk_s3::operation::put_bucket_replication::PutBucketReplicationOutput;
use aws_sdk_s3::operation::put_object_acl::PutObjectAclOutput;
use aws_sdk_s3::operation::put_object_legal_hold::PutObjectLegalHoldOutput;
use aws_sdk_s3::operation::put_object_retention::PutObjectRetentionOutput;
use aws_sdk_s3::operation::put_object_tagging::PutObjectTaggingOutput;
use aws_sdk_s3::operation::restore_object::RestoreObjectOutput;
use aws_sdk_s3::types::{
    AccessControlPolicy, MetadataDirective, ObjectCannedAcl, ObjectLockLegalHold,
    ObjectLockRetention, ReplicationConfiguration, RestoreRequest, Tagging,
};

use super::{S3Client, S3Error, S3Response, strip_unset, zero_content_length};

/// `x-amz-copy-source` 값. .NET은 `{버킷}/{키}` 전체를 RFC 3986으로 인코딩(`/`도 `%2F`)하고,
/// 버전 ID는 `?versionId=`에 `/`, `+`를 남기고 인코딩한다.
pub(crate) fn copy_source(bucket: &str, key: &str, version_id: Option<&str>) -> String {
    let mut source = encode(&format!("{bucket}/{key}"), "");
    if let Some(version) = version_id {
        source.push_str("?versionId=");
        source.push_str(&encode(version, "/+"));
    }
    source
}

fn encode(value: &str, keep: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'_' | b'.' | b'~')
            || keep.as_bytes().contains(&b)
        {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl S3Client {
    /// 원본 `PutObjectAcl(bucketName, key, S3CannedACL acl = null, S3AccessControlList accessControlPolicy = null)`.
    pub async fn put_object_acl(
        &self,
        bucket_name: &str,
        key: &str,
        acl: Option<ObjectCannedAcl>,
        access_control_policy: Option<AccessControlPolicy>,
    ) -> Result<S3Response<PutObjectAclOutput>, S3Error> {
        send!(
            self.client
                .put_object_acl()
                .bucket(bucket_name)
                .key(key)
                .set_acl(acl)
                .set_access_control_policy(access_control_policy),
            mutate = strip_unset
        )
    }

    /// 원본 `GetObjectAcl(bucketName, key, versionId = null)`.
    pub async fn get_object_acl(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<S3Response<GetObjectAclOutput>, S3Error> {
        send!(
            self.client
                .get_object_acl()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
        )
    }

    /// 원본 `CopyObject(sourceBucket, sourceKey, destinationBucket, destinationKey, versionId = null)`.
    pub async fn copy_object(
        &self,
        source_bucket: &str,
        source_key: &str,
        destination_bucket: &str,
        destination_key: &str,
        version_id: Option<&str>,
    ) -> Result<S3Response<CopyObjectOutput>, S3Error> {
        send!(
            self.client
                .copy_object()
                .bucket(destination_bucket)
                .key(destination_key)
                .copy_source(copy_source(source_bucket, source_key, version_id))
                // .NET은 `x-amz-metadata-directive: COPY`를 항상 보낸다.
                .metadata_directive(MetadataDirective::Copy),
            mutate = zero_content_length
        )
    }

    /// 원본 `ListVersions(bucketName, prefix = null, nextKeyMarker = null, nextVersionIdMarker = null, maxKeys = 1000, delimiter = null)`.
    pub async fn list_versions(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
        next_key_marker: Option<&str>,
        next_version_id_marker: Option<&str>,
        max_keys: i32,
        delimiter: Option<&str>,
    ) -> Result<S3Response<ListObjectVersionsOutput>, S3Error> {
        send!(
            self.client
                .list_object_versions()
                .bucket(bucket_name)
                .max_keys(max_keys)
                .set_key_marker(next_key_marker.map(str::to_string))
                .set_version_id_marker(next_version_id_marker.map(str::to_string))
                .set_prefix(prefix.map(str::to_string))
                .set_delimiter(delimiter.map(str::to_string))
        )
    }

    /// 원본 `GetObjectTagging(bucketName, key, versionId = null)`.
    pub async fn get_object_tagging(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<S3Response<GetObjectTaggingOutput>, S3Error> {
        send!(
            self.client
                .get_object_tagging()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
        )
    }

    /// 원본 `PutObjectTagging(bucketName, key, Tagging tagging)`.
    pub async fn put_object_tagging(
        &self,
        bucket_name: &str,
        key: &str,
        tagging: Tagging,
    ) -> Result<S3Response<PutObjectTaggingOutput>, S3Error> {
        send!(
            self.client
                .put_object_tagging()
                .bucket(bucket_name)
                .key(key)
                .tagging(tagging),
            mutate = strip_unset
        )
    }

    /// 원본 `DeleteObjectTagging(bucketName, key)`.
    pub async fn delete_object_tagging(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<S3Response<DeleteObjectTaggingOutput>, S3Error> {
        send!(
            self.client
                .delete_object_tagging()
                .bucket(bucket_name)
                .key(key)
        )
    }

    /// 원본 `GetObjectRetention(bucketName, key, versionId = null)`.
    pub async fn get_object_retention(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<S3Response<GetObjectRetentionOutput>, S3Error> {
        send!(
            self.client
                .get_object_retention()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
        )
    }

    /// 원본 `PutObjectRetention(bucketName, key, retention, contentMD5 = null, versionId = null, bool? bypass = false)`.
    /// `bypass`가 `true`일 때만 헤더를 보낸다.
    pub async fn put_object_retention(
        &self,
        bucket_name: &str,
        key: &str,
        retention: ObjectLockRetention,
        content_md5: Option<&str>,
        version_id: Option<&str>,
        bypass: Option<bool>,
    ) -> Result<S3Response<PutObjectRetentionOutput>, S3Error> {
        send!(
            self.client
                .put_object_retention()
                .bucket(bucket_name)
                .key(key)
                .retention(retention)
                .set_content_md5(content_md5.map(str::to_string))
                .set_version_id(version_id.map(str::to_string))
                .set_bypass_governance_retention((bypass == Some(true)).then_some(true))
        )
    }

    /// 원본 `PutObjectLegalHold(bucketName, key, ObjectLockLegalHold legalHold, versionId = null)`.
    pub async fn put_object_legal_hold(
        &self,
        bucket_name: &str,
        key: &str,
        legal_hold: ObjectLockLegalHold,
        version_id: Option<&str>,
    ) -> Result<S3Response<PutObjectLegalHoldOutput>, S3Error> {
        send!(
            self.client
                .put_object_legal_hold()
                .bucket(bucket_name)
                .key(key)
                .legal_hold(legal_hold)
                .set_version_id(version_id.map(str::to_string))
        )
    }

    /// 원본 `GetObjectLegalHold(bucketName, key, versionId = null)`.
    pub async fn get_object_legal_hold(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<S3Response<GetObjectLegalHoldOutput>, S3Error> {
        send!(
            self.client
                .get_object_legal_hold()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
        )
    }

    /// 원본 `GetBucketReplication(string bucketName)`.
    pub async fn get_bucket_replication(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketReplicationOutput>, S3Error> {
        send!(self.client.get_bucket_replication().bucket(bucket_name))
    }

    /// 원본 `PutBucketReplication(string bucketName, ReplicationConfiguration replicationConfig, string token = null)`.
    /// 원본과 같이 `token`은 요청에 넣지 않는다.
    pub async fn put_bucket_replication(
        &self,
        bucket_name: &str,
        replication_config: ReplicationConfiguration,
        token: Option<&str>,
    ) -> Result<S3Response<PutBucketReplicationOutput>, S3Error> {
        let _ = token;
        send!(
            self.client
                .put_bucket_replication()
                .bucket(bucket_name)
                .replication_configuration(replication_config),
            mutate = strip_unset
        )
    }

    /// 원본 `DeleteBucketReplication(string bucketName)`.
    pub async fn delete_bucket_replication(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketReplicationOutput>, S3Error> {
        send!(self.client.delete_bucket_replication().bucket(bucket_name))
    }

    /// 원본 `RestoreObject(bucketName, key, versionId = null, int days = -1)`. `days`는 0보다 클 때만 보낸다.
    pub async fn restore_object(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
        days: i32,
    ) -> Result<S3Response<RestoreObjectOutput>, S3Error> {
        let request = RestoreRequest::builder()
            .set_days((days > 0).then_some(days))
            .build();
        send!(
            self.client
                .restore_object()
                .bucket(bucket_name)
                .key(key)
                .set_version_id(version_id.map(str::to_string))
                .restore_request(request)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::copy_source;

    #[test]
    fn copy_source_matches_dotnet_encoding() {
        assert_eq!(
            copy_source("src-bucket", "src/key 1.txt", Some("v1")),
            "src-bucket%2Fsrc%2Fkey%201.txt?versionId=v1"
        );
        // 오라클 캡처(`copy-object-special`): 버전 ID의 `/`, `+`는 그대로 둔다.
        assert_eq!(
            copy_source("src-bucket", "src/한글 a+b(1)*'!~_-.txt", Some("v 1/+")),
            "src-bucket%2Fsrc%2F%ED%95%9C%EA%B8%80%20a%2Bb%281%29%2A%27%21~_-.txt?versionId=v%201/+"
        );
        assert_eq!(copy_source("b", "k", None), "b%2Fk");
    }
}
