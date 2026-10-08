//! TESTCore `Util/VersionInfoProvider.cs` 대응.

/// 빌드 시점의 `태그_커밋수_해시` 버전 문자열. `build.rs`가 채운다.
pub fn version_info() -> &'static str {
    match env!("AWSCLI_RUST_GIT_VERSION") {
        "" => "unknown",
        version => version,
    }
}
