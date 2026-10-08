# 테스트 시나리오 (`crates/scenarios`)

TESTCore `Test/*.cs`를 옮긴 크레이트다. 원본 클래스 하나를 모듈 하나(`src/<이름>.rs`)로 옮겼다. 메뉴 연결은 `crates/cli/src/dispatch/tests/`의 묶음별 모듈이 맡는다.

| 묶음(cli 모듈) | 시나리오 모듈 | 원본 |
| --- | --- | --- |
| `up_down` | `up_down`(+`up_down/save.rs`), `clear` | UpDownTest, FullTest, ClearTest |
| `system` | `local`, `multi_system`, `access_ips`, `used_size` | LocalTest, MultiSystemTest, AccessIpsTest, UsedSizeTest |
| `compare` | `compare`, `copy`, `duplicate`, `lifecycle`, `mover` | CompareTest, CopyTest(RangeReadCopy), DuplicateTest, LifecycleTest, MoverTest |
| `transfer` | `multi_part`, `multi_upload`, `range_read`, `find_tag`, `multi_download`, `io` | MultiPartTest(ManualUpload), MultiUploadTest, RangeReadTest, FindTagTest, MultiDownloadTest, IoTest |

옮기지 않은 것: `ReplicationTest`(디스패처에 case가 없다), `ListObjTest`(생성하는 곳이 없다. `--test-list-obj`는 `UpDownTest.ListObjectTest`).

## 공용 모듈

-   `error::ScenarioError`: .NET 예외 형식 이름과 메시지. cli의 `CommandError`는 이 형식을 다시 내보낸 것이다. 클라이언트 오류(`S3Error`, `UpDownError`, `LocalError`, `PortalError`, `UtilError`, `io::Error`)에서 변환한다.
-   `runner::TestTasks`: 원본 `_testList`·`_taskList`와 `TaskStart`·`TaskCheck`·`TestStop`·`JoinTasks`. 작업 안에서 난 `Err`는 원본처럼 프로세스를 끝낸다(사용자 결정). 이때 stderr에 `Unhandled exception. 형식: 메시지`를 쓰고 원본 종료 코드(`common::dotnet_exit`)로 끝낸다. 동기 클라이언트는 `add_blocking`으로 돌린다.
-   `shutdown::activate`: 원본 `Console.CancelKeyPress`를 옮긴 것이다. Ctrl+C는 활성 테스트의 토큰만 취소한다.
    -   UpDownTest는 처리기가 남고(`Persistent`), MultiSystemTest는 실행 동안만 건다(`Scoped`).
    -   처리기가 없으면 원본처럼 `STATUS_CONTROL_C_EXIT`로 끝낸다.
    -   클라이언트의 `QuitFlag`는 테스트 토큰의 자식(`with_quit`)이다.
-   `util`: 원본 `Utility` 중 다른 크레이트에 없던 것(`GetDummyFileName`, `SanitizeFileName`, `RandomText`, `CreateBucket`, `GetNewBucket`, `CompareDirMD5`, `GetETag`, `GetMD5HexFromString`).
-   `files`, `input`: 파일 입출력과 `File.ReadAllText`, 자주 나는 .NET 예외.

## 원본 동작 규칙

-   원본 `S3Client`는 `GetAwaiter().GetResult()`로 기다려 예외를 그대로 던진다. 그래서 `catch (AggregateException)` 블록은 실행되지 않고, `catch (Exception e) { log.Error(e); }`가 `형식: 메시지`를 남긴다.
-   AWSSDK v4는 빈 목록을 `null`로 둔다. 원본이 `.Count`·`.Select`를 부르는 자리에서는 `NullReferenceException`이나 `ArgumentNullException (Parameter 'source')`를 같은 자리에서 낸다.
-   시나리오마다 그대로 둔 원본 특이점은 각 모듈의 문서 주석에 적었다.

### TESTCore에서 고친 원본 버그 (`c83e35f`, 사용자 결정)

기능을 쓸 수 없게 만들던 4건은 원본과 이식본을 함께 고쳤다. 기준 출력은 이 커밋으로 만든 빌드에서 만든다.

1.  CompareTest·FindTagTest: `ListObjects(bucket, nextMarker)`가 marker를 prefix로 보냈다. `marker:`로 고쳤다.
2.  MultiDownloadTest: 다운로드 경로를 로컬 파일 MD5로 계산했다. `"버킷/키"` 문자열 MD5로 고쳤다.
3.  AccessIpsTest: 접근 IP를 해제할 때 IP를 `BucketName`으로 보냈다. 사용자 단위로 해제하도록 고쳤다.
4.  MultiSystemTest `ListCore`: 기대 이름 계산과 개수 집계를 고쳤다.

## 비교 테스트

-   사례: `tests/parity/cli/run/scenarios/<시나리오>/*.json`. 사례 옵션(`stats`, `ignore_body`, `drop_lines`, `dirs`, `outputs`, `delays`, `mask`, `db_rows`)은 [cli-dispatch.md](cli-dispatch.md)에 있다.
-   UsedSizeTest는 가짜 MySQL 서버(`tests/parity/support/fake_mysql.rs`)로 사용량 조회까지 비교한다. DB는 `mysql_async`(TLS 없음)로 붙는다. .NET은 `SslMode=Preferred`다.
-   시간으로 끝나는 시나리오는 `Times=1`과 응답 지연(`delays`)으로 요청 수를 정한다. 통계 숫자는 가린다.
-   MultiDownloadTest의 65,536개 디렉터리 미리 만들기는 사례에서 첫 디렉터리가 바로 실패하게 해 건너뛴다. 생성 로직은 단위 테스트로 확인한다.

## 알려진 차이

-   CompareTest 메타데이터 비교: .NET은 서버가 보낸 `x-amz-meta-*` 헤더 이름의 대소문자를 그대로 두고 이름의 서수(ordinal) 순서로 비교한다. Rust(hyper)는 헤더 이름을 소문자로 바꾸므로, 대문자가 섞인 메타데이터 헤더는 로그의 이름과 비교 순서가 다를 수 있다(S3는 소문자로 보낸다). 받은 대소문자를 얻으려면 HTTP 전송을 바꿔야 해서 그대로 두었다.
-   빈 버킷 이름: `DoesS3BucketExist`(`GET /?acl`, 경로 방식 주소일 때만)와 `PutBucket`(`ArgumentException`)만 .NET과 맞췄다. 그 밖의 연산은 빈 버킷으로 부르는 곳이 없어 확인하지 않았다.

S3 클라이언트를 처음 옮길 때 피했던 차이(`HeadObject`의 `null` 버전 ID, ETag 없는 파트, `UploadId` 없는 시작 응답, 본문이 빈 목록 응답, `DeleteObjectsException`, `/`로 시작하는 키, 빈 버킷 이름의 `GET /?acl`, 버전·삭제 마커의 문서 순서, `ContentLength` 없음)는 고쳤다. 동작은 [s3-client.md](s3-client.md)의 ".NET 응답·예외와 맞춘 것"에 있고, 사례는 이름에 `sdk-`가 붙은 `cli/run` 사례와 `s3/`의 `*-leading-slash`·`*-empty-*`·`delete-objects-errors`다.
