//! 원본 `CommandDispatcher` 중 유틸리티(Legal Hold, SSE-S3, 상위 API 업로드·다운로드, 버킷 비우기).

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
