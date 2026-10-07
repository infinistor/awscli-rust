//! 원본 `CommandDispatcher` 중 버킷·객체 ACL 조회·설정.

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
