//! Local·Curl·ZeroMq·MultiSystem·UpDown 클라이언트, Portal·Jenkins·Mover
//!
//! 이식한 것: `KHttpClient`([`khttp`]), `PortalManager`([`portal`]).
//!
//! 이식하지 않은 것: `Jenkins/JenkinsManager.cs`, `Util/Curl.cs`. TESTCore 안에서 어디서도 쓰지 않는다
//! (`JenkinsManager`·`Curl` 형식을 참조하는 코드가 없다). `JenkinsConfig`는 설정 파일 읽기용이라 `awscli-rest-config`에 있다.

mod http;
pub mod json;
pub mod khttp;
pub mod portal;
