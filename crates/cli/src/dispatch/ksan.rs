//! 원본 `CommandDispatcher` 중 KSAN 전용 API(태그 인덱스·태그 검색).

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
