//! TESTCore `Util/TestCoreApplication.cs` 대응: 인자 파싱, 설정 로딩, 명령 실행 흐름.
//!
//! 지금은 골격 단계라 `-v|--version`만 처리한다. 옵션 파서(`Cli/CliOptionParser.cs`)와
//! 명령 디스패치는 4단계에서 옮긴다.

use std::time::Instant;

use tracing::info;

use crate::version::version_info;

/// TESTCore `ERROR_NORMAL`.
pub const ERROR_NORMAL: i32 = -1;

/// 명령행 인자(프로그램 이름 제외)로 애플리케이션을 실행하고 종료 코드를 돌려준다.
pub fn run(args: &[String]) -> i32 {
    let started = Instant::now();
    info!("Main start");

    if args.iter().any(|arg| is_option(arg, &["v", "version"])) {
        println!("{}", version_info());
        return 0;
    }

    // TODO(4단계): CliOptionParser·ConfigBootstrapper·CommandDispatcher 이식 후 제거.
    eprintln!(
        "awscli-rest: 아직 이식되지 않은 명령입니다: {}",
        args.join(" ")
    );
    info!("Main complete time = {}ms", started.elapsed().as_millis());
    ERROR_NORMAL
}

/// Mono.Options처럼 `-`, `--`, `/` 접두사를 모두 받아 옵션 이름을 비교한다.
fn is_option(arg: &str, names: &[&str]) -> bool {
    let name = arg
        .strip_prefix("--")
        .or_else(|| arg.strip_prefix('-'))
        .or_else(|| arg.strip_prefix('/'));
    name.is_some_and(|name| names.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_prefixes() {
        for arg in ["-v", "--v", "/v", "-version", "--version", "/version"] {
            assert!(is_option(arg, &["v", "version"]), "{arg}");
        }
        for arg in ["v", "version", "---version", "--ver"] {
            assert!(!is_option(arg, &["v", "version"]), "{arg}");
        }
    }
}
