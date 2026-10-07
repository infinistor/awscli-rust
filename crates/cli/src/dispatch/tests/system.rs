//! 원본 `CommandDispatcher`의 시나리오 실행: LocalTest, MultiSystemTest, AccessIpsTest, UsedSizeTest.

use awscli_rest_scenarios::local::LocalTest;

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    use MenuList::*;
    match menu {
        LocalPrepareTest
        | LocalPutTest
        | LocalGetTest
        | LocalGetTestV2
        | LocalPutGetTest
        | LocalDeleteTest
        | LocalMultipartPrepareTest
        | LocalMultipartPutTest
        | LocalMultipartGetTest
        | LocalMultipartGetTestV2
        | LocalMultipartPutGetTest => local(ctx, menu).await,
        _ => not_ported(menu),
    }
}

/// `LocalXxxTest` 메뉴. 대상 경로 확인(`TargetPath`)은 상위(`tests/mod.rs`)에서 한다.
async fn local(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    use MenuList::*;
    let config = ctx.config();
    let use_multipart = matches!(
        menu,
        LocalMultipartPrepareTest
            | LocalMultipartPutTest
            | LocalMultipartGetTest
            | LocalMultipartGetTestV2
            | LocalMultipartPutGetTest
    );
    let mut test = LocalTest::new(
        &config.main,
        &config.up_down,
        &config.main.target_path,
        use_multipart,
    );
    let (check, start) = (ctx.options.check, ctx.options.start_count);
    match menu {
        LocalPrepareTest | LocalMultipartPrepareTest => test.prepare(check, start).await?,
        LocalPutTest | LocalMultipartPutTest => test.write().await?,
        LocalGetTest | LocalMultipartGetTest => test.read().await?,
        LocalGetTestV2 | LocalMultipartGetTestV2 => test.read_v2(start).await?,
        LocalPutGetTest | LocalMultipartPutGetTest => test.put_get().await?,
        _ => test.delete().await?,
    }
    Ok(0)
}
