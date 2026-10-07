//! 원본 `CommandDispatcher`의 시나리오 실행: MultiPartTest(ManualUpload), MultiUploadTest, RangeReadTest, FindTagTest, MultiDownloadTest, IoTest.

use std::time::Instant;

use awscli_rest_scenarios::multi_part::MultiPartTest;
use tracing::info;

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    match menu {
        MenuList::ManualUpload => manual_upload(ctx).await,
        _ => not_ported(menu),
    }
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
