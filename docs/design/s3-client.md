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

`tools/dotnet-oracle`의 `s3` 명령이 로컬 캡처 서버로 .NET `S3Client`의 요청을 모두 기록한다(`tests/parity/s3/`, `tests/parity/baseline/s3/`). 사례 JSON의 `routes`(요청 줄에 `contains`가 들어 있으면 해당 응답)로 멀티파트처럼 요청마다 응답이 다른 흐름도 재현한다. `Program.cs`의 `S3Probe`가 기본 op를, `S3Ops.cs`가 나머지 op(설정 객체는 실제 쓰는 값)를 맡고, Rust 쪽은 `tests/parity/support/s3_ops.rs`가 같은 이름·같은 값으로 호출한다.

## 모듈 구성

| 모듈 | 내용 |
| --- | --- |
| `bucket.rs` | 목록, 생성, 버전 관리, 존재 확인 |
| `bucket_config.rs` | ACL, 소유권, 위치, 로깅, 알림, CORS, 태그, 수명 주기, 정책, 객체 잠금, 퍼블릭 액세스 차단, 암호화, 웹사이트, 인벤토리, 메트릭, 분석 |
| `object.rs` | 업로드·다운로드·목록·삭제 기본 연산, `PutBody` |
| `object_config.rs` | 복사, 객체 ACL, 태그, 보존, 법적 보존, 복제, 복원, 버전 목록 |
| `multipart.rs` | 시작, 파트(파일 구간·복사), 완료, 중단, 목록 |
| `transfer.rs` | `Upload`·`Download`(`TransferUtility`) |
| `presign.rs` | `GeneratePresignedURL` |
| `mime.rs` | `AmazonS3Util.MimeTypeFromExtension` 표(173개, 리플렉션으로 추출) |

`GetObjectAttributes`는 원본에서도 쓰이지 않아 옮기지 않았다.

## .NET 요청과 맞추려고 덧붙인 것

Rust SDK가 기본으로 보내지 않지만 .NET이 항상 보내는 헤더는 `customize().mutate_request`로 서명 전에 넣는다.

-   `Content-MD5`: 인벤토리·메트릭·분석 설정 저장, `CompleteMultipartUpload`.
-   `x-amz-checksum-crc32` + `x-amz-sdk-checksum-algorithm: CRC32`: `PutBucketNotification`(Rust SDK의 알림 설정 요청은 체크섬 설정이 없다).
-   `Content-Length: 0`: 본문 없는 `CreateMultipartUpload`, `UploadPartCopy`, `CopyObject`.
-   `x-amz-metadata-directive: COPY`: `CopyObject`.
-   `Content-Type: text/plain`: 모든 `UploadPart`(파일·스트림 모두).
-   `x-amz-copy-source`: `{버킷}/{키}` 전체를 RFC 3986으로 인코딩(`/`도 `%2F`), 버전 ID는 `/`, `+`만 남기고 인코딩.

`Content-Type` 기본값은 AWSSDK의 확장자 표(`mime.rs`)를 따른다: `PutObject`는 파일이면 파일 경로, 아니면 키의 확장자로 정하고 확장자가 없으면 `text/plain`; `CreateMultipartUpload`는 키의 확장자로 정하고 없으면 헤더를 보내지 않는다; `TransferUtility`는 파일이면 파일 확장자(없으면 `application/octet-stream`).

## .NET 응답·예외와 맞춘 것(오라클로 확인)

-   본문이 빈 200 응답: 목록 연산(`ListBuckets`, `ListDirectoryBuckets`, `ListObjects`(V2), `ListVersions`, `ListMultipartUploads`, `ListParts`, 인벤토리·메트릭·분석 목록)은 `send!(..., empty_body = "루트")`로 빈 결과(목록은 `None` = .NET `null`)로 읽는다.
-   `DeleteObjects` 응답에 `<Error>`가 있으면 `S3Error::DeleteObjects`(`Amazon.S3.DeleteObjectsException`, `Error deleting objects. Deleted objects: N. Delete errors: M`, `StatusCode` 0, `ErrorCode` 없음). `AmazonS3Exception`의 하위 형식이므로 `is_amazon_s3_exception()`이 참이다.
-   `UploadId`가 빈 값: `UploadPart`·`CopyPart`는 `uploadId` 쿼리 없이 보낸다(.NET은 `null`을 쿼리에 넣지 않는다). `CompleteMultipartUpload`·`AbortMultipartUpload`·`ListParts`는 요청 없이 `AmazonS3Exception`(`Request object does not have required field UploadId set`, `StatusCode` 0).
-   `PartETag::new(번호, Option<&str>)`: 응답에 ETag가 없으면 완료 요청 XML에서 `<ETag>`를 뺀다.
-   키가 `/`로 시작하면 요청 경로에서 그 `/` 하나를 뺀다(`/a` → `/버킷/a`, `//a` → `/버킷//a`, 서명된 URL 포함). `x-amz-copy-source`는 키를 그대로 쓴다. 클라이언트 인터셉터 `TrimKeySlash`.
-   빈 버킷 이름: `DoesS3BucketExist("")`는 `GET /?acl`을 보낸다. 경로 방식 주소에서만 자리표시 버킷 이름으로 요청을 만들고 서명 전에 경로에서 지운다. `PutBucket("")`는 요청 없이 `ArgumentException`.
-   `ListVersions`: .NET은 `Version`과 `DeleteMarker`를 문서 순서로 한 목록(`Versions`)에 담는다. `list_versions`는 응답 본문에서 순서를 기록한 `ListVersions`(SDK 출력으로 `Deref`)를 돌려주고, `entries()`가 그 순서의 `VersionEntry` 목록(둘 다 없으면 `None`)을 만든다.

## `TransferUtility` 동작(캡처로 확인)

-   크기 < `partSize`: `PutObject` 한 번(청크 서명). 크기 >= `partSize`: 멀티파트(크기가 `partSize`와 같아도 파트 1개짜리 멀티파트).
-   파트 크기 = `max(partSize, ceil(크기 / 10000))`. 크기 100010, `partSize` 10이면 11바이트 9091개 + 9바이트 1개.
-   파트가 실패하면 `AbortMultipartUpload` 후 원래 오류를 던진다.
-   빈 파일은 청크 서명 없이 한 번에 보낸다.
-   파일 경로와 스트림을 함께 주면 `ArgumentException`("Please specify one of either an InputStream or a FilePath ..."). `S3Error::Argument`로 옮겼다.
-   `Download`: `GetObject` 한 번. 덮어쓰고, 없는 디렉터리는 만들고, 오류 응답이면 기존 파일을 건드리지 않는다. ETag가 32자리 16진수면 내용의 MD5와 비교하고 다르면 파일을 남긴 채 `Expected hash not equal to calculated hash`(`AmazonClientException`).

## `GeneratePresignedURL`

-   `Protocol.HTTP`이므로 항상 `http://`. 관리자 헤더는 서명하지 않는다(.NET은 `BeforeRequestEvent`를 거치지 않으므로 인터셉터 없는 별도 클라이언트로 서명한다).
-   `X-Amz-Expires` = `Expires`의 초 - 현재 시각의 초(소수 버림).
-   만료가 7일을 넘으면 .NET은 서명 V2(`AWSAccessKeyId`, `Expires`, `Signature`)를 만든다. HMAC-SHA1로 같은 방식으로 직접 계산했고, 고정 시각 사례(`presign-v2-*`)는 서명까지 같다.
-   다른 점: 이미 지난 시각은 .NET이 음수 `X-Amz-Expires`로 URL을 만들지만 Rust는 오류. 사용자 주소가 없을 때의 V2 URL은 `http://{버킷}.s3.ap-northeast-2.amazonaws.com/...` 형식(검증하지 못함). `DateTime.Kind`가 `Unspecified`면 .NET은 로컬 시간으로 보지만 Rust는 항상 UTC.

## 요청 본문 비교 규칙(`tests/parity/s3_client.rs`)

-   XML 본문은 SDK마다 요소 순서(.NET 이름순, Rust SDK 모델 순서)가 달라 바이트 비교가 실패하면 형제 요소를 이름순(같은 이름은 원래 순서 유지)으로 정렬해 다시 비교한다. 이때 본문에서 계산되는 `Content-Length`, `x-amz-content-sha256`, `x-amz-checksum-crc32`, `Content-MD5`는 각 요청의 본문과 맞는지 검증한 뒤 자리표시자로 바꾼다. 이 방식으로 비교한 사례는 테스트 출력(`--nocapture`)에 표시된다.
-   `UseChunkEncoding = true` 업로드가 섞인 사례(`STRIP_TRAILER_CASES`)는 Rust 요청에서 CRC32 트레일러 두 줄, `x-amz-trailer`, `x-amz-sdk-checksum-algorithm`을 걷어내고(`Content-Length`와 `x-amz-content-sha256`는 트레일러 없는 형식으로 바꿔) 비교한다.
-   `upload-multi-parallel`은 동시 업로드라 요청 줄 순으로 정렬해 비교한다.
-   `copy-part`는 .NET이 본문 없는 요청에 붙이는 `Content-Type: application/x-amz-json-1.0`을 비교하지 않는다.
-   서명된 URL은 경로, 쿼리 이름, 값(서명·`X-Amz-Date`·`X-Amz-Credential`의 날짜는 제외, `X-Amz-Expires`는 ±1초)을 비교한다.

## 원본에서 수상한 부분(옮기지 않고 그대로 둠)

-   `S3Client.cs:681` `PutBucketReplication`은 `token` 인자를 받고 쓰지 않는다.
-   `S3Client.cs:803-823` `Upload`는 `filePath`가 있으면 항상 `request.FilePath`에 넣어서 `body`/`byteBody`와 함께 줄 수 없다(`ArgumentException`). `filePath`를 `null`로 넘겨야만 스트림 업로드가 된다.
-   `S3Client.cs:801` `MinSizeBeforePartUpload = partSize`라서 크기가 `partSize`와 같은 파일이 파트 1개짜리 멀티파트로 올라간다.
-   `S3Client.cs:718-719` `UploadPart`의 `if (useChunkEncoding) ... = true; else ... = false;`는 대입 한 줄과 같다.
-   `S3Client.cs:638` `PutObjectRetention`만 `bypass` 기본값이 `false`이고(다른 메서드는 `null`), `== true`로만 쓰이므로 결과는 같다.
-   `S3Client.cs:578` `GetObjectAttributes`는 쓰이지 않는다.
