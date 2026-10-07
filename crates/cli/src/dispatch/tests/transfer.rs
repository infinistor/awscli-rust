//! 원본 `CommandDispatcher`의 시나리오 실행: MultiPartTest(ManualUpload), MultiUploadTest, RangeReadTest, FindTagTest, MultiDownloadTest, IoTest.

use std::time::Instant;

use awscli_rest_scenarios::multi_part::MultiPartTest;
use awscli_rest_scenarios::multi_upload::MultiUploadTest;
use tracing::info;

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    match menu {
        MenuList::ManualUpload => manual_upload(ctx).await,
        MenuList::MultiUploadTest => multi_upload_test(ctx).await,
        _ => not_ported(menu),
    }
}

/// 원본 `case MenuList.MultiUploadTest`(검증은 `tests/mod.rs`).
async fn multi_upload_test(ctx: &mut CommandContext) -> CommandResult {
    let o = &ctx.options;
    let thread = if o.thread < 1 { 10 } else { o.thread };
    let bucket = o.bucket_name.as_deref().unwrap_or_default();
    let key = o.key.as_deref().unwrap_or_default();
    let (file_size, part_size) = (o.file_size, o.part_size);

    info!("MultiUploadTest Start");

    let sw = Instant::now();
    let client = ctx.client();
    // 버킷 생성
    client.create_bucket(bucket).await;
    // 스레드 생성
    let mut tasks = Vec::new();
    for i in 0..thread {
        if file_size > 1 {
            let test = MultiUploadTest::new_multipart(
                client.clone(),
                i,
                bucket,
                key,
                file_size,
                part_size as i32,
            );
            tasks.push(tokio::spawn(async move { test.multipart_upload().await }));
        } else {
            let test = MultiUploadTest::new_put(client.clone(), i, bucket, key, file_size);
            tasks.push(tokio::spawn(async move { test.put_object().await }));
        }
    }
    // 모든 스레드가 종료될 때까지 대기
    for task in tasks {
        let _ = task.await;
    }

    info!(
        "MultiUploadTest : complete time = {}ms",
        sw.elapsed().as_millis()
    );
    Ok(0)
}

/// 원본 `case MenuList.ManualUpload`(검증은 `tests/mod.rs`).
async fn manual_upload(ctx: &mut CommandContext) -> CommandResult {
    let o = &ctx.options;
    info!("ManualUpload Start");
    let sw = Instant::now();
    let mut test = MultiPartTest::new(&ctx.config().main_user);
    test.start(
        o.bucket_name.as_deref().unwrap_or_default(),
        o.key.as_deref().unwrap_or_default(),
        o.file_path.as_deref().unwrap_or_default(),
        o.part_size,
        10,
    )
    .await?;
    info!(
        "ManualUpload : complete time = {}ms",
        sw.elapsed().as_millis()
    );
    Ok(0)
}
