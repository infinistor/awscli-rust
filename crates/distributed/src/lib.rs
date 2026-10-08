//! 분산 실행(TESTCore `Distributed/*`): Controller가 여러 Worker에 같은 부하 테스트를 나눠 실행하고 결과를 모은다.
//!
//! 통신은 Worker의 `/driver` HTTP API(JSON, camelCase)다. .NET Controller·Worker와 섞어 쓸 수 있어야 하므로 계약
//! 형식([`contracts`])과 상태 전이(`awscli_rest_scenarios::run_control`)를 원본과 같게 둔다.

pub mod console;
pub mod contracts;
pub mod controller;
pub mod diagnostics;
pub mod result_writer;
pub mod runner;
pub mod settings;
pub mod worker;
