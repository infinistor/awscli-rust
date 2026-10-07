# parity 테스트

TESTCore(.NET)와 awscli-rest의 외부 동작을 비교한다.

-   `baseline/`: .NET으로 수집한 기준 출력. 콘솔 출력은 UTF-8로 저장하고, 로그 줄(`INFO `, `ERROR` 등)은 필요할 때만 남긴다.
    -   `version.txt`: `TestCore --version` 출력 (`78004ff` 빌드)
    -   `help.txt`: `TestCore --help` 출력에서 로그 줄을 뺀 것 (`78004ff` 빌드, 4단계 옵션 파서 비교용)
    -   `ini/*.json`: `ini/*.ini`를 TESTCore `IniFile`로 읽은 결과
    -   `config/*.json`: `config/*.ini`를 `Config.GetConfig`로 읽은 뒤 `Config.ToString()`한 JSON. 사용자 섹션을 지정한 경우 `<픽스처>.user-<이름>.json`(Windows는 파일 이름 대소문자를 구분하지 않으므로 이름이 겹치지 않게 한다). 로드에 실패한 입력은 `null`이다. `Default.FilePath`가 비어 있으면 원본이 현재 디렉터리를 쓰므로 `<CWD>`로 바꿔 저장했다.
-   `config/`: Config 픽스처(`-text`로 보관).
-   `ini/`: INI 파서 픽스처. 줄 끝과 BOM을 그대로 보관하도록 `.gitattributes`에서 `-text`로 지정했다.
-   테스트 파일(`*.rs`)은 해당 크레이트의 `Cargo.toml`에 `[[test]]`로 등록한다.

## 기준 출력 다시 수집하기

`tools/dotnet-oracle`은 TESTCore 빌드 결과(`TestCore.dll`)를 그대로 호출해 기준 출력을 만든다.

**TESTCore 저장소의 `bin/TestCore`는 예전 커밋으로 빌드된 것일 수 있다.** 기준 출력은 반드시 TESTCore HEAD로 빌드한 결과로 만든다. `build-testcore.ps1`이 HEAD 소스를 임시 디렉터리로 내보내(`git archive`, TESTCore는 건드리지 않음) 빌드하고 그 경로를 출력한다.

```powershell
$bin = pwsh tools/dotnet-oracle/build-testcore.ps1
dotnet build tools/dotnet-oracle -p:TestCoreBin=$bin
$env:TESTCORE_BIN = $bin
dotnet tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll ini tests/parity/ini/sample.ini > tests/parity/baseline/ini/sample.json
dotnet tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll config tests/parity/ini/sample.ini
pwsh tools/dotnet-oracle/gen-updown-cases.ps1   # UpDownClient 사례와 기준 출력
```

`78004ff` 빌드로 만든 기준 출력(`version.txt`, `help.txt`, ini·config·checksum·sign·uri·ksan·s3·json·stats)은 그 뒤 HEAD(`8d27dc2`)까지 바뀐 파일(`UpDownClient.cs`, `MultiSystemClient.cs`, `MultiSystemTest.cs`)과 관계없어 그대로 쓴다. `updown/`, `local/`, `multisystem/`은 HEAD 빌드로 만들었다.

Windows에서 .NET 콘솔은 파이프 출력에 시스템 코드 페이지(CP949)를 쓰므로 `TestCore.exe` 출력은 UTF-8로 바꿔서 수집한다(오라클은 UTF-8로 출력한다).

```powershell
[Console]::OutputEncoding = [Text.Encoding]::UTF8
TestCore.exe --help
```
