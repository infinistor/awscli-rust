# 운영 절차

awscli-rust의 빌드·배포, TESTCore에서의 전환, 분산 실행 교체 순서, 성능 비교, 롤백, 기준 출력 갱신 절차를 정리한다.

## 빌드와 배포

### Windows

PowerShell에서 실행한다(Git Bash의 `/usr/bin/link`가 MSVC `link.exe`를 가려 링크가 실패한다). Rust stable과 Visual Studio Build Tools의 C++ 워크로드가 필요하다.

```powershell
cargo build --release
```

산출물은 `target\release\awscli-rust.exe`다. 이 파일 하나와 `config.ini`만 옮기면 되고 .NET 런타임은 필요 없다.

### Linux (정적 musl 바이너리)

Docker Desktop이 있는 Windows에서 `pwsh tools/perf/build-linux.ps1`을 실행하면 `rust:alpine` 컨테이너에서 `x86_64-unknown-linux-musl` 정적 바이너리를 만든다(성능 비교 묶음에 들어가며 `target/linux/release/awscli-rust`에 남는다). 대상 장비의 glibc 버전과 무관하게 실행되고 런타임 설치도 필요 없다. 컨테이너 안에서 직접 만들 때는 다음과 같다.

```bash
apk add --no-cache musl-dev gcc
cargo build --release --target x86_64-unknown-linux-musl
```

배포는 바이너리를 복사하고 실행 권한(`chmod +x awscli-rust`)을 주면 끝이다.

### 변경 후 검증

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## TESTCore에서 전환

-   `config.ini`와 명령행 인자는 TESTCore와 같다. 설정 파일은 그대로 쓰고, 실행 파일 이름만 `TestCore`에서 `awscli-rust`로 바꾼다.
-   `TestCore`(또는 `dotnet TESTCore.dll`)를 부르는 스크립트·작업 정의(Jenkins 등)를 `awscli-rust`로 바꾼다.
-   콘솔 출력, CSV·JSON 결과 파일, 종료 코드를 읽는 후처리는 그대로 동작한다.
-   원본과 다른 점은 [README의 알려진 차이](../README.md#알려진-차이)를 확인한다.

## 분산 실행 전환 순서

.NET과 Rust 구현은 Controller·Worker HTTP 계약이 같고 혼합 구성이 검증되어 있다([DISTRIBUTED.md](../DISTRIBUTED.md#혼합-구성)). 따라서 한 번에 바꾸지 않아도 된다.

1.  Worker를 하나씩 awscli-rust로 교체한다. 교체한 Worker는 `--worker -c worker.ini`로 다시 띄우고, Worker 설정(`worker.sample.ini`)은 그대로 쓴다. 교체할 때마다 .NET Controller로 짧은 Prepare·Get을 실행해 확인한다.
2.  모든 Worker를 교체한 뒤 Controller를 awscli-rust로 바꾼다(`--controller`, 설정 `controller.sample.ini`).
3.  문제가 생기면 해당 Worker·Controller만 .NET으로 되돌린다.

Worker는 한 번에 하나의 실행만 처리하므로, 교체는 진행 중인 실행이 없을 때 한다.

## 성능 비교 다시 하기

TESTCore와 awscli-rust를 같은 KSAN·같은 부하로 번갈아 실행해 처리량·평균 지연·최대 메모리를 비교한다.

1.  묶음을 만든다(Docker Desktop 필요). `-Ref`로 비교할 TESTCore 커밋을 고를 수 있다(기본 `HEAD`).
    ```powershell
    pwsh tools/perf/build-linux.ps1
    ```
    결과는 `dist/perf-bundle.tar.gz`(awscli-rust 정적 바이너리, self-contained TESTCore, `compare.sh`, `README.txt`)다.
2.  묶음을 클라이언트 테스트 장비에 올리고 푼다.
3.  묶음 디렉터리에 설정 파일 두 개를 둔다: `testcore.ini`, `awscli-rust.ini`. 접속 정보(`[Main User]` URL·AccessKey·SecretKey)와 `[Default] BucketName`만 쓰며 부하 값은 `compare.sh`가 실행용 사본에서 덮어쓴다. **자격 증명이 들어 있으므로 저장소에 커밋하지 않는다**(`ksan.ini`는 `.gitignore`에 있다).
4.  실행한다.
    ```bash
    ./compare.sh --net-ini testcore.ini --rs-ini awscli-rust.ini
    ```
    기본값은 3회, 스레드 32, 60초, 크기 1M, 파일 32개이며 `--rounds`, `--threads`, `--times`, `--size`, `--files`로 바꾼다. 버킷은 도구별로 `{BucketName}-perf-net`·`-perf-rs`를 만들고 끝나면 비워서 지운다.
5.  결과는 `results/summary.md`(표와 ±10% 판정), `results/runs.csv`(원시 값), `results/logs/`에 남는다.

.NET 쪽은 self-contained로 게시되어 런타임은 필요 없지만, Linux에서 `libicu`와 `libssl`이 설치되어 있어야 실행된다. awscli-rust는 정적 바이너리라 추가 라이브러리가 필요 없다.

## 롤백 (TESTCore 다시 빌드)

문제가 있으면 .NET TESTCore로 되돌린다. 전환 직전 최종 .NET 소스는 TESTCore 저장소의 태그 `dotnet-final`이다. 다음 명령은 해당 태그의 소스를 `git archive`로 임시 디렉터리에 내보내(TESTCore 작업 디렉터리는 건드리지 않는다) Release로 빌드하고 `TestCore.dll`이 있는 경로를 출력한다.

```powershell
pwsh tools/dotnet-oracle/build-testcore.ps1 -Ref dotnet-final
```

-   `-Ref`는 TESTCore의 빌드할 커밋·태그다(생략하면 `HEAD`). `-TestCore`로 저장소 경로, `-Out`으로 빌드 위치를 바꿀 수 있다.
-   출력 경로의 파일을 배포하고, 스크립트의 실행 파일 이름을 `TestCore`로 되돌린다. 설정 파일은 그대로 쓴다.
-   Linux용 self-contained 묶음은 `pwsh tools/perf/build-linux.ps1 -Ref dotnet-final`로 만든다.

## 기준 출력 갱신

TESTCore 쪽이 바뀌었거나 새 시나리오를 추가했을 때 parity 기준 출력을 다시 만드는 방법은 [tests/parity/README.md](../tests/parity/README.md)의 "기준 출력 다시 수집하기"를 따른다. 기준 출력은 항상 TESTCore HEAD(또는 비교하려는 커밋)로 빌드한 결과로 만들고, .NET과 섞어 도는 테스트는 `TESTCORE_BIN` 환경 변수에 그 빌드 경로를 지정한다.
