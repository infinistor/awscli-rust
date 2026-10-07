//! `Test/UpDownTest.cs`: 오브젝트 업·다운로드 부하 테스트(스레드마다 `UpDownClient` 하나).
//!
//! 원본은 시나리오 메서드마다 같은 틀을 되풀이한다: 시작 시각 기록 → 버킷·더미 파일 준비 → 스레드마다 클라이언트와
//! 작업 생성 → 시작 → 2초마다 진행 상황 출력 → 최종 결과 출력과 JSON 저장. 틀은 [`UpDownTest::watch`]와
//! [`UpDownTest::complete_final_result`]로 모으고 시나리오마다 다른 부분만 각 메서드에 둔다.
//!
//! 종료 처리: 원본은 `Console.CancelKeyPress`와 `ProcessExit` 처리기가 활성 테스트를 멈추고 최종 결과를 남긴다.
//! 여기서는 테스트마다 토큰을 두고 클라이언트의 `Quit`을 그 자식에 묶는다. 작업을 시작할 때 토큰을 활성 목록에
//! 올리고(`RegisterActiveTest`) `FinalizeTasks`에서 내린다. Ctrl+C는 활성 테스트의 토큰만 취소하므로 모든
//! 클라이언트가 `Quit`이 되어 감시 루프가 끝나고, 주 흐름이 최종 출력과 JSON 저장을 한 번만 한다(처리기 스레드와의
//! 경합은 재현하지 않는다, [`crate::shutdown`]). `FullTest`처럼 다음 테스트를 이어 가면 새 테스트는 새 토큰이라
//! 원본처럼 정상 실행된다.
//!
//! 분산 실행(`RunControl`, 원본의 `_control != null` 분기)은 6단계에서 옮긴다. 이 모듈은 `_control == null`만 다룬다.
//! 분산 분기가 들어갈 자리는 `6단계` 주석으로 표시했다. 그 분기는 `TimeWatcher.Start()`를 이미 실행 중인
//! 스톱워치에 다시 부르는데(.NET은 아무 일도 안 한다) Rust의 `TimeWatcher::start`는 시작 시각을 다시 잡는다.
//! 여기서는 시작을 한 번만 하므로 차이가 없다.
//!
//! 원본 특이점(그대로 둔다)
//!
//! - JSON의 `Time`은 경과 시간이 아니라 설정의 `Times`(`TimeWatcher.EndTime`)다. 개수로 끝나는 시나리오도 마찬가지다.
//! - `StartTime`은 시나리오 메서드에 들어간 시각이다(버킷·더미 파일 준비 시간 포함).
//! - 마지막 출력에 진행 상황 형식을 쓰는 시나리오: Upload·MultipartUpload(`PrintMultiUpload`),
//!   MultipartUploadV2(`PrintMultiUploadV2`), Download(`PrintDownload`), UploadTag(`PrintPrepare`), AWSTest(`PrintAWS`).
//! - `X Test End` 로그가 없는 시나리오: Write, Mix, MixV2, MixNew, PutGet, All, Upload, Download, UploadTag, AWSTest,
//!   MultipartUpload(V2), ListObjectTest.
//! - `HeadTest`는 시간 제한이 없다(`Head`가 끝나지 않고 감시 루프에도 `IsEnd` 검사가 없다). Ctrl+C로만 끝난다.
//! - `Head`는 더미 파일 이름 대신 `Main.FilePath`를 파일로 넘긴다.
//! - `AWSTest`는 `BucketType`을 무시하고 `Main.BucketName`을 쓴다(버킷 생성도 항상 한다).
//! - `DeleteOne`은 버킷 생성·버저닝(Enabled)·수명 주기 설정을 `try/catch` 없이 한다. 예외는 호출한 쪽으로 올라간다.
//! - `UploadTag`는 스레드 0·1만 태그를 붙여 올리고 나머지는 `Upload`(TransferUtility)를 한다.
//! - 스레드 안에서 처리하지 않은 예외는 프로세스를 끝내고 최종 출력과 JSON 저장은 하지 않는다([`crate::runner`]).
//! - `S3Client`의 `catch (AggregateException)`는 실행되지 않아 `catch (Exception e)`가 `형식: 메시지`를 로그에 남긴다.

mod save;

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use aws_sdk_s3::types::{
    BucketLifecycleConfiguration, BucketVersioningStatus, ExpirationStatus, LifecycleExpiration,
    LifecycleRule, NoncurrentVersionExpiration,
};
use awscli_rest_clients::file_util::create_random_file;
use awscli_rest_clients::up_down::{UpDownClient, UpDownError};
use awscli_rest_common::DotnetDateTime;
use awscli_rest_config::{EnumBucketTypes, MainConfig, UpDownClientConfig, UpDownConfig, UserData};
use awscli_rest_model::up_down_stats::log_info;
use awscli_rest_model::{TimeWatcher, UpDownStats};
use awscli_rest_s3::S3Client;
use rust_decimal::Decimal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::ScenarioError;
use crate::runner::{FinalResult, TestTasks, idle};
use crate::shutdown::{ActiveGuard, Handler, activate};
use crate::util::dummy_file_name;

/// 진행 상황·최종 결과 출력 형식(원본 `Print*`·`Print*Final`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Report {
    Prepare,
    Write,
    Head,
    Read,
    ReadV2,
    Delete,
    DeleteV2,
    MultiUpload,
    MultiUploadV2,
    Download,
    Mix,
    All,
    Aws,
    ListObject,
    PrepareFinal,
    WriteFinal,
    HeadFinal,
    ReadFinal,
    ReadV2Final,
    DeleteFinal,
    DeleteV2Final,
    MixFinal,
    AllFinal,
    ListObjectFinal,
}

/// 원본 `UpDownTest`.
pub struct UpDownTest {
    main_config: MainConfig,
    config: UpDownConfig,
    user: UserData,
    watcher: TimeWatcher,
    client: S3Client,
    tasks: TestTasks<UpDownClient>,
    client_config: UpDownClientConfig,
    stats: UpDownStats,
    /// 테스트 시작 시각(JSON `StartTime`).
    test_start_time: DotnetDateTime,
    finalized: bool,
    /// 원본 `_finalResult`: 마지막 출력 형식과 저장 이름.
    final_report: Option<(Report, String)>,
    final_result: FinalResult,
    /// 이 테스트의 토큰(프로세스 토큰의 자식). 클라이언트의 `Quit`이 이 토큰의 자식이고, Ctrl+C는 활성 테스트의
    /// 토큰만 취소한다.
    cancel: CancellationToken,
    /// 원본 `_activeTests` 등록(작업 시작부터 `FinalizeTasks`까지).
    active: Option<ActiveGuard>,
}

impl UpDownTest {
    /// 원본 `UpDownTest(mainConfig, config, user)`. `cancel`은 프로세스 전체 취소 토큰이다.
    pub fn new(
        main_config: &MainConfig,
        config: &UpDownConfig,
        user: &UserData,
        cancel: &CancellationToken,
    ) -> Self {
        let client_config = UpDownClientConfig::new(
            &main_config.thread_prefix,
            &main_config.object_prefix,
            config.read_ratio,
            config.write_ratio,
            config.delete_ratio,
            main_config.file_size,
            config.bucket_type,
            config.etag_check,
            config.division_count,
            main_config.retry_count,
            main_config.is_admin,
            config.use_chunk_encoding,
        );
        Self {
            main_config: main_config.clone(),
            config: config.clone(),
            user: user.clone(),
            watcher: TimeWatcher::new(config.times),
            client: S3Client::from_user(user, false, 3, false),
            tasks: TestTasks::new(),
            client_config,
            stats: UpDownStats::new(main_config.file_size),
            test_start_time: DotnetDateTime::now(),
            finalized: false,
            final_report: None,
            final_result: FinalResult::default(),
            cancel: cancel.child_token(),
            active: None,
        }
    }

    // ---- CosBench Like Test ----

    /// 순차적으로 파일을 올린다(기존 파일 건너뛰기 가능). `super_random`이면 매번 다른 파일을 올린다.
    pub async fn prepare(
        &mut self,
        check: bool,
        start: i32,
        super_random: bool,
    ) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Prepare Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            if super_random {
                self.add_work(client, move |c| async move {
                    c.prepare_random(count, start).await
                });
            } else {
                self.add_work(client, move |c| async move {
                    c.prepare(count, check, start).await
                });
            }
        }
        info!("Prepare Test Start");
        if self
            .watch(Report::Prepare, Report::PrepareFinal, "Prepare", false)
            .await
        {
            info!("Prepare Test End");
        }
        Ok(())
    }

    /// Prepare와 같으나 빈 본문과 키 끝 `/`로 폴더(디렉터리 마커)를 만든다.
    pub async fn prepare_dir(&mut self, check: bool, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Prepare Dir Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            // 폴더는 빈 본문이라 더미 파일이 필요 없다.
            let client = self.new_client(bucket, index, "");
            let count = self.config.file_count;
            self.add_work(client, move |c| async move {
                c.prepare_dir(count, check, start).await
            });
        }
        info!("Prepare Dir Test Start");
        if self
            .watch(Report::Prepare, Report::PrepareFinal, "PrepareDir", false)
            .await
        {
            info!("Prepare Dir Test End");
        }
        Ok(())
    }

    /// Prepare와 같으나 매 PutObject마다 새 S3Client를 만든다.
    pub async fn prepare_new(&mut self, check: bool, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Prepare New Client Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(client, move |c| async move {
                c.prepare_new(count, check, start).await
            });
        }
        info!("Prepare New Client Test Start");
        if self
            .watch(Report::Prepare, Report::PrepareFinal, "PrepareNew", false)
            .await
        {
            info!("Prepare New Client Test End");
        }
        Ok(())
    }

    /// 순차적으로 파일을 조회한다(사전에 올려 둬야 한다). 원본은 `Main.FilePath`를 파일로 넘긴다.
    pub async fn head(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Head Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let file_path = self.main_config.file_path.clone();
            let client = self.new_client(bucket, index, &file_path);
            let count = self.config.file_count;
            self.add_work(client, move |c| async move { c.head(count, 0).await });
        }
        info!("Head Test Start");
        if self
            .watch(Report::Head, Report::HeadFinal, "Head", false)
            .await
        {
            info!("Head Test End");
        }
        Ok(())
    }

    /// 파일을 무작위로 읽는다(사전에 올려 둬야 한다).
    pub async fn read(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Random Read Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let dummy_file = self.dummy_file(index);
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(client, move |c| async move { c.read(count).await });
            // 6단계: `_control != null`이면 `PrepareDistributedRead()`.
        }
        info!("Random Read Test Start");
        if self
            .watch(Report::Read, Report::ReadFinal, "Read", true)
            .await
        {
            info!("Random Read Test End");
        }
        Ok(())
    }

    /// 순차적으로 파일을 읽는다(사전에 올려 둬야 한다).
    pub async fn read_v2(&mut self, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Sequential Read Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let dummy_file = self.dummy_file(index);
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(
                client,
                move |c| async move { c.read_v2(count, start).await },
            );
        }
        info!("Sequential Read Test Start");
        if self
            .watch(Report::ReadV2, Report::ReadV2Final, "ReadV2", false)
            .await
        {
            info!("Sequential Read Test End");
        }
        Ok(())
    }

    /// ReadV2와 같으나 매 GetObject마다 새 S3Client를 만든다.
    pub async fn read_new(&mut self, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Sequential Read New Client Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let dummy_file = self.dummy_file(index);
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(
                client,
                move |c| async move { c.read_new(count, start).await },
            );
        }
        info!("Sequential Read New Client Test Start");
        if self
            .watch(Report::ReadV2, Report::ReadV2Final, "ReadNew", false)
            .await
        {
            info!("Sequential Read New Client Test End");
        }
        Ok(())
    }

    /// ListObjectsV2로 순차적으로 파일을 읽는다.
    pub async fn read_v3(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Listing Read Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let dummy_file = self.dummy_file(index);
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.read_v3().await });
        }
        info!("Listing Read Test Start");
        if self
            .watch(Report::Read, Report::ReadFinal, "ReadV3", true)
            .await
        {
            info!("Listing Read Test End");
        }
        Ok(())
    }

    /// 일정 시간 동안 오브젝트를 올린다. `super_random`이면 매번 다른 파일을 올린다.
    pub async fn write(&mut self, start: i32, super_random: bool) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Write Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            if super_random {
                self.add_work(client, move |c| async move { c.write_random(start).await });
            } else {
                self.add_work(client, move |c| async move { c.write(start).await });
            }
        }
        info!("Write Test Start");
        self.watch(Report::Write, Report::WriteFinal, "Write", true)
            .await;
        Ok(())
    }

    /// 일정 시간 동안 지정한 비율로 오브젝트를 올리고 읽는다.
    pub async fn mix(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("MIX Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.mix().await });
        }
        info!("MIX Test Start");
        let name = format!(
            "Write/Read({}:{})",
            self.config.write_ratio, self.config.read_ratio
        );
        self.watch(Report::Mix, Report::MixFinal, &name, true).await;
        Ok(())
    }

    /// 일정 시간 동안 지정한 비율로 오브젝트를 쓰고 읽고 지운다(쓰기 비율이 삭제 비율보다 높아야 한다).
    pub async fn mix_v2(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("MIX V2 Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.mix_v2().await });
        }
        info!("MIX V2 Test Start");
        let name = format!(
            "Write/Read/Delete({}:{}:{})",
            self.config.write_ratio, self.config.read_ratio, self.config.delete_ratio
        );
        self.watch(Report::All, Report::AllFinal, &name, true).await;
        Ok(())
    }

    /// All과 같으나 매 Put/Get/Delete마다 새 S3Client를 만든다.
    pub async fn mix_new(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("MIX New Client Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.mix_new().await });
        }
        info!("MIX New Client Test Start");
        self.watch(Report::All, Report::AllFinal, "MixNew", true)
            .await;
        Ok(())
    }

    /// 파일을 올리고 내려받아 ETag를 비교한다.
    pub async fn put_get(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("PutGet Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.put_get().await });
        }
        info!("PutGet Test Start");
        self.watch(Report::Mix, Report::MixFinal, "Write/Read(1:1)", true)
            .await;
        Ok(())
    }

    /// 일정 시간 동안 오브젝트를 올리고 내려받고 바로 지운다.
    pub async fn all(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("All Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            self.add_work(client, move |c| async move { c.all().await });
        }
        info!("All Test Start");
        self.watch(Report::All, Report::AllFinal, "All", true).await;
        Ok(())
    }

    /// 단일 오브젝트를 삭제한다. 버킷에 버전 설정과 만료 기한을 건다(예외는 호출한 쪽으로 올라간다).
    pub async fn delete_one(&mut self, key: &str, max_count: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("One Object Delete Test Initialize");
        let bucket = self.main_config.bucket_name.clone();
        self.client.create_bucket(&bucket).await;
        self.client
            .put_bucket_versioning(&bucket, Some(BucketVersioningStatus::Enabled))
            .await?;
        let rule = LifecycleRule::builder()
            .id("Rule1")
            .status(ExpirationStatus::Enabled)
            .expiration(
                LifecycleExpiration::builder()
                    .days(1)
                    .expired_object_delete_marker(true)
                    .build(),
            )
            .noncurrent_version_expiration(
                NoncurrentVersionExpiration::builder()
                    .noncurrent_days(1)
                    .build(),
            )
            .build()
            .map_err(|e| ScenarioError::new("System.ArgumentException", e.to_string()))?;
        let lifecycle = BucketLifecycleConfiguration::builder()
            .rules(rule)
            .build()
            .map_err(|e| ScenarioError::new("System.ArgumentException", e.to_string()))?;
        self.client
            .put_lifecycle_configuration(&bucket, lifecycle)
            .await?;
        for index in 0..self.config.thread_count {
            let client = self.new_client(bucket.clone(), index, "");
            let key = key.to_string();
            let count = if max_count > 0 {
                max_count
            } else {
                self.config.file_count
            };
            self.add_work(
                client,
                move |c| async move { c.delete_one(&key, count).await },
            );
        }
        info!("One Object Delete Test Start");
        if self
            .watch(Report::Delete, Report::DeleteFinal, "DeleteOne", false)
            .await
        {
            info!("One Object Delete Test End");
        }
        Ok(())
    }

    /// ListObjectsV2로 파일 목록을 삭제한다. `bulk`면 1000개씩 묶어 지운다.
    pub async fn delete(&mut self, bulk: bool, max_count: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Listing Delete Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let client = self.new_client(bucket, index, "");
            self.add_work(
                client,
                move |c| async move { c.delete(bulk, max_count).await },
            );
        }
        info!("Listing Delete Test Start");
        if self
            .watch(Report::Delete, Report::DeleteFinal, "Delete", false)
            .await
        {
            info!("Listing Delete Test End");
        }
        Ok(())
    }

    /// Prepare로 올린 오브젝트를 순차적으로 삭제하고 모두 지우면 끝낸다.
    pub async fn delete_v2(&mut self, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Sequential Delete Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let client = self.new_client(bucket, index, "");
            let count = self.config.file_count;
            self.add_work(
                client,
                move |c| async move { c.delete_v2(count, start).await },
            );
        }
        info!("Sequential Delete Test Start");
        if self
            .watch(Report::DeleteV2, Report::DeleteV2Final, "DeleteV2", false)
            .await
        {
            info!("Sequential Delete Test End");
        }
        Ok(())
    }

    /// DeleteV2와 같으나 매 DeleteObject마다 새 S3Client를 만든다.
    pub async fn delete_new(&mut self, start: i32) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Sequential Delete New Client Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let client = self.new_client(bucket, index, "");
            let count = self.config.file_count;
            self.add_work(
                client,
                move |c| async move { c.delete_new(count, start).await },
            );
        }
        info!("Sequential Delete New Client Test Start");
        if self
            .watch(Report::DeleteV2, Report::DeleteV2Final, "DeleteNew", false)
            .await
        {
            info!("Sequential Delete New Client Test End");
        }
        Ok(())
    }

    /// ListVersions로 버전을 삭제한다.
    pub async fn delete_version(
        &mut self,
        bulk: bool,
        max_count: i32,
        prefix: Option<&str>,
    ) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("ListVersions Delete Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let client = self.new_client(bucket, index, "");
            let prefix = prefix.map(str::to_string);
            self.add_work(client, move |c| async move {
                c.delete_version(bulk, max_count, prefix.as_deref()).await
            });
        }
        info!("ListVersions Delete Test Start");
        if self
            .watch(Report::Delete, Report::DeleteFinal, "DeleteVersion", false)
            .await
        {
            info!("ListVersions Delete Test End");
        }
        Ok(())
    }

    /// ListObject로 디렉터리를 삭제한다.
    pub async fn delete_directory(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Delete Directory Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let client = self.new_client(bucket, index, "");
            self.add_work(client, move |c| async move { c.delete_directory().await });
        }
        info!("Delete Directory Test Start");
        if self
            .watch(Report::Delete, Report::DeleteFinal, "DeleteDirectory", true)
            .await
        {
            info!("Delete Directory Test End");
        }
        Ok(())
    }

    // ---- HighLevel Utility Test ----

    /// AWS SDK TransferUtility로 파일을 올린다.
    pub async fn upload(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Upload Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let (count, part_size) = (self.config.file_count, self.main_config.part_size);
            self.add_work(
                client,
                move |c| async move { c.upload(count, part_size).await },
            );
        }
        info!("Upload Test Start");
        // 마지막 출력도 진행 상황 형식이다(원본은 `PrintMultiUpload`를 그대로 쓴다).
        self.watch(Report::MultiUpload, Report::MultiUpload, "Upload", false)
            .await;
        Ok(())
    }

    /// AWS SDK TransferUtility로 파일을 내려받는다.
    pub async fn download(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("Download Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let dummy_file = self.dummy_file(index);
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(client, move |c| async move { c.download(count).await });
        }
        info!("Download Test Start");
        self.watch(Report::Download, Report::Download, "Download", false)
            .await;
        Ok(())
    }

    // ---- Tag Test ----

    /// 스레드 0·1은 태그를 붙여 올리고 나머지는 TransferUtility로 올린다.
    pub async fn upload_tag(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("UploadTag Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let count = self.config.file_count;
            if index < 2 {
                self.add_work(client, move |c| async move { c.upload_tag(count).await });
            } else {
                let part_size = self.main_config.part_size;
                self.add_work(
                    client,
                    move |c| async move { c.upload(count, part_size).await },
                );
            }
        }
        info!("UploadTag Test Start");
        self.watch(Report::Prepare, Report::Prepare, "UploadTag", false)
            .await;
        Ok(())
    }

    // ---- AWS Test ----

    /// AWS S3에 Put/Head/Get/Delete/List를 반복하는 종합 테스트. `BucketType`을 무시한다.
    pub async fn aws_test(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("AWS Test Initialize");
        let bucket = self.main_config.bucket_name.clone();
        self.client.create_bucket(&bucket).await;
        for index in 0..self.config.thread_count {
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket.clone(), index, &dummy_file);
            let count = self.config.file_count;
            self.add_work(client, move |c| async move { c.aws_test(count).await });
        }
        info!("AWS Test Start");
        self.watch(Report::Aws, Report::Aws, "AWSTest", false).await;
        Ok(())
    }

    // ---- Multipart Test ----

    /// 오브젝트를 멀티파트로 올린다.
    pub async fn multipart_upload(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("MultipartUpload Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let (count, part_size) = (self.config.file_count, self.main_config.part_size);
            self.add_work(client, move |c| async move {
                c.multi_upload(count, part_size).await
            });
        }
        info!("MultipartUpload Test Start");
        self.watch(
            Report::MultiUpload,
            Report::MultiUpload,
            "MultipartUpload",
            false,
        )
        .await;
        Ok(())
    }

    /// 멀티파트로 올린 오브젝트를 내려받아 검증한다.
    pub async fn multipart_upload_v2(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("MultipartUploadV2 Test Initialize");
        self.create_main_bucket().await;
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, true).await;
            let dummy_file = self.dummy_file(index);
            self.create_test_file(&dummy_file).await;
            let client = self.new_client(bucket, index, &dummy_file);
            let (count, part_size) = (self.config.file_count, self.main_config.part_size);
            self.add_work(client, move |c| async move {
                c.multi_upload_v2(count, part_size).await
            });
        }
        info!("MultipartUploadV2 Test Start");
        self.watch(
            Report::MultiUploadV2,
            Report::MultiUploadV2,
            "MultipartUploadV2",
            false,
        )
        .await;
        Ok(())
    }

    // ---- List Test ----

    /// ListObject를 반복 호출해 목록 조회 성능을 잰다.
    pub async fn list_object_test(&mut self) -> Result<(), ScenarioError> {
        self.test_start_time = DotnetDateTime::now();
        info!("ListObjectTest Test Initialize");
        for index in 0..self.config.thread_count {
            let bucket = self.thread_bucket(index, false).await;
            let client = self.new_client(bucket, index, "");
            self.add_work(client, move |c| async move { c.list_object().await });
        }
        info!("ListObjectTest Test Start");
        self.watch(
            Report::ListObject,
            Report::ListObjectFinal,
            "ListObjectTest",
            true,
        )
        .await;
        Ok(())
    }

    // ---- 공통 틀 ----

    /// `BucketType`이 `Thread`가 아니면 공용 버킷을 만든다(원본 `CreateTestBucket`/`_client.CreateBucket`).
    async fn create_main_bucket(&self) {
        if self.config.bucket_type != EnumBucketTypes::Thread {
            self.client
                .create_bucket(&self.main_config.bucket_name)
                .await;
        }
    }

    /// 스레드가 쓸 버킷 이름. `Thread`면 `{BucketName}-{index:0000}`(`create`면 만든다).
    async fn thread_bucket(&self, index: i32, create: bool) -> String {
        if self.config.bucket_type == EnumBucketTypes::Thread {
            let bucket = format!("{}-{index:04}", self.main_config.bucket_name);
            if create {
                self.client.create_bucket(&bucket).await;
            }
            bucket
        } else {
            self.main_config.bucket_name.clone()
        }
    }

    /// 원본 `Utility.GetDummyFileName(index, _mainConfig.FilePath)`.
    fn dummy_file(&self, index: i32) -> String {
        dummy_file_name(index, Some(&self.main_config.file_path))
    }

    /// 원본 `CreateTestFile`/`Utility.CreateRandomFile(path, FileSize)`. 실패는 로그만 남긴다.
    async fn create_test_file(&self, path: &str) {
        let path = path.to_string();
        let size = self.main_config.file_size;
        // 파일 쓰기는 블로킹 작업이라 별도 스레드에서 한다.
        let _ =
            tokio::task::spawn_blocking(move || create_random_file(Path::new(&path), size, false))
                .await;
    }

    /// 원본 `new UpDownClient(bucketName, index, file, _clientConfig, _user)`. `Quit`은 이 테스트의 토큰에 묶는다.
    fn new_client(&self, bucket: String, index: i32, file: &str) -> Arc<UpDownClient> {
        Arc::new(
            UpDownClient::new(
                bucket,
                index,
                file,
                self.client_config.clone(),
                self.user.clone(),
            )
            .with_quit(&self.cancel),
        )
    }

    /// 원본 `_testList.Add(client)`, `_taskList.Add(new Thread(...))`.
    fn add_work<F, Fut>(&mut self, client: Arc<UpDownClient>, work: F)
    where
        F: FnOnce(Arc<UpDownClient>) -> Fut,
        Fut: Future<Output = Result<(), UpDownError>> + Send + 'static,
    {
        let worker = client.clone();
        self.tasks.add(client, work(worker));
    }

    /// 시작 로그 뒤의 공통 흐름: 최종 결과 준비 → 시작 → 감시 루프 → 최종 결과.
    /// `until_end`는 감시 루프의 `!_watcher.IsEnd`(시간 제한) 검사. 시작에 실패하면 `false`(원본은 여기서 끝낸다).
    async fn watch(
        &mut self,
        progress: Report,
        final_report: Report,
        name: &str,
        until_end: bool,
    ) -> bool {
        self.final_report = Some((final_report, name.to_string()));
        self.final_result = FinalResult::default();
        if !self.task_start().await {
            return false;
        }
        self.watcher.start();
        while (!until_end || !self.watcher.is_end()) && self.tasks.check() {
            if self.watcher.is_next() {
                self.print(progress);
            } else {
                idle().await;
            }
        }
        self.tasks.stop();
        self.complete_final_result().await;
        true
    }

    /// 원본 `TaskStart()`(`_control == null`): 종료 처리기를 등록하고 작업을 시작한다.
    async fn task_start(&mut self) -> bool {
        if self.tasks.is_empty() {
            error!("Task Start Failed");
            return false;
        }
        // 6단계: `_control != null`이면 `ThrowIfCancellationRequested`, 클라이언트 공개, `ReadyAndWait`.
        // 원본 `RegisterShutdownHandlers()`, `RegisterActiveTest()`.
        self.active = Some(activate(&self.cancel, Handler::Persistent));
        self.tasks.start().await
    }

    /// 원본 `FinalizeTasks()`: 클라이언트를 멈추고 작업이 끝나길 기다린다(한 번만).
    async fn finalize_tasks(&mut self) {
        if self.finalized {
            return;
        }
        self.tasks.stop();
        self.tasks.join().await;
        // 원본 `UnregisterActiveTest()`.
        self.active = None;
        self.finalized = true;
    }

    /// 원본 `CompleteFinalResult()`: 마무리, 최종 출력, 결과 저장을 한 번만 한다.
    async fn complete_final_result(&mut self) {
        let Some((report, name)) = self.final_report.clone() else {
            return;
        };
        if !self.final_result.begin() {
            return;
        }
        self.finalize_tasks().await;
        self.print(report);
        if !name.is_empty() {
            self.save_result_to_json(&name);
        }
    }

    /// 원본 `Print*`: 통계를 갱신하고 해당 형식으로 로그를 남긴다.
    fn print(&mut self, report: Report) {
        let clients: Vec<&UpDownClient> = self.tasks.clients().iter().map(|c| &**c).collect();
        self.stats.update(&clients);
        let total = i64::from(self.config.total_file_count());
        let now = self.watcher.now();
        let part_size = self.main_config.part_size;
        let s = &self.stats;
        let message = match report {
            Report::Prepare => s.prepare_message(total, now),
            Report::Write => s.write_message(now),
            Report::Head => s.head_message(now),
            Report::Read => s.read_message(now),
            Report::ReadV2 => s.read_total_message(total, now),
            Report::Delete => s.delete_message(now),
            Report::DeleteV2 => s.delete_total_message(total, now),
            Report::MultiUpload => s.multi_upload_message(total, now, part_size),
            Report::MultiUploadV2 => s.multi_upload_v2_message(total, now, part_size),
            Report::Download => s.download_message(total, now),
            Report::Mix => s.mix_message(now),
            Report::All => s.all_message(now),
            Report::Aws => s.aws_message(now),
            Report::ListObject => s.list_object_message(now),
            Report::PrepareFinal => s.prepare_final_message(total, now),
            Report::WriteFinal => s.write_final_message(now),
            Report::HeadFinal => s.head_final_message(now),
            Report::ReadFinal => s.read_final_message(now),
            Report::ReadV2Final => s.read_v2_final_message(total, now),
            Report::DeleteFinal => s.delete_final_message(now),
            Report::DeleteV2Final => s.delete_v2_final_message(total, now),
            Report::MixFinal => s.mix_final_message(now),
            Report::AllFinal => s.all_final_message(now),
            Report::ListObjectFinal => s.list_object_final_message(now),
        };
        log_info(&message);
    }

    /// 원본 `SaveResultToJson(testType)`. `Time`은 경과 시간이 아니라 설정의 `Times`다(원본 그대로).
    fn save_result_to_json(&self, test_type: &str) {
        let Some(save) = self
            .config
            .save
            .as_deref()
            .filter(|save| !save.trim().is_empty())
        else {
            return;
        };
        let mut result = self.stats.to_up_down_result(
            test_type,
            &self.config,
            &self.main_config,
            Decimal::from(self.watcher.end_time()),
        );
        result.start_time = self.test_start_time;
        result.end_time = DotnetDateTime::now();
        save::save_result(&result, save, test_type);
    }
}
