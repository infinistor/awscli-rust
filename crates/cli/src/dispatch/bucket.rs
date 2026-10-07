//! 원본 `CommandDispatcher` 중 버킷 생성·삭제·조회·목록, 버전 관리, 소유권 설정.

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
