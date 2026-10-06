# TESTCore .NET → Rust 전환 계획

TESTCore(`net10.0`, C# 약 26,600줄, 172개 파일)를 Rust 프로젝트 `awscli-rest`로 옮기기 위한 진행 방식과 Claude Code 모델 사용 기준을 정리한다.

-   원본: `E:\Code\Git\TESTCore` (.NET, 전환 기간 동안 기준 구현)
-   대상: `E:\Code\Git\awscli-rest` (<https://github.com/infinistor/awscli-rest>)
-   바이너리 이름: `awscli-rest` (기존 `TestCore`)

## 1. 기본 방향

-   **한 번에 다시 쓰지 않고 모듈 단위로 옮긴다.** 각 단계가 끝날 때마다 .NET 버전과 같은 결과가 나오는지 확인한 뒤 다음 단계로 넘어간다.
-   **외부 동작은 그대로 유지한다.** CLI 옵션 이름, `config.ini` 형식, 콘솔 출력, CSV·JSON 결과 형식, Controller·Worker HTTP 계약이 바뀌지 않아야 기존 스크립트와 Jenkins 작업을 그대로 쓸 수 있다. 바이너리 이름만 `TestCore`에서 `awscli-rest`로 바뀌므로 기존 스크립트는 실행 파일 이름만 바꾸면 된다.
-   **전환 기간에는 .NET 버전이 기준(oracle)이다.** 같은 입력으로 두 바이너리를 실행해 결과를 비교한다. 기능 추가는 전환이 끝날 때까지 .NET 쪽에서 멈추거나, 추가할 경우 Rust 작업 목록에도 같이 등록한다.
-   **Controller·Worker 계약(`Distributed/Contracts.cs`)을 JSON 수준에서 동일하게 유지한다.** 그러면 .NET Controller가 Rust Worker를 구동하는 혼합 구성으로 단계적으로 검증할 수 있다.

## 2. Claude 모델 선택

작업 성격에 따라 모델을 나눈다. 판단이 필요한 설계·검증은 상위 모델, 패턴이 정해진 대량 변환은 빠른 모델이 맡는다.

| 모델 | 맡길 작업 | 이유 |
| --- | --- | --- |
| **Opus 5.5** (`claude-opus-5-5`) | 전체 설계와 crate 구조, 모듈 경계·trait 설계, 동시성 모델(tokio) 결정, AWS SigV4·Chunked 서명, 분산 Controller·Worker, `CommandDispatcher` 분해, 최종 리뷰 | 소유권·수명·async 설계처럼 한 번 잘못 정하면 전체에 퍼지는 결정과 정확성이 중요한 코드 |
| **Sonnet 5.5** (`claude-sonnet-5-5`) | 설계가 정해진 뒤의 모듈 변환(Client, Test 시나리오, Config, 통계), 단위·비교 테스트 작성, 컴파일 오류·clippy 경고 수정 | 정해진 패턴을 따라 많은 코드를 빠르고 안정적으로 옮김 |
| **Haiku 4.5** (`claude-haiku-4-5-20251001`) | `Data/S3/My*.cs`, `Portal/Request·Response`, `Mover/*`, enum 같은 단순 DTO를 `serde` 구조체로 변환, 문서·주석 번역, 반복 수정 | 로직이 거의 없는 기계적 변환이라 비용·속도 이점이 큼 |

운영 방식:

-   **메인 세션은 Opus 5.5**로 두고, 단계 시작 시 Plan 모드로 설계를 확정한다.
-   확정된 설계에 따라 모듈별 변환은 **Sonnet 5.5 서브에이전트**에 맡긴다. 서로 독립적인 모듈(예: `Portal`, `Mover`, `Jenkins`)은 worktree를 나눠 병렬로 진행한다.
-   DTO 일괄 변환은 **Haiku 4.5 서브에이전트**에 맡기고, 결과는 Sonnet 또는 Opus가 컴파일과 테스트로 확인한다.
-   단계가 끝날 때마다 Opus 5.5로 `/code-review`를 실행해 정확성 문제를 확인한다.
-   서명, 체크섬, 분산 동기화처럼 결과가 틀려도 겉으로 드러나지 않는 영역은 처음부터 Opus 5.5가 작성한다.

## 3. 라이브러리 대응표

| .NET | Rust 후보 | 비고 |
| --- | --- | --- |
| `AWSSDK.S3`, `AWSSDK.Core` | `aws-sdk-s3`, `aws-config` | `force_path_style(true)` 필수. 최근 SDK는 기본으로 CRC32 체크섬을 계산하므로 `request_checksum_calculation`·`response_checksum_validation`을 .NET 동작에 맞춰 명시 |
| `AWSSDK.Extensions.CrtIntegration` | `aws-sdk-s3` 내장 체크섬 | CRC64NVMe는 SDK가 지원. 직접 계산은 `crc-fast` 등 사용 |
| `Signers/AWS4Signer*` (자체 구현) | 그대로 이식 (`hmac`, `sha2`, `hex`) | 표준 요청은 `aws-sigv4`로 대체 가능하지만 Chunked Upload·POST·Query 서명과 KSAN 확장 API 때문에 직접 이식이 안전. AWS 공식 테스트 벡터로 검증 |
| `HttpClient` (`KHttpClient`, `KsanClient`, `Portal`, `Jenkins`) | `reqwest` | 자체 서명 인증서 허용 여부를 기존과 동일하게 |
| ASP.NET Core (`Distributed/WorkerHost.cs`) | `axum` + `tokio` | 경로(`/driver`)와 JSON 필드 이름을 동일하게 유지 |
| `NetMQ` (`ZeroMqClient`) | `zeromq` (순수 Rust) 또는 `zmq` | `zmq`는 libzmq 네이티브 빌드가 필요해 Windows 배포가 번거로움 |
| `MySql.Data`, `Microsoft.Data.SqlClient` | `sqlx`(mysql) 또는 `mysql_async`, `tiberius` | 실제 사용 범위(`Test/UsedSizeTest.cs` 등)부터 확인 |
| `log4net` + `LogConfig.xml` | `tracing`, `tracing-subscriber`, `tracing-appender` | 로그 파일 위치·회전 정책을 기존과 맞춤 |
| `Mono.Options`, `Cli/CliOptionParser.cs` | `clap` 또는 기존 파서 직접 이식 | `--put-object`처럼 명령이 플래그 형태라 `clap` 서브커맨드와 맞지 않을 수 있음. 호환이 우선이면 기존 파서를 이식 |
| `Util/INIParser.cs` | 직접 이식 또는 `rust-ini` | 대소문자 처리, 중복 키, 주석 규칙이 기존과 같아야 함 |
| `System.Text.Json`, `Converter/*` | `serde`, `serde_json`, `chrono` 또는 `time` | 날짜 형식 변환기 동작 확인 |
| `System.IO.Hashing`, MD5 | `crc32fast`, `crc-fast`, `md-5`, `sha1`, `sha2` | |
| `Thread`, `Task`, Ctrl+C 처리 | `tokio`, `tokio-util::CancellationToken`, `tokio::signal` | 아래 5장 참고 |
| MSBuild Git 버전 정보 | `build.rs` (`git describe` 실행 또는 `vergen`) | 출력 형식 `태그_커밋수_해시` 유지 |
| `System.Security.Cryptography.Pkcs`, `System.Configuration.ConfigurationManager` | 불필요 예상 | 코드에서 직접 사용하는 곳이 없음. 전환 전에 제거 여부 확인 |

## 4. Cargo 구조

```text
awscli-rest/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ config/                 # awscli-rest-config: INI 파서, Config/*, UserData
│  ├─ model/                  # awscli-rest-model: Data/*, Portal·Mover DTO, 통계 구조체
│  ├─ s3/                     # awscli-rest-s3: Signers, 체크섬, S3Client·KHttpClient·KsanClient
│  ├─ clients/                # awscli-rest-clients: Local·Curl·ZeroMq·MultiSystem·UpDown 클라이언트, Portal·Jenkins·Mover
│  ├─ scenarios/              # awscli-rest-scenarios: Test/* 시나리오
│  ├─ distributed/            # awscli-rest-distributed: Controller, Worker(axum), ResultWriter
│  └─ cli/                    # awscli-rest-cli: main, 옵션 파서, 명령 디스패치 (바이너리 이름 awscli-rest)
└─ tests/parity/              # .NET 결과와 비교하는 통합 테스트
```

디렉터리 이름은 짧게 두고, 패키지 이름에는 `awscli-rest-` 접두사를 붙인다.

`Commands/CommandDispatcher.cs`(4,310줄)는 그대로 옮기지 않고 명령 그룹별 모듈(bucket, object, multipart, lifecycle, replication, ksan, test 등)로 나눈다.

## 5. 주의할 차이

-   **부하 특성:** .NET 스레드 수(`ThreadCount`)를 tokio 태스크 수로 바꾸면 동시 요청 수와 지연 측정 방식이 달라질 수 있다. 같은 대상·설정에서 .NET과 Rust의 처리량·지연 분포를 비교해 의미가 같은지 확인한다. 블로킹 파일 I/O는 `spawn_blocking` 또는 `tokio::fs`로 처리한다.
-   **시간 측정:** `TimeWatcher`, `UpDownStats`의 측정 구간(요청 시작~응답 본문 수신 완료 등)을 정확히 맞춘다. 통계가 달라지면 기존 결과와 비교할 수 없다.
-   **종료 처리:** 최근 커밋에서 보완한 종료 플래그와 다중 시스템 Ctrl+C 처리를 `CancellationToken` 하나로 통일하고, 목록 순회·재시도 루프마다 취소를 확인한다.
-   **SDK 기본값:** 재시도 횟수, 타임아웃, 체크섬, `Expect: 100-continue`, Chunked 인코딩 사용 여부가 .NET SDK와 다르다. 요청을 캡처해 헤더를 비교한다.
-   **오류 처리:** .NET 예외 메시지를 그대로 출력하던 부분은 Rust 오류 타입(`thiserror`, 바이너리에서는 `anyhow`)으로 바꾸되, 사용자에게 보이는 메시지와 종료 코드는 유지한다.
-   **배포:** Windows·Linux 단일 바이너리로 배포할 수 있다. `zmq`처럼 네이티브 라이브러리가 필요한 crate는 피하거나 정적 링크를 확인한다.

## 6. 진행 단계

| 단계 | 내용 | 주 모델 | 완료 기준 |
| --- | --- | --- | --- |
| 0. 준비 | 기능 목록 정리, .NET 출력 기준 데이터 수집(명령별 콘솔 출력, CSV·JSON, 요청 헤더), 테스트용 S3(MinIO 또는 사내 KSAN) 준비, 미사용 패키지 정리 | Opus | 비교 기준 데이터와 실행 스크립트 확보 |
| 1. 골격 | workspace·crate 생성, 공통 오류 타입, 로깅, `build.rs` 버전 정보, CI(`cargo fmt`, `clippy -D warnings`, `test`) | Opus | 빈 CLI가 빌드되고 `--version` 출력 형식(`태그_커밋수_해시`)이 같음 |
| 2. 기반 | INI 파서, Config, DTO, Utility, 체크섬, AWS4 서명 | Opus(서명·체크섬), Sonnet(Config), Haiku(DTO) | 같은 `config.ini` 파싱 결과 일치, 서명 테스트 벡터 통과, 체크섬 값 일치 |
| 3. 클라이언트 | S3Client, KHttpClient, KsanClient, Local·Curl·ZeroMq·MultiSystem·UpDown 클라이언트, Portal·Jenkins·Mover | Sonnet (S3Client·KsanClient는 Opus 리뷰) | 같은 요청에 대해 응답 처리 결과와 요청 헤더가 동일 |
| 4. CLI·명령 | 옵션 파서, 명령 디스패치 모듈화, 도움말 출력 | Opus(구조), Sonnet(명령별 이식) | 모든 명령의 도움말·출력이 기준 데이터와 일치 |
| 5. 테스트 시나리오 | `Test/*` (UpDown, MultiPart, Lifecycle, Replication, Compare, UsedSize 등) | Sonnet | 각 시나리오의 결과 요약과 성공·실패 판정 일치 |
| 6. 분산 | WorkerHost(axum), Controller, RunControl, ResultWriter, 진단 | Opus | .NET Controller + Rust Worker, Rust Controller + .NET Worker, Rust 단독 구성 모두 동작 |
| 7. 성능·전환 | 처리량·지연·메모리 비교, 문서(README, DISTRIBUTED) 갱신, .NET 코드 보관 | Opus | 같은 부하에서 통계 차이가 허용 범위 안이고 운영 절차 문서화 완료 |

## 7. Claude Code 작업 방식

1.  저장소의 `CLAUDE.md`에 공통 규칙을 적는다: crate 구조, 오류 처리 방식, 출력 호환 원칙, 커밋 메시지 형식(기존과 같이 한국어), 매 작업 후 `cargo fmt && cargo clippy -- -D warnings && cargo test` 실행.
2.  단계마다 Opus 5.5 세션에서 Plan 모드로 대상 C# 파일을 읽고 Rust 설계(타입, trait, 모듈 경계)를 먼저 확정한다.
3.  모듈 하나를 옮길 때는 대상 C# 파일과 확정된 설계, 비교 기준 데이터를 함께 지정해 Sonnet 5.5에 맡긴다. 예시 요청:
    ```text
    TESTCore의 Client/KsanClient.cs를 crates/s3/src/ksan.rs로 이식해줘.
    설계는 docs/rust-design.md의 KsanClient 절을 따르고, 응답 XML 파싱 결과가
    tests/parity/ksan/ 기준 데이터와 같은지 테스트를 추가해줘.
    ```
4.  변환 결과는 반드시 컴파일, 단위 테스트, 비교 테스트를 통과한 뒤 커밋한다. 모듈 하나당 커밋 하나를 원칙으로 한다.
5.  단계가 끝나면 Opus 5.5로 `/code-review high`를 실행하고, 지적 사항을 반영한 뒤 다음 단계로 넘어간다.
6.  큰 파일(`CommandDispatcher.cs`, `UpDownTest.cs`, `UpDownClient.cs`, `INIParser.cs`)은 한 번에 옮기지 말고 기능 단위로 나눠 요청한다.

## 8. 위험과 대응

| 위험 | 대응 |
| --- | --- |
| 통계 측정 방식이 바뀌어 기존 성능 결과와 비교 불가 | 측정 구간을 문서화하고 같은 환경에서 두 버전을 나란히 실행해 확인 |
| 서명·체크섬 오류가 일부 경우에만 발생 | 테스트 벡터와 실제 서버 요청을 모두 검증하고 Opus가 작성·리뷰 |
| 전환 중 .NET 쪽 기능 추가로 범위가 계속 늘어남 | 전환 기간 기능 동결 또는 양쪽 동시 반영 규칙 적용 |
| 분산 계약 불일치로 혼합 구성 실패 | `Contracts.cs` JSON 예시를 기준 데이터로 저장하고 직렬화 결과를 비교 테스트 |
| 사용되지 않는 기능까지 옮기는 비용 | 0단계에서 실제 사용 명령을 확인하고, 쓰지 않는 기능은 이식 대상에서 제외 |
