//! 원본 종료 처리기(`Console.CancelKeyPress`)를 실행 중인 테스트의 `CancellationToken` 목록으로 옮긴다.
//!
//! 원본은 처리기를 등록한 시나리오에서만 Ctrl+C를 가로챈다.
//!
//! - UpDownTest(`RegisterShutdownHandlers`): 처리기를 프로세스에 한 번 등록하고 지우지 않는다. Ctrl+C마다 그때
//!   활성 목록(`_activeTests`)에 있는 테스트만 멈추고 최종 결과를 남긴다. 활성 테스트가 없으면 아무 일도 하지 않는다
//!   (프로세스도 끝나지 않는다). FullTest의 Prepare 중 Ctrl+C는 Prepare만 멈추고 다음 ReadV2는 그대로 돈다.
//! - MultiSystemTest(`RunWithShutdown`): 실행 동안만 처리기를 걸고 끝나면 지운다.
//! - 처리기가 없는 명령은 Ctrl+C에 바로 끝난다(.NET 종료 코드 `STATUS_CONTROL_C_EXIT`).
//!
//! 여기서는 Ctrl+C를 처음 가로챌 때 한 번 tokio 처리기를 걸고, [`activate`]로 등록한 토큰을 Ctrl+C마다 취소한다.
//! 등록한 토큰이 없을 때는 UpDownTest 처리기가 남아 있으면 무시하고, 아니면 원본처럼 프로세스를 끝낸다.
//!
//! 원본 `ProcessExit` 처리기(`StopAllActiveTests`)는 정상 흐름에서 이미 최종 처리를 마친 뒤에 불려 할 일이 없으므로
//! 옮기지 않는다. 원본 처리기 스레드와 주 스레드가 함께 최종 출력을 하던 경합도 재현하지 않는다.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, Once};

use tokio_util::sync::CancellationToken;

/// Ctrl+C로 끝날 때의 종료 코드(`STATUS_CONTROL_C_EXIT`).
pub const CONTROL_C_EXIT_CODE: i32 = 0xC000_013A_u32 as i32;

static INSTALL: Once = Once::new();
/// UpDownTest 처리기처럼 지우지 않는 처리기가 걸렸는지.
static PERSISTENT: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicU64 = AtomicU64::new(0);
static ACTIVE: Mutex<Vec<(u64, CancellationToken)>> = Mutex::new(Vec::new());

/// 처리기 등록 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handler {
    /// 프로세스에 남는 처리기(UpDownTest `RegisterShutdownHandlers`).
    Persistent,
    /// 실행 동안만 거는 처리기(MultiSystemTest `RunWithShutdown`).
    Scoped,
}

/// 활성 테스트 등록. 버리면 목록에서 빠진다(원본 `UnregisterActiveTest`, `CancelKeyPress -= handler`).
#[derive(Debug)]
pub struct ActiveGuard(u64);

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        active().retain(|(id, _)| *id != self.0);
    }
}

fn active() -> std::sync::MutexGuard<'static, Vec<(u64, CancellationToken)>> {
    ACTIVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Ctrl+C가 오면 `token`을 취소하도록 등록한다. tokio 런타임 안에서 불러야 한다.
pub fn activate(token: &CancellationToken, handler: Handler) -> ActiveGuard {
    if handler == Handler::Persistent {
        PERSISTENT.store(true, Ordering::Relaxed);
    }
    INSTALL.call_once(|| {
        tokio::spawn(async {
            while tokio::signal::ctrl_c().await.is_ok() {
                on_ctrl_c();
            }
        });
    });
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    active().push((id, token.clone()));
    ActiveGuard(id)
}

fn on_ctrl_c() {
    let tokens: Vec<CancellationToken> = active().iter().map(|(_, t)| t.clone()).collect();
    if tokens.is_empty() && !PERSISTENT.load(Ordering::Relaxed) {
        std::process::exit(CONTROL_C_EXIT_CODE);
    }
    for token in tokens {
        token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn guard_unregisters() {
        let token = CancellationToken::new();
        let guard = activate(&token, Handler::Scoped);
        assert_eq!(active().len(), 1);
        on_ctrl_c();
        assert!(token.is_cancelled());
        drop(guard);
        assert!(active().is_empty());
    }
}
