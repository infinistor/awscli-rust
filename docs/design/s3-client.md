# S3Client 설계

TESTCore `Client/S3Client.cs`(AWSSDK.S3 래퍼)를 `crates/s3/src/s3_client/`로 옮긴 방식과 .NET과의 차이를 정리한다.

## 구현 방식

-   `aws-sdk-s3`를 감싼다. 메서드 이름·인자는 원본 래퍼를 따르고, 응답은 SDK 출력 형식에 HTTP 상태 코드를 붙인 `S3Response<T>`로 돌려준다. 원본이 `response.HttpStatusCode`를 225곳에서 비교하기 때문이다.
-   오류는 `S3Error`로 바꾼다. 서비스 오류는 상태 코드, 오류 코드, 메시지를 보존하고, `dotnet_type()`과 `Display`는 원본 예외의 형식 이름과 메시지(`Error making request with Error Code ... No further error information was returned by the service.` 포함)를 따른다.

## .NET 설정과 맞춘 값

| 항목 | .NET | Rust |
| --- | --- | --- |
| 타임아웃 | `Timeout = 3600초` | `operation_attempt_timeout(3600초)` |
| 재시도 | `MaxErrorRetry = retryCount` | 표준 재시도, 최대 시도 `retryCount + 1` |
| 주소 | `ServiceURL`, `ForcePathStyle`, `UseHttp` | `endpoint_url`, `force_path_style`, 스킴 없으면 `http://` |
| 리전 | 사용자 주소면 서명 리전 `us-east-1`, 주소가 없으면 `ap-northeast-2` | 같음 |
| 응답 체크섬 | `WHEN_REQUIRED` | `WhenRequired` |
| 요청 체크섬 | `calculateRequestChecksum`이면 `WHEN_SUPPORTED`, 아니면 `WHEN_REQUIRED` | 같음 |
| 관리자 헤더 | `BeforeRequestEvent`에서 `x-ifs-backend`, `x-ksan-backend` 추가(서명 포함) | `modify_before_signing` 인터셉터 |
| 버킷 존재 확인 | `DoesS3BucketExistV2` = `GET /{bucket}?acl`, `NoSuchBucket`만 없음으로 판단 | 같음 |

## `UseChunkEncoding`

.NET SDK는 `UseChunkEncoding = true`(PutObject·UploadPart 기본값)면 청크마다 서명해 보낸다. 체크섬이 없으면 `STREAMING-AWS4-HMAC-SHA256-PAYLOAD`, 체크섬이 있으면 `STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER`다.

`aws-sdk-s3`는 체크섬 트레일러가 있는 스트림 본문일 때만 청크 서명을 쓴다. 그래서 다음과 같이 대체했다(사용자 결정, 2026-10-06).

-   `true`: 요청 체크섬을 `WhenSupported`로 바꾸고 본문을 스트림으로 보낸다. 결과는 CRC32 트레일러가 붙은 청크 서명(`...-PAYLOAD-TRAILER`)이다. .NET과 비교하면 청크 형식과 청크 서명 방식은 같고, 마지막에 `x-amz-checksum-crc32`와 `x-amz-trailer-signature` 두 줄이 더 붙는다.
-   `false`: `WhenRequired`로 본문 전체의 SHA256을 서명해 한 번에 보낸다. .NET과 같다.
-   체크섬 알고리즘을 지정한 업로드(`--checksum-type`)는 .NET처럼 항상 체크섬을 계산한다. 청크면 트레일러, 아니면 헤더로 보내며 .NET과 같은 형식이다.

## 비교하지 않는 차이

`tests/parity/s3_client.rs`에서 다음은 SDK 차이로 보고 비교하지 않는다. 서버의 요청 해석과 서명 검증에는 영향이 없다.

-   `User-Agent`, `x-amz-user-agent`, `amz-sdk-invocation-id`, `amz-sdk-request`
-   `Expect: 100-continue`(.NET만 보냄)
-   `x-amz-api-version: 2006-03-01`(.NET이 XML 본문 요청에 붙임)
-   서명 대상 목록의 `content-length`(Rust SDK만 서명에 포함)
-   쿼리 매개변수 순서

Rust SDK가 붙이는 `x-id=<연산 이름>` 쿼리 매개변수는 서명 전에 지워 .NET과 맞췄다.

## 기준 데이터

`tools/dotnet-oracle`의 `s3` 명령이 로컬 캡처 서버로 .NET `S3Client`의 요청을 모두 기록한다(`tests/parity/s3/`, `tests/parity/baseline/s3/`).
