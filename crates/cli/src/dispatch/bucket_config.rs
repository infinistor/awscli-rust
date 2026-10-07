//! 원본 `CommandDispatcher` 중 버킷 분석·CORS·암호화·인벤토리·메트릭·로깅·알림·웹사이트 설정.

use super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

/// 옮긴 메뉴.
pub(super) const PORTED: &[MenuList] = &[];

pub(super) async fn run(ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    let _ = ctx;
    not_ported(menu)
}
