//! 원본 `#region Bucket Function`, `#region S3Util`.

use aws_sdk_s3::operation::create_bucket::CreateBucketOutput;
use aws_sdk_s3::operation::get_bucket_versioning::GetBucketVersioningOutput;
use aws_sdk_s3::operation::list_buckets::ListBucketsOutput;
use aws_sdk_s3::operation::put_bucket_versioning::PutBucketVersioningOutput;
use aws_sdk_s3::types::{
    BucketCannedAcl, BucketVersioningStatus, ObjectOwnership, VersioningConfiguration,
};
use tracing::{error, info};

use super::{S3Client, S3Error, S3Response};

impl S3Client {
    /// 원본 `ListBuckets(string prefix = null, int maxBuckets = 10000, string continuationToken = null)`.
    pub async fn list_buckets(
        &self,
        prefix: Option<&str>,
        max_buckets: i32,
        continuation_token: Option<&str>,
    ) -> Result<S3Response<ListBucketsOutput>, S3Error> {
        send!(
            self.client
                .list_buckets()
                .set_prefix(prefix.map(str::to_string))
                .max_buckets(max_buckets)
                .set_continuation_token(continuation_token.map(str::to_string)),
            empty_body = "ListAllMyBucketsResult"
        )
    }

    /// 원본 `PutBucket(string bucketName, S3CannedACL acl = null, bool? objectLockEnabledForBucket = null, ObjectOwnership ownership = null)`.
    pub async fn put_bucket(
        &self,
        bucket_name: &str,
        acl: Option<BucketCannedAcl>,
        object_lock_enabled_for_bucket: Option<bool>,
        ownership: Option<ObjectOwnership>,
    ) -> Result<S3Response<CreateBucketOutput>, S3Error> {
        send!(
            self.client
                .create_bucket()
                .bucket(bucket_name)
                .set_acl(acl)
                .set_object_lock_enabled_for_bucket(object_lock_enabled_for_bucket)
                .set_object_ownership(ownership)
        )
    }

    /// 원본 `PutBucketVersioning(string bucketName, VersionStatus status = null)`.
    pub async fn put_bucket_versioning(
        &self,
        bucket_name: &str,
        status: Option<BucketVersioningStatus>,
    ) -> Result<S3Response<PutBucketVersioningOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_versioning()
                .bucket(bucket_name)
                .versioning_configuration(
                    VersioningConfiguration::builder()
                        .set_status(status)
                        .build()
                )
        )
    }

    /// 원본 `GetBucketVersioning(string bucketName)`.
    pub async fn get_bucket_versioning(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketVersioningOutput>, S3Error> {
        send!(self.client.get_bucket_versioning().bucket(bucket_name))
    }

    /// 원본 `DoesS3BucketExist`: 오류는 모두 `false`.
    ///
    /// 원본 `AmazonS3Util.DoesS3BucketExistV2Async`는 버킷 ACL 조회(`GET /{bucket}?acl`)로 확인하고,
    /// `NoSuchBucket`이면 `false`, 그 밖의 서비스 오류(예: 접근 거부)는 버킷이 있는 것으로 본다.
    pub async fn does_s3_bucket_exist(&self, bucket_name: &str) -> bool {
        match self
            .client
            .get_bucket_acl()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(_) => true,
            Err(e) => match S3Error::from(e) {
                S3Error::Service { code, .. } => code != "NoSuchBucket",
                _ => false,
            },
        }
    }

    /// 원본 `CreateBucket(string bucketName)`: 없을 때만 만들고 결과를 로그로 남긴다.
    pub async fn create_bucket(&self, bucket_name: &str) -> bool {
        if self.does_s3_bucket_exist(bucket_name).await {
            return false;
        }
        match self.put_bucket(bucket_name, None, None, None).await {
            Ok(response) if response.status == 200 => {
                info!("CreateBucket({bucket_name}) : Create!!");
                true
            }
            Ok(_) => {
                error!("CreateBucket({bucket_name}) : Create failed");
                false
            }
            // 원본의 `catch (AggregateException)`은 `GetAwaiter().GetResult()`가 내부 예외를 그대로 던지므로
            // 실행되지 않고, `catch (Exception e) { log.Error(e); }`로 간다.
            Err(e) => {
                error!("{}: {e}", e.dotnet_type());
                false
            }
        }
    }
}
