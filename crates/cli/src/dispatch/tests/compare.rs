//! 원본 `CommandDispatcher`의 시나리오 실행: CompareTest, CopyTest(RangeReadCopy), DuplicateTest, LifecycleTest, MoverTest.

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(_ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    // TODO(5단계): 시나리오 실행.
    not_ported(menu)
}
