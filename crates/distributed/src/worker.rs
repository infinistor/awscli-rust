//! 원본 `Distributed/WorkerHost.cs`: `WorkerManager`(작업 하나씩, RunId 중복 판정, 결과 저장), `WorkerJob`, `WorkerHost`(HTTP).

use awscli_rest_scenarios::ScenarioError;
use tokio_util::sync::CancellationToken;

use crate::settings::DistributedSettings;

/// 원본 `WorkerHost.RunAsync(settings, debug, ct)`: `[worker] url`의 주소에서 `/driver` API를 연다. `shutdown`이 취소되면
/// 실행 중인 작업을 멈추고(최대 30초 대기) 끝낸다.
pub async fn run_worker(
    settings: DistributedSettings,
    debug: bool,
    shutdown: CancellationToken,
) -> Result<(), ScenarioError> {
    let _ = (settings, debug, shutdown);
    Err(ScenarioError::new(
        "System.NotImplementedException",
        "Worker는 아직 이식되지 않았습니다.",
    ))
}
