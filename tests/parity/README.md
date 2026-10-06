# parity 테스트

TESTCore(.NET)와 awscli-rest의 외부 동작을 비교한다.

-   `baseline/`: .NET 바이너리(`TestCore.exe`)로 수집한 기준 출력. 콘솔 출력은 UTF-8로 저장하고, 로그 줄(`INFO `, `ERROR` 등)은 필요할 때만 남긴다.
    -   `version.txt`: `TestCore --version` 출력 (`78004ff` 빌드)
    -   `help.txt`: `TestCore --help` 출력에서 로그 줄을 뺀 것 (`78004ff` 빌드, 4단계 옵션 파서 비교용)
-   테스트 파일(`*.rs`)은 `crates/cli/Cargo.toml`의 `[[test]]`로 등록해 `awscli-rest` 바이너리를 직접 실행한다.

## 기준 출력 다시 수집하기

Windows에서 .NET 콘솔은 파이프 출력에 시스템 코드 페이지(CP949)를 쓰므로 UTF-8로 바꿔서 수집한다.

```powershell
[Console]::OutputEncoding = [Text.Encoding]::UTF8
TestCore.exe --help
```
