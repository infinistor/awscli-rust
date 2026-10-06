//! 빌드 시점의 Git 정보로 `태그_커밋수_해시` 형식의 버전 문자열을 만든다.
//! TESTCore.csproj의 `GetGitVersion` 대상과 같은 명령과 기본값을 사용한다.

use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn main() {
    let tag = git(&["describe", "--tags", "--abbrev=0"]).unwrap_or_else(|| "v0.0.0".into());
    let count = git(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "0".into());
    let hash = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=AWSCLI_REST_GIT_VERSION={tag}_{count}_{hash}");

    // 커밋·태그가 바뀌면 다시 계산한다.
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]).map(PathBuf::from) {
        for path in ["HEAD", "packed-refs", "refs/tags"] {
            println!("cargo:rerun-if-changed={}", git_dir.join(path).display());
        }
        if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(head_ref).display()
            );
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
