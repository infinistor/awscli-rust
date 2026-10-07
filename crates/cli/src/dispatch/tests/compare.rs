//! 원본 `CommandDispatcher`의 시나리오 실행: CompareTest, CopyTest(RangeReadCopy), DuplicateTest, LifecycleTest, MoverTest.

use std::time::Instant;

use awscli_rest_config::CopyConfig;
use awscli_rest_scenarios::compare::CompareTest;
use awscli_rest_scenarios::copy::CopyTest;
use tracing::info;

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    match menu {
        MenuList::CompareTest => {
            let config = ctx.config();
            let test = if ctx.options.another {
                CompareTest::with_alt(config.compare.clone(), &config.main_user, &config.alt_user)
            } else {
                CompareTest::new(config.compare.clone(), &config.main_user)
            };
            test.start().await
        }
        MenuList::RangeReadCopy => {
            info!("RangeReadCopy Start");
            let o = &ctx.options;
            let copy = CopyConfig {
                source_bucket: o.source.clone().unwrap_or_default(),
                source_object: o.source_key.clone().unwrap_or_default(),
                target_bucket: o.bucket_name.clone().unwrap_or_default(),
                target_object: o.key.clone().unwrap_or_default(),
            };
            let started = Instant::now();
            let test = CopyTest::new(copy, ctx.config().main_user.clone());
            test.start().await?;
            info!(
                "RangeReadCopy : complete time = {}ms",
                started.elapsed().as_millis()
            );
            Ok(0)
        }
        // TODO(5단계): 나머지 시나리오 실행.
        _ => not_ported(menu),
    }
}
