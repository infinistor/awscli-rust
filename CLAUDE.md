# awscli-rust

TESTCore(.NET)를 Rust로 옮기는 프로젝트. 전체 계획과 단계는 [RUST_MIGRATION.md](RUST_MIGRATION.md)를 따른다.

## 원본

-   .NET 원본: `E:\Code\Git\TESTCore` (전환 기간 동안 기준 구현, 수정하지 않음)
-   이식할 때는 원본 C# 파일을 먼저 읽고, 동작(출력, 오류 메시지, 종료 코드, 요청 헤더)을 그대로 맞춘다.
-   원본에 버그가 있어 보여도 임의로 고치지 말고, 사용자에게 알린 뒤 결정을 따른다.

## 호환 원칙

-   바이너리 이름은 `awscli-rust`. 그 외 CLI 옵션 이름, `config.ini` 형식, 콘솔 출력, CSV·JSON 결과, Controller·Worker HTTP 계약은 TESTCore와 같아야 한다.
-   통계 측정 구간(`TimeWatcher`, `UpDownStats`)과 체크섬·서명 결과가 원본과 일치해야 한다.

## 구조

-   cargo workspace. 크레이트는 `crates/<이름>`에 두고 패키지 이름은 `awscli-rust-<이름>`으로 한다.
-   비동기 런타임은 `tokio`, 취소는 `tokio_util::sync::CancellationToken`으로 통일한다.
-   라이브러리 크레이트의 오류는 `thiserror`, 바이너리는 `anyhow`를 쓴다.
-   큰 C# 파일(`CommandDispatcher.cs`, `UpDownTest.cs`, `UpDownClient.cs`, `INIParser.cs`)은 기능 단위로 나눠 옮긴다.

## 작업 규칙

-   변경 후 반드시 실행하고 모두 통과시킨다:
    ```bash
    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    ```
-   모듈 하나를 옮기면 원본과 비교하는 테스트를 함께 추가한다(`tests/parity/`).
-   커밋은 모듈 단위로 하나씩, 메시지는 한국어로 TESTCore 저장소와 같은 형식으로 쓴다.

## 개발 환경

-   Windows에서는 cargo를 PowerShell에서 실행한다. Git Bash의 `/usr/bin/link`가 MSVC `link.exe`를 가려 링크가 실패한다.
-   MSVC 타깃(`x86_64-pc-windows-msvc`)을 쓰며 Visual Studio Build Tools의 C++ 워크로드가 필요하다.
