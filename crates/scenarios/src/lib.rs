//! 부하·기능 테스트 시나리오 (TESTCore Test/*)
//!
//! 원본 `Test/X.cs` 하나를 모듈 하나로 옮긴다. 시나리오와 CLI 명령이 함께 쓰는 .NET 예외 형식([`ScenarioError`]),
//! 입력 처리([`input`]), 파일 도우미([`files`]), 원본 `Utility`([`util`]), 스레드 조립([`runner`]),
//! Ctrl+C 처리([`shutdown`])도 여기에 둔다.
//!
//! 호출되지 않는 원본 `ReplicationTest`(디스패처에 case가 없다)와 `ListObjTest`(생성하는 곳이 없다)는 옮기지 않는다.

pub mod error;
pub mod files;
pub mod input;
pub mod runner;
pub mod shutdown;
pub mod util;

pub use error::ScenarioError;

// UpDownTest·FullTest
pub mod clear;
pub mod up_down;

// LocalTest, MultiSystemTest, AccessIpsTest, UsedSizeTest
pub mod access_ips;
pub mod local;
pub mod multi_system;
pub mod used_size;

// CompareTest, CopyTest, DuplicateTest, LifecycleTest, MoverTest
pub mod compare;
pub mod copy;
pub mod duplicate;
pub mod lifecycle;
pub mod mover;

// MultiPartTest, MultiUploadTest, RangeReadTest, FindTagTest, MultiDownloadTest, IoTest
pub mod find_tag;
pub mod io;
pub mod multi_download;
pub mod multi_part;
pub mod multi_upload;
pub mod range_read;
