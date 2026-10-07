# parity 테스트

TESTCore(.NET)와 awscli-rest의 외부 동작을 비교한다.

-   `baseline/`: .NET으로 수집한 기준 출력. 콘솔 출력은 UTF-8로 저장하고, 로그 줄(`INFO `, `ERROR` 등)은 필요할 때만 남긴다.
    -   `version.txt`: `TestCore --version` 출력 (`78004ff` 빌드)
    -   `help.txt`: `TestCore --help` 출력에서 로그 줄을 뺀 것 (`78004ff` 빌드, 4단계 옵션 파서 비교용)
    -   `ini/*.json`: `ini/*.ini`를 TESTCore `IniFile`로 읽은 결과
    -   `config/*.json`: `config/*.ini`를 `Config.GetConfig`로 읽은 뒤 `Config.ToString()`한 JSON. 사용자 섹션을 지정한 경우 `<픽스처>.user-<이름>.json`(Windows는 파일 이름 대소문자를 구분하지 않으므로 이름이 겹치지 않게 한다). 로드에 실패한 입력은 `null`이다. `Default.FilePath`가 비어 있으면 원본이 현재 디렉터리를 쓰므로 `<CWD>`로 바꿔 저장했다.
    -   `portal/*.json`·`mover/*.json`·`zeromq/*.json`: 오라클의 같은 이름 명령(`portal`, `mover`, `zeromq`)이 실제 `PortalManager`·`MoverClient`·`ZeroMqClient`를 로컬 서버에 연결해 기록한 요청(요청 줄·헤더·본문), 결과(반환값·예외 형식과 메시지), log4net 로그. `variants`가 있는 사례는 응답 본문만 바꿔가며 JSON 읽기 경계(`System.Text.Json` 오류 메시지 포함)를 비교한다. 다시 만들 때는 `pwsh tests/parity/gen-client-baselines.ps1`.
-   `config/`: Config 픽스처(`-text`로 보관).
-   `portal/`, `mover/`, `zeromq/`: 위 기준 출력을 만드는 사례(`op`와 응답 본문 등). Portal·Mover 테스트는 이 디렉터리를 모두 읽는다.
-   `ini/`: INI 파서 픽스처. 줄 끝과 BOM을 그대로 보관하도록 `.gitattributes`에서 `-text`로 지정했다.
-   테스트 파일(`*.rs`)은 해당 크레이트의 `Cargo.toml`에 `[[test]]`로 등록한다.

## 기준 출력 다시 수집하기

`tools/dotnet-oracle`은 TESTCore 빌드 결과(`TestCore.dll`)를 그대로 호출해 기준 출력을 만든다. TESTCore 경로가 다르면 `TestCoreBin`(빌드)·`TESTCORE_BIN`(실행)으로 지정한다.

```powershell
dotnet build tools/dotnet-oracle
dotnet tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll ini tests/parity/ini/sample.ini > tests/parity/baseline/ini/sample.json
dotnet tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll config tests/parity/ini/sample.ini
```

Windows에서 .NET 콘솔은 파이프 출력에 시스템 코드 페이지(CP949)를 쓰므로 `TestCore.exe` 출력은 UTF-8로 바꿔서 수집한다(오라클은 UTF-8로 출력한다).

```powershell
[Console]::OutputEncoding = [Text.Encoding]::UTF8
TestCore.exe --help
```
