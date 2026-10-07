//! 원본 `CommandDispatcher` 중 객체 업로드·다운로드·복사·삭제·조회·목록과 객체 설정.

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
