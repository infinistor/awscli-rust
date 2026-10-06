//! 원본 `#region Bucket Function` 중 버킷 설정 연산(ACL, CORS, 태그, 수명 주기, 정책, 암호화, 웹사이트,
//! 로깅, 알림, 소유권, 퍼블릭 액세스 차단, 객체 잠금, 인벤토리, 메트릭, 분석, 위치).
//!
//! 설정 객체는 원본이 받던 AWSSDK 모델 대신 `aws-sdk-s3`의 같은 이름 모델을 받는다. 인자 순서와 이름은 원본을 따른다.

use aws_sdk_s3::operation::delete_bucket::DeleteBucketOutput;
use aws_sdk_s3::operation::delete_bucket_analytics_configuration::DeleteBucketAnalyticsConfigurationOutput;
use aws_sdk_s3::operation::delete_bucket_cors::DeleteBucketCorsOutput;
use aws_sdk_s3::operation::delete_bucket_encryption::DeleteBucketEncryptionOutput;
use aws_sdk_s3::operation::delete_bucket_inventory_configuration::DeleteBucketInventoryConfigurationOutput;
use aws_sdk_s3::operation::delete_bucket_lifecycle::DeleteBucketLifecycleOutput;
use aws_sdk_s3::operation::delete_bucket_metrics_configuration::DeleteBucketMetricsConfigurationOutput;
use aws_sdk_s3::operation::delete_bucket_ownership_controls::DeleteBucketOwnershipControlsOutput;
use aws_sdk_s3::operation::delete_bucket_policy::DeleteBucketPolicyOutput;
use aws_sdk_s3::operation::delete_bucket_tagging::DeleteBucketTaggingOutput;
use aws_sdk_s3::operation::delete_bucket_website::DeleteBucketWebsiteOutput;
use aws_sdk_s3::operation::delete_public_access_block::DeletePublicAccessBlockOutput;
use aws_sdk_s3::operation::get_bucket_acl::GetBucketAclOutput;
use aws_sdk_s3::operation::get_bucket_analytics_configuration::GetBucketAnalyticsConfigurationOutput;
use aws_sdk_s3::operation::get_bucket_cors::GetBucketCorsOutput;
use aws_sdk_s3::operation::get_bucket_encryption::GetBucketEncryptionOutput;
use aws_sdk_s3::operation::get_bucket_inventory_configuration::GetBucketInventoryConfigurationOutput;
use aws_sdk_s3::operation::get_bucket_lifecycle_configuration::GetBucketLifecycleConfigurationOutput;
use aws_sdk_s3::operation::get_bucket_location::GetBucketLocationOutput;
use aws_sdk_s3::operation::get_bucket_logging::GetBucketLoggingOutput;
use aws_sdk_s3::operation::get_bucket_metrics_configuration::GetBucketMetricsConfigurationOutput;
use aws_sdk_s3::operation::get_bucket_notification_configuration::GetBucketNotificationConfigurationOutput;
use aws_sdk_s3::operation::get_bucket_ownership_controls::GetBucketOwnershipControlsOutput;
use aws_sdk_s3::operation::get_bucket_policy::GetBucketPolicyOutput;
use aws_sdk_s3::operation::get_bucket_policy_status::GetBucketPolicyStatusOutput;
use aws_sdk_s3::operation::get_bucket_tagging::GetBucketTaggingOutput;
use aws_sdk_s3::operation::get_bucket_website::GetBucketWebsiteOutput;
use aws_sdk_s3::operation::get_object_lock_configuration::GetObjectLockConfigurationOutput;
use aws_sdk_s3::operation::get_public_access_block::GetPublicAccessBlockOutput;
use aws_sdk_s3::operation::list_bucket_analytics_configurations::ListBucketAnalyticsConfigurationsOutput;
use aws_sdk_s3::operation::list_bucket_inventory_configurations::ListBucketInventoryConfigurationsOutput;
use aws_sdk_s3::operation::list_bucket_metrics_configurations::ListBucketMetricsConfigurationsOutput;
use aws_sdk_s3::operation::list_directory_buckets::ListDirectoryBucketsOutput;
use aws_sdk_s3::operation::put_bucket_acl::PutBucketAclOutput;
use aws_sdk_s3::operation::put_bucket_analytics_configuration::PutBucketAnalyticsConfigurationOutput;
use aws_sdk_s3::operation::put_bucket_cors::PutBucketCorsOutput;
use aws_sdk_s3::operation::put_bucket_encryption::PutBucketEncryptionOutput;
use aws_sdk_s3::operation::put_bucket_inventory_configuration::PutBucketInventoryConfigurationOutput;
use aws_sdk_s3::operation::put_bucket_lifecycle_configuration::PutBucketLifecycleConfigurationOutput;
use aws_sdk_s3::operation::put_bucket_logging::PutBucketLoggingOutput;
use aws_sdk_s3::operation::put_bucket_metrics_configuration::PutBucketMetricsConfigurationOutput;
use aws_sdk_s3::operation::put_bucket_notification_configuration::PutBucketNotificationConfigurationOutput;
use aws_sdk_s3::operation::put_bucket_ownership_controls::PutBucketOwnershipControlsOutput;
use aws_sdk_s3::operation::put_bucket_policy::PutBucketPolicyOutput;
use aws_sdk_s3::operation::put_bucket_tagging::PutBucketTaggingOutput;
use aws_sdk_s3::operation::put_bucket_website::PutBucketWebsiteOutput;
use aws_sdk_s3::operation::put_object_lock_configuration::PutObjectLockConfigurationOutput;
use aws_sdk_s3::operation::put_public_access_block::PutPublicAccessBlockOutput;
use aws_sdk_s3::types::{
    AccessControlPolicy, AnalyticsConfiguration, BucketCannedAcl, BucketLifecycleConfiguration,
    BucketLoggingStatus, CorsConfiguration, InventoryConfiguration, LambdaFunctionConfiguration,
    MetricsConfiguration, NotificationConfiguration, ObjectLockConfiguration, ObjectOwnership,
    OwnershipControls, OwnershipControlsRule, PublicAccessBlockConfiguration, QueueConfiguration,
    ServerSideEncryptionConfiguration, Tag, Tagging, TopicConfiguration, WebsiteConfiguration,
};

use super::{S3Client, S3Error, S3Response, add_content_md5, add_crc32, built};

impl S3Client {
    /// 원본 `ListDirectoryBuckets(int maxKeys = 1000, string continuationToken = null)`.
    pub async fn list_directory_buckets(
        &self,
        max_keys: i32,
        continuation_token: Option<&str>,
    ) -> Result<S3Response<ListDirectoryBucketsOutput>, S3Error> {
        send!(
            self.client
                .list_directory_buckets()
                .max_directory_buckets(max_keys)
                .set_continuation_token(continuation_token.map(str::to_string)),
            // SDK는 이 연산에 S3 Express 세션 인증을 고르는데, 세션 인증은 버킷 이름이 있어야 해서 실패한다.
            // .NET은 일반 서명(SigV4)으로 `GET /`를 보내므로 이 요청만 세션 인증을 끈다.
            config = aws_sdk_s3::config::Builder::default().disable_s3_express_session_auth(true)
        )
    }

    /// 원본 `DeleteBucket(string bucketName)`.
    pub async fn delete_bucket(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketOutput>, S3Error> {
        send!(self.client.delete_bucket().bucket(bucket_name))
    }

    /// 원본 `GetBucketOwnershipControls(string bucketName)`.
    pub async fn get_bucket_ownership_controls(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketOwnershipControlsOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_ownership_controls()
                .bucket(bucket_name)
        )
    }

    /// 원본 `PutBucketOwnershipControls(string bucketName, ObjectOwnership ownership)`.
    pub async fn put_bucket_ownership_controls(
        &self,
        bucket_name: &str,
        ownership: ObjectOwnership,
    ) -> Result<S3Response<PutBucketOwnershipControlsOutput>, S3Error> {
        let controls = built(
            OwnershipControls::builder()
                .rules(built(
                    OwnershipControlsRule::builder()
                        .object_ownership(ownership)
                        .build(),
                )?)
                .build(),
        )?;
        send!(
            self.client
                .put_bucket_ownership_controls()
                .bucket(bucket_name)
                .ownership_controls(controls)
        )
    }

    /// 원본 `DeleteBucketOwnershipControls(string bucketName)`.
    pub async fn delete_bucket_ownership_controls(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketOwnershipControlsOutput>, S3Error> {
        send!(
            self.client
                .delete_bucket_ownership_controls()
                .bucket(bucket_name)
        )
    }

    /// 원본 `GetBucketLocation(string bucketName)`.
    pub async fn get_bucket_location(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketLocationOutput>, S3Error> {
        send!(self.client.get_bucket_location().bucket(bucket_name))
    }

    /// 원본 `PutBucketLogging(string bucketName, S3BucketLoggingConfig loggingConfig)`.
    pub async fn put_bucket_logging(
        &self,
        bucket_name: &str,
        logging_config: BucketLoggingStatus,
    ) -> Result<S3Response<PutBucketLoggingOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_logging()
                .bucket(bucket_name)
                .bucket_logging_status(logging_config)
        )
    }

    /// 원본 `GetBucketLogging(string bucketName)`.
    pub async fn get_bucket_logging(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketLoggingOutput>, S3Error> {
        send!(self.client.get_bucket_logging().bucket(bucket_name))
    }

    /// 원본 `PutBucketNotification(bucketName, topicConfigurations = null, queueConfigurations = null, lambdaFunctionConfigurations = null)`.
    /// `null`인 목록은 요청에 넣지 않는다.
    pub async fn put_bucket_notification(
        &self,
        bucket_name: &str,
        topic_configurations: Option<Vec<TopicConfiguration>>,
        queue_configurations: Option<Vec<QueueConfiguration>>,
        lambda_function_configurations: Option<Vec<LambdaFunctionConfiguration>>,
    ) -> Result<S3Response<PutBucketNotificationConfigurationOutput>, S3Error> {
        let configuration = NotificationConfiguration::builder()
            .set_topic_configurations(topic_configurations)
            .set_queue_configurations(queue_configurations)
            .set_lambda_function_configurations(lambda_function_configurations)
            .build();
        send!(
            self.client
                .put_bucket_notification_configuration()
                .bucket(bucket_name)
                .notification_configuration(configuration),
            mutate = add_crc32
        )
    }

    /// 원본 `GetBucketNotification(string bucketName)`.
    pub async fn get_bucket_notification(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketNotificationConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_notification_configuration()
                .bucket(bucket_name)
        )
    }

    /// 원본 `GetBucketAcl(string bucketName)`.
    pub async fn get_bucket_acl(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketAclOutput>, S3Error> {
        send!(self.client.get_bucket_acl().bucket(bucket_name))
    }

    /// 원본 `PutBucketAcl(string bucketName, S3CannedACL acl = null, S3AccessControlList accessControlPolicy = null)`.
    pub async fn put_bucket_acl(
        &self,
        bucket_name: &str,
        acl: Option<BucketCannedAcl>,
        access_control_policy: Option<AccessControlPolicy>,
    ) -> Result<S3Response<PutBucketAclOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_acl()
                .bucket(bucket_name)
                .set_acl(acl)
                .set_access_control_policy(access_control_policy)
        )
    }

    /// 원본 `PutCORS(string bucketName, CORSConfiguration configuration)`.
    pub async fn put_cors(
        &self,
        bucket_name: &str,
        configuration: CorsConfiguration,
    ) -> Result<S3Response<PutBucketCorsOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_cors()
                .bucket(bucket_name)
                .cors_configuration(configuration)
        )
    }

    /// 원본 `GetCORS(string bucketName)`.
    pub async fn get_cors(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketCorsOutput>, S3Error> {
        send!(self.client.get_bucket_cors().bucket(bucket_name))
    }

    /// 원본 `DeleteCORS(string bucketName)`.
    pub async fn delete_cors(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketCorsOutput>, S3Error> {
        send!(self.client.delete_bucket_cors().bucket(bucket_name))
    }

    /// 원본 `GetBucketTagging(string bucketName)`.
    pub async fn get_bucket_tagging(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketTaggingOutput>, S3Error> {
        send!(self.client.get_bucket_tagging().bucket(bucket_name))
    }

    /// 원본 `PutBucketTagging(string bucketName, List<Tag> TagSet)`.
    pub async fn put_bucket_tagging(
        &self,
        bucket_name: &str,
        tag_set: Vec<Tag>,
    ) -> Result<S3Response<PutBucketTaggingOutput>, S3Error> {
        let tagging = built(Tagging::builder().set_tag_set(Some(tag_set)).build())?;
        send!(
            self.client
                .put_bucket_tagging()
                .bucket(bucket_name)
                .tagging(tagging)
        )
    }

    /// 원본 `DeleteBucketTagging(string bucketName)`.
    pub async fn delete_bucket_tagging(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketTaggingOutput>, S3Error> {
        send!(self.client.delete_bucket_tagging().bucket(bucket_name))
    }

    /// 원본 `PutLifecycleConfiguration(string bucketName, LifecycleConfiguration lifecycleConfig)`.
    pub async fn put_lifecycle_configuration(
        &self,
        bucket_name: &str,
        lifecycle_config: BucketLifecycleConfiguration,
    ) -> Result<S3Response<PutBucketLifecycleConfigurationOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_lifecycle_configuration()
                .bucket(bucket_name)
                .lifecycle_configuration(lifecycle_config)
        )
    }

    /// 원본 `GetLifecycleConfiguration(string bucketName)`.
    pub async fn get_lifecycle_configuration(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketLifecycleConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_lifecycle_configuration()
                .bucket(bucket_name)
        )
    }

    /// 원본 `DeleteLifecycle(string bucketName)`.
    pub async fn delete_lifecycle(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketLifecycleOutput>, S3Error> {
        send!(self.client.delete_bucket_lifecycle().bucket(bucket_name))
    }

    /// 원본 `PutBucketPolicy(string bucketName, string policy)`.
    pub async fn put_bucket_policy(
        &self,
        bucket_name: &str,
        policy: &str,
    ) -> Result<S3Response<PutBucketPolicyOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_policy()
                .bucket(bucket_name)
                .policy(policy)
        )
    }

    /// 원본 `GetBucketPolicy(string bucketName)`.
    pub async fn get_bucket_policy(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketPolicyOutput>, S3Error> {
        send!(self.client.get_bucket_policy().bucket(bucket_name))
    }

    /// 원본 `DeleteBucketPolicy(string bucketName)`.
    pub async fn delete_bucket_policy(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketPolicyOutput>, S3Error> {
        send!(self.client.delete_bucket_policy().bucket(bucket_name))
    }

    /// 원본 `GetBucketPolicyStatus(string bucketName)`.
    pub async fn get_bucket_policy_status(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketPolicyStatusOutput>, S3Error> {
        send!(self.client.get_bucket_policy_status().bucket(bucket_name))
    }

    /// 원본 `PutObjectLockConfiguration(string bucketName, ObjectLockConfiguration lockConfig)`.
    pub async fn put_object_lock_configuration(
        &self,
        bucket_name: &str,
        lock_config: ObjectLockConfiguration,
    ) -> Result<S3Response<PutObjectLockConfigurationOutput>, S3Error> {
        send!(
            self.client
                .put_object_lock_configuration()
                .bucket(bucket_name)
                .object_lock_configuration(lock_config)
        )
    }

    /// 원본 `GetObjectLockConfiguration(string bucketName)`.
    pub async fn get_object_lock_configuration(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetObjectLockConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_object_lock_configuration()
                .bucket(bucket_name)
        )
    }

    /// 원본 `PutPublicAccessBlock(string bucketName, PublicAccessBlockConfiguration publicAccessBlockConfig)`.
    pub async fn put_public_access_block(
        &self,
        bucket_name: &str,
        public_access_block_config: PublicAccessBlockConfiguration,
    ) -> Result<S3Response<PutPublicAccessBlockOutput>, S3Error> {
        send!(
            self.client
                .put_public_access_block()
                .bucket(bucket_name)
                .public_access_block_configuration(public_access_block_config)
        )
    }

    /// 원본 `GetPublicAccessBlock(string bucketName)`.
    pub async fn get_public_access_block(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetPublicAccessBlockOutput>, S3Error> {
        send!(self.client.get_public_access_block().bucket(bucket_name))
    }

    /// 원본 `DeletePublicAccessBlock(string bucketName)`.
    pub async fn delete_public_access_block(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeletePublicAccessBlockOutput>, S3Error> {
        send!(self.client.delete_public_access_block().bucket(bucket_name))
    }

    /// 원본 `GetBucketEncryption(string bucketName)`.
    pub async fn get_bucket_encryption(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketEncryptionOutput>, S3Error> {
        send!(self.client.get_bucket_encryption().bucket(bucket_name))
    }

    /// 원본 `PutBucketEncryption(string bucketName, ServerSideEncryptionConfiguration sseConfig)`.
    pub async fn put_bucket_encryption(
        &self,
        bucket_name: &str,
        sse_config: ServerSideEncryptionConfiguration,
    ) -> Result<S3Response<PutBucketEncryptionOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_encryption()
                .bucket(bucket_name)
                .server_side_encryption_configuration(sse_config)
        )
    }

    /// 원본 `DeleteBucketEncryption(string bucketName)`.
    pub async fn delete_bucket_encryption(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketEncryptionOutput>, S3Error> {
        send!(self.client.delete_bucket_encryption().bucket(bucket_name))
    }

    /// 원본 `GetBucketWebsite(string bucketName)`.
    pub async fn get_bucket_website(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<GetBucketWebsiteOutput>, S3Error> {
        send!(self.client.get_bucket_website().bucket(bucket_name))
    }

    /// 원본 `PutBucketWebsite(string bucketName, WebsiteConfiguration webConfig)`.
    pub async fn put_bucket_website(
        &self,
        bucket_name: &str,
        web_config: WebsiteConfiguration,
    ) -> Result<S3Response<PutBucketWebsiteOutput>, S3Error> {
        send!(
            self.client
                .put_bucket_website()
                .bucket(bucket_name)
                .website_configuration(web_config)
        )
    }

    /// 원본 `DeleteBucketWebsite(string bucketName)`.
    pub async fn delete_bucket_website(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<DeleteBucketWebsiteOutput>, S3Error> {
        send!(self.client.delete_bucket_website().bucket(bucket_name))
    }

    /// 원본 `GetBucketInventory(string bucketName, string Id)`.
    pub async fn get_bucket_inventory(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<GetBucketInventoryConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_inventory_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `ListBucketInventory(string bucketName)`.
    pub async fn list_bucket_inventory(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<ListBucketInventoryConfigurationsOutput>, S3Error> {
        send!(
            self.client
                .list_bucket_inventory_configurations()
                .bucket(bucket_name)
        )
    }

    /// 원본 `PutBucketInventory(string bucketName, InventoryConfiguration inventoryConfig)`: 아이디는 설정 객체에서 가져온다.
    /// .NET SDK는 이 요청에 `Content-MD5`를 붙이므로 같게 맞춘다.
    pub async fn put_bucket_inventory(
        &self,
        bucket_name: &str,
        inventory_config: InventoryConfiguration,
    ) -> Result<S3Response<PutBucketInventoryConfigurationOutput>, S3Error> {
        let id = inventory_config.id().to_string();
        send!(
            self.client
                .put_bucket_inventory_configuration()
                .bucket(bucket_name)
                .id(id)
                .inventory_configuration(inventory_config),
            mutate = add_content_md5
        )
    }

    /// 원본 `DeleteBucketInventory(string bucketName, string Id)`.
    pub async fn delete_bucket_inventory(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<DeleteBucketInventoryConfigurationOutput>, S3Error> {
        send!(
            self.client
                .delete_bucket_inventory_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `GetBucketMetrics(string bucketName, string Id)`.
    pub async fn get_bucket_metrics(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<GetBucketMetricsConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_metrics_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `ListBucketMetrics(string bucketName)`.
    pub async fn list_bucket_metrics(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<ListBucketMetricsConfigurationsOutput>, S3Error> {
        send!(
            self.client
                .list_bucket_metrics_configurations()
                .bucket(bucket_name)
        )
    }

    /// 원본 `PutBucketMetrics(string bucketName, MetricsConfiguration metricsConfig)`: 아이디는 설정 객체에서 가져온다.
    pub async fn put_bucket_metrics(
        &self,
        bucket_name: &str,
        metrics_config: MetricsConfiguration,
    ) -> Result<S3Response<PutBucketMetricsConfigurationOutput>, S3Error> {
        let id = metrics_config.id().to_string();
        send!(
            self.client
                .put_bucket_metrics_configuration()
                .bucket(bucket_name)
                .id(id)
                .metrics_configuration(metrics_config),
            mutate = add_content_md5
        )
    }

    /// 원본 `DeleteBucketMetrics(string bucketName, string Id)`.
    pub async fn delete_bucket_metrics(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<DeleteBucketMetricsConfigurationOutput>, S3Error> {
        send!(
            self.client
                .delete_bucket_metrics_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `DeleteBucketAnalytics(string bucketName, string Id)`.
    pub async fn delete_bucket_analytics(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<DeleteBucketAnalyticsConfigurationOutput>, S3Error> {
        send!(
            self.client
                .delete_bucket_analytics_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `GetBucketAnalytics(string bucketName, string id)`.
    pub async fn get_bucket_analytics(
        &self,
        bucket_name: &str,
        id: &str,
    ) -> Result<S3Response<GetBucketAnalyticsConfigurationOutput>, S3Error> {
        send!(
            self.client
                .get_bucket_analytics_configuration()
                .bucket(bucket_name)
                .id(id)
        )
    }

    /// 원본 `ListBucketAnalytics(string bucketName)`.
    pub async fn list_bucket_analytics(
        &self,
        bucket_name: &str,
    ) -> Result<S3Response<ListBucketAnalyticsConfigurationsOutput>, S3Error> {
        send!(
            self.client
                .list_bucket_analytics_configurations()
                .bucket(bucket_name)
        )
    }

    /// 원본 `PutBucketAnalytics(string bucketName, AnalyticsConfiguration analyticsConfig)`: 아이디는 설정 객체에서 가져온다.
    pub async fn put_bucket_analytics(
        &self,
        bucket_name: &str,
        analytics_config: AnalyticsConfiguration,
    ) -> Result<S3Response<PutBucketAnalyticsConfigurationOutput>, S3Error> {
        let id = analytics_config.id().to_string();
        send!(
            self.client
                .put_bucket_analytics_configuration()
                .bucket(bucket_name)
                .id(id)
                .analytics_configuration(analytics_config),
            mutate = add_content_md5
        )
    }
}
