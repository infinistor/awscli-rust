//! 원본 `CommandDispatcher`의 시나리오 실행: UpDownTest(Upload·Download·Prepare·Put·Get·Delete·Mix·All·Multipart·PutTag·AWS·ListObj·FullTest·MultiDelete).

use super::super::{CommandContext, CommandResult, not_ported};
use crate::menu::MenuList;

pub(super) async fn run(_ctx: &mut CommandContext, menu: MenuList) -> CommandResult {
    // TODO(5단계): 시나리오 실행.
    not_ported(menu)
}
