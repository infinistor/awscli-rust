//! 원본 종료 처리기(`Console.CancelKeyPress`)를 프로세스 전체 `CancellationToken` 하나로 옮긴다.
//!
//! 원본은 처리기를 등록한 시나리오(UpDownTest의 `RegisterShutdownHandlers`, MultiSystemTest의 `RunWithShutdown`)에서만
//! Ctrl+C를 가로채 테스트를 멈추고 최종 결과를 남긴다. 처리기가 없는 명령은 Ctrl+C에 바로 끝난다.
//! 그래서 [`register_ctrl_c`]도 원본이 처리기를 등록하는 자리에서 처음 부를 때 한 번만 Ctrl+C를 가로챈다. 그 뒤로는
//! (원본처럼 처리기가 남아 있으므로) Ctrl+C마다 토큰을 취소한다.
//!
//! 원본 `ProcessExit` 처리기(`StopAllActiveTests`)는 정상 흐름에서 이미 최종 처리를 마친 뒤에 불려 할 일이 없으므로
//! 옮기지 않는다. 원본 처리기 스레드와 주 스레드가 함께 최종 출력을 하던 경합도 재현하지 않는다.

use std::sync::Once;

use tokio_util::sync::CancellationToken;

static REGISTER: Once = Once::new();

/// Ctrl+C를 가로채 `token`을 취소한다(프로세스에서 처음 부를 때만 등록). tokio 런타임 안에서 불러야 한다.
pub fn register_ctrl_c(token: &CancellationToken) {
    REGISTER.call_once(|| {
        let token = token.clone();
        tokio::spawn(async move {
            while tokio::signal::ctrl_c().await.is_ok() {
                token.cancel();
            }
        });
    });
}
