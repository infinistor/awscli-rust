//! 부하·기능 테스트 시나리오 (TESTCore Test/*)
//!
//! 원본 `Test/X.cs` 하나를 모듈 하나로 옮긴다. 시나리오와 CLI 명령이 함께 쓰는 .NET 예외 형식([`ScenarioError`]),
//! 입력 처리([`input`]), 파일 도우미([`files`])도 여기에 둔다.

pub mod clear;
pub mod error;
pub mod files;
pub mod input;

pub use error::ScenarioError;
