//! 원본 `CommandDispatcher`의 시나리오 실행: CompareTest, CopyTest(RangeReadCopy), DuplicateTest, LifecycleTest, MoverTest.

use awscli_rest_scenarios::compare::CompareTest;

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
        // TODO(5단계): 나머지 시나리오 실행.
        _ => not_ported(menu),
    }
}
