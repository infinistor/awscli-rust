//! TESTCore `Data/Usage.cs`: 옵션 이름, 오류 문구, 메뉴별 사용법 문자열.
//!
//! 상수 값은 원본 빌드에서 리플렉션으로 뽑은 값(`DotnetOracle cli-usage`, `tests/parity/baseline/cli/usage.json`)을
//! 그대로 옮겼다. 원본의 조합 문자열(`Config.USAGE_*` 연결, `SubFlag` 호출 결과)도 최종 값으로 둔다.
//! 원본 버그(`USAGE_BUCKET_NAME`이 설명을 형식 자리에 넘김)도 그대로다.
#![allow(dead_code)]

/// 원본 `MainFlag(value, summary)`.
pub fn main_flag(value: &str, summary: &str) -> String {
    format!("--{value}{summary}\n")
}

/// 원본 `MainFlagValue(value, type = "string", summary)`.
pub fn main_flag_value(value: &str, type_name: &str, summary: &str) -> String {
    format!("--{value} <{type_name}>{summary}\n")
}

/// 원본 `SubFlag(value, type = "string", summary)`.
pub fn sub_flag(value: &str, type_name: &str, summary: &str) -> String {
    format!("\t--{value} <{type_name}>{summary}\n")
}

/// 원본 `Optional(value)`, `Optional(value, summary)`.
pub fn optional(value: &str, summary: &str) -> String {
    format!("\t[--{value}]{summary}\n")
}

/// 원본 `OptionalValue(value, type = "string", summary)`.
pub fn optional_value(value: &str, type_name: &str, summary: &str) -> String {
    format!("\t[--{value} <{type_name}>]{summary}\n")
}

pub const USAGE_BUCKET_NAME: &str = "\t--bucket < : 버킷명>\n";
pub const USAGE_THREAD_COUNT: &str = "\t--thread <int> : 스레드 개수\n";
pub const USAGE_SIZE: &str = "\t--size <string> : 파일 크기\n";
pub const USAGE_FILE_COUNT: &str = "\t--count <int> : 파일 개수\n";
pub const USAGE_TIMES: &str = "\t--times <int> : 테스트 수행 시간(초)\n";
pub const USAGE_CONFIG: &str = "\t[--config <string>] : 설정 파일 경로. default = config.ini\n";
pub const USAGE_PREFIX: &str = "\t[--prefix <string>] : 객체명 Prefix.\n";
pub const USAGE_PATH: &str = "\t[--path <string>] : temp 파일 경로. default = ./test/\n";
pub const USAGE_START: &str = "\t[--start <int>] : 시작 번호\n";
pub const ABORT_MULTIPART_UPLOAD: &str = "abort-multipart-upload";
pub const COMPLETE_MULTIPART_UPLOAD: &str = "complete-multipart-upload";
pub const COPY_OBJECT: &str = "copy-object";
pub const CREATE_BUCKET: &str = "create-bucket";
pub const CREATE_MULTIPART_UPLOAD: &str = "create-multipart-upload";
pub const DELETE_BUCKET: &str = "delete-bucket";
pub const DELETE_BUCKET_ANALYTICS: &str = "delete-bucket-analytics";
pub const DELETE_BUCKET_CORS: &str = "delete-bucket-cors";
pub const DELETE_BUCKET_ENCRYPTION: &str = "delete-bucket-encryption";
pub const DELETE_BUCKET_INVENTORY: &str = "delete-bucket-inventory";
pub const DELETE_BUCKET_LIFECYCLE: &str = "delete-bucket-lifecycle";
pub const DELETE_BUCKET_METRICS: &str = "delete-bucket-metrics";
pub const DELETE_BUCKET_OWNERSHIP_CONTROLS: &str = "delete-bucket-ownership-controls";
pub const DELETE_BUCKET_POLICY: &str = "delete-bucket-policy";
pub const DELETE_BUCKET_REPLICATION: &str = "delete-bucket-replication";
pub const DELETE_BUCKET_TAGGING: &str = "delete-bucket-tagging";
pub const DELETE_BUCKET_WEBSITE: &str = "delete-bucket-website";
pub const DELETE_OBJECT: &str = "delete-object";
pub const DELETE_OBJECT_TAGGING: &str = "delete-object-tagging";
pub const DELETE_OBJECTS: &str = "delete-objects";
pub const DELETE_PUBLIC_ACCESS_BLOCK: &str = "delete-public-access-block";
pub const GET_BUCKET_ACL: &str = "get-bucket-acl";
pub const GET_BUCKET_ANALYTICS: &str = "get-bucket-analytics";
pub const GET_BUCKET_CORS: &str = "get-bucket-cors";
pub const GET_BUCKET_ENCRYPTION: &str = "get-bucket-encryption";
pub const GET_BUCKET_INVENTORY: &str = "get-bucket-inventory";
pub const GET_BUCKET_LIFECYCLE: &str = "get-bucket-lifecycle";
pub const GET_BUCKET_LOGGING: &str = "get-bucket-logging";
pub const GET_BUCKET_METRICS: &str = "get-bucket-metrics";
pub const GET_BUCKET_LOCATION: &str = "get-bucket-location";
pub const GET_BUCKET_NOTIFICATION: &str = "get-bucket-notification";
pub const GET_BUCKET_OWNERSHIP_CONTROLS: &str = "get-bucket-ownership-controls";
pub const GET_BUCKET_POLICY: &str = "get-bucket-policy";
pub const GET_BUCKET_POLICY_STATUS: &str = "get-bucket-policy-status";
pub const GET_BUCKET_REPLICATION: &str = "get-bucket-replication";
pub const GET_BUCKET_TAGGING: &str = "get-bucket-tagging";
pub const GET_BUCKET_VERSIONING: &str = "get-bucket-versioning";
pub const GET_BUCKET_WEBSITE: &str = "get-bucket-website";
pub const GET_OBJECT: &str = "get-object";
pub const GET_OBJECT_ACL: &str = "get-object-acl";
pub const GET_OBJECT_LEGAL_HOLD: &str = "get-object-legal-hold";
pub const GET_OBJECT_LOCK: &str = "get-object-lock";
pub const GET_OBJECT_RETENTION: &str = "get-object-retention";
pub const GET_OBJECT_TAGGING: &str = "get-object-tagging";
pub const GET_PUBLIC_ACCESS_BLOCK: &str = "get-public-access-block";
pub const GET_PRESIGNED_URL: &str = "get-presigned-url";
pub const HEAD_BUCKET: &str = "head-bucket";
pub const HEAD_OBJECT: &str = "head-object";
pub const RESTORE_OBJECT: &str = "restore-object";
pub const LIST_BUCKET_ANALYTICS: &str = "list-bucket-analytics";
pub const LIST_BUCKETS: &str = "list-buckets";
pub const LIST_DIRECTORY_BUCKETS: &str = "list-directory-buckets";
pub const LIST_BUCKET_INVENTORY: &str = "list-bucket-inventory";
pub const LIST_BUCKET_METRICS: &str = "list-bucket-metrics";
pub const LIST_MULTIPART_UPLOADS: &str = "list-multipart-uploads";
pub const LIST_OBJECT_VERSIONS: &str = "list-object-versions";
pub const LIST_OBJECTS: &str = "list-objects";
pub const LIST_OBJECTS_V2: &str = "list-objects-v2";
pub const LIST_PARTS: &str = "list-parts";
pub const PUT_BUCKET_ACL: &str = "put-bucket-acl";
pub const PUT_BUCKET_ANALYTICS: &str = "put-bucket-analytics";
pub const PUT_BUCKET_CORS: &str = "put-bucket-cors";
pub const PUT_BUCKET_ENCRYPTION: &str = "put-bucket-encryption";
pub const PUT_BUCKET_INVENTORY: &str = "put-bucket-inventory";
pub const PUT_BUCKET_LIFECYCLE: &str = "put-bucket-lifecycle";
pub const PUT_BUCKET_LOGGING: &str = "put-bucket-logging";
pub const PUT_BUCKET_METRICS: &str = "put-bucket-metrics";
pub const PUT_BUCKET_NOTIFICATION: &str = "put-bucket-notification";
pub const PUT_BUCKET_OWNERSHIP_CONTROLS: &str = "put-bucket-ownership";
pub const PUT_BUCKET_POLICY: &str = "put-bucket-policy";
pub const PUT_BUCKET_REPLICATION: &str = "put-bucket-replication";
pub const PUT_BUCKET_TAGGING: &str = "put-bucket-tagging";
pub const PUT_BUCKET_VERSIONING: &str = "put-bucket-versioning";
pub const PUT_BUCKET_WEBSITE: &str = "put-bucket-website";
pub const PUT_OBJECT: &str = "put-object";
pub const PUT_OBJECTS: &str = "put-objects";
pub const PUT_OBJECT_ACL: &str = "put-object-acl";
pub const PUT_OBJECT_LEGAL_HOLD: &str = "put-object-legal-hold";
pub const PUT_OBJECT_LOCK: &str = "put-object-lock";
pub const PUT_OBJECT_RETENTION: &str = "put-object-retention";
pub const PUT_OBJECT_TAGGING: &str = "put-object-tagging";
pub const PUT_PUBLIC_ACCESS_BLOCK: &str = "put-public-access-block";
pub const UPLOAD_PART: &str = "upload-part";
pub const UPLOAD_PART_COPY: &str = "upload-part-copy";
pub const DELETE_BUCKET_TAG_INDEX: &str = "delete-bucket-tagindex";
pub const GET_BUCKET_TAG_INDEX: &str = "get-bucket-tagindex";
pub const PUT_BUCKET_TAG_INDEX: &str = "put-bucket-tagindex";
pub const LIST_BUCKET_TAG_SEARCH: &str = "list-bucket-tagsearch";
pub const STORAGE_MOVE: &str = "storage-move";
pub const UPLOAD: &str = "upload";
pub const DOWNLOAD: &str = "download";
pub const MANUAL_UPLOAD: &str = "manual-upload";
pub const MANUAL_DOWNLOAD: &str = "manual-download";
pub const TEST_ACCESS_IPS: &str = "test-access-ips";
pub const TEST_REPLICATION: &str = "test-replication";
pub const TEST_RANGE_COPY: &str = "test-range-copy";
pub const TEST_COMPARE: &str = "test-compare";
pub const TEST_LIST_OBJECT: &str = "test-list-object";
pub const TEST_UPLOAD: &str = "test-upload";
pub const TEST_DOWNLOAD: &str = "test-download";
pub const TEST_PREPARE: &str = "test-prepare";
pub const TEST_PREPARE_DIR: &str = "test-prepare-dir";
pub const TEST_NEW_PUT: &str = "test-new-put";
pub const TEST_PUT: &str = "test-put";
pub const TEST_HEAD: &str = "test-head";
pub const TEST_GET: &str = "test-get";
pub const TEST_GET_V2: &str = "test-get-v2";
pub const TEST_NEW_GET: &str = "test-new-get";
pub const TEST_GET_V3: &str = "test-get-v3";
pub const TEST_GET_ALL: &str = "test-get-all";
pub const TEST_GET_LIST: &str = "test-get-list";
pub const TEST_DELETE: &str = "test-delete";
pub const TEST_DELETE_V2: &str = "test-delete-v2";
pub const TEST_NEW_DEL: &str = "test-new-del";
pub const TEST_DELETE_VERSION: &str = "test-delete-version";
pub const TEST_DELETE_DIRECTORY: &str = "test-delete-directory";
pub const TEST_MIX: &str = "test-mix";
pub const TEST_MIX_V2: &str = "test-mix-v2";
pub const TEST_NEW_MIX: &str = "test-new-mix";
pub const TEST_PUT_GET: &str = "test-put-get";
pub const TEST_ALL: &str = "test-all";
pub const TEST_FULL: &str = "test-full";
pub const TEST_IO: &str = "test-io";
pub const TEST_MOVER: &str = "test-mover";
pub const TEST_PUT_TAG: &str = "test-put-tag";
pub const TEST_FIND_TAG: &str = "test-find-tag";
pub const TEST_MULTI_DELETE: &str = "test-multi-delete";
pub const TEST_USED_SIZE: &str = "test-used-size";
pub const TEST_DUPLICATE: &str = "test-duplicate";
pub const TEST_MULTI_UPLOAD: &str = "test-multi-upload";
pub const TEST_DIRECTORY_DOWNLOAD: &str = "test-directory-download";
pub const TEST_FILE_LIST_DOWNLOAD: &str = "test-file-list";
pub const TEST_MULTIPART_UPLOAD: &str = "test-multipart-upload";
pub const TEST_MULTIPART_UPLOAD_DOWNLOAD: &str = "test-multipart-upload-v2";
pub const TEST_MULTI_SYSTEM_LIST: &str = "test-multi-system-list";
pub const TEST_MULTI_SYSTEM_UPLOAD: &str = "test-multi-system-upload";
pub const TEST_MULTI_SYSTEM_UP_DOWN: &str = "test-multi-system-up-down";
pub const TEST_MULTI_SYSTEM_ALL: &str = "test-multi-system-all";
pub const TEST_COMPARE_LIFECYCLE: &str = "test-compare-lifecycle";
pub const TEST_RANGE_READ: &str = "test-range-read";
pub const TEST_AWS: &str = "test-aws";
pub const TEST_LOCAL_PREPARE: &str = "test-local-prepare";
pub const TEST_LOCAL_PUT: &str = "test-local-put";
pub const TEST_LOCAL_GET: &str = "test-local-get";
pub const TEST_LOCAL_GET_V2: &str = "test-local-get-v2";
pub const TEST_LOCAL_PUT_GET: &str = "test-local-put-get";
pub const TEST_LOCAL_DELETE: &str = "test-local-delete";
pub const TEST_LOCAL_MULTIPART_PREPARE: &str = "test-local-multipart-prepare";
pub const TEST_LOCAL_MULTIPART_PUT: &str = "test-local-multipart-put";
pub const TEST_LOCAL_MULTIPART_GET: &str = "test-local-multipart-get";
pub const TEST_LOCAL_MULTIPART_GET_V2: &str = "test-local-multipart-get-v2";
pub const TEST_LOCAL_MULTIPART_PUT_GET: &str = "test-local-multipart-put-get";
pub const SET_SSE_S3: &str = "set-sse-s3";
pub const SET_OBJECT_LOCK: &str = "set-object-lock";
pub const DEL_OBJECT_LOCK: &str = "del-object-lock";
pub const UTIL_CLEAR: &str = "clear";
pub const UTIL_BUCKET_CLEAR: &str = "bucket-clear";
pub const UTIL_CURRENT_CLEAR: &str = "current-clear";
pub const UTIL_NONCURRENT_CLEAR: &str = "noncurrent-clear";
pub const UTIL_MARKER_CLEAR: &str = "marker-clear";
pub const USE_CHUNK_ENCODING: &str = "use-chunk-encoding";
pub const CONFIG: &str = "config";
pub const SAVE: &str = "save";
pub const ADMIN: &str = "admin";
pub const ACCESS_KEY: &str = "access-key";
pub const SECRET_KEY: &str = "secret-key";
pub const BUCKET: &str = "bucket";
pub const KEY: &str = "key";
pub const FILE: &str = "file";
pub const PATH: &str = "path";
pub const BODY: &str = "body";
pub const VERSION_ID: &str = "version-id";
pub const DAYS: &str = "days";
pub const YEARS: &str = "years";
pub const DATE: &str = "date";
pub const UPLOAD_ID: &str = "upload-id";
pub const PART_SIZE: &str = "part-size";
pub const START_BYTE: &str = "start-byte";
pub const END_BYTE: &str = "end-byte";
pub const SOURCE: &str = "source";
pub const TARGET: &str = "target";
pub const SOURCE_KEY: &str = "source-key";
pub const ACL: &str = "acl";
pub const BYPASS: &str = "bypass";
pub const ENCRYPTION_KEY: &str = "encryption-key";
pub const QUIET: &str = "quiet";
pub const PREFIX: &str = "prefix";
pub const SUFFIX: &str = "suffix";
pub const CONTINUATION_TOKEN: &str = "continuation-token";
pub const DELIMITER: &str = "delimiter";
pub const MARKER: &str = "marker";
pub const MAX_KEYS: &str = "max-keys";
pub const PART_NUMBER: &str = "part-number";
pub const VERSIONING: &str = "versioning";
pub const TAGS: &str = "tag";
pub const TAG_SET: &str = "tag-set";
pub const STORAGE_CLASS: &str = "storage-class";
pub const CURRENT: &str = "current";
pub const SIZE: &str = "size";
pub const TIMES: &str = "times";
pub const ID: &str = "id";
pub const THREAD_COUNT: &str = "thread";
pub const COUNT: &str = "count";
pub const START: &str = "start";
pub const FLAG: &str = "flag";
pub const NOT_EMPTY: &str = "not-empty";
pub const RANGE_LIST: &str = "range-list";
pub const BUCKET_TYPE: &str = "bucket-type";
pub const THREAD_PREFIX: &str = "thread-prefix";
pub const PRINT: &str = "print";
pub const ALL: &str = "all";
pub const BULK: &str = "bulk";
pub const URL: &str = "url";
pub const USER: &str = "user";
pub const CHECK: &str = "check";
pub const OWNERSHIP: &str = "ownership";
pub const LOCK_ENABLE: &str = "lock-enable";
pub const LOCK_MODE: &str = "lock-mode";
pub const DEBUG: &str = "debug";
pub const CHECKSUM: &str = "checksum";
pub const CHECKSUM_TYPE: &str = "checksum-type";
pub const MD5SUM: &str = "md5sum";
pub const MULTIPART: &str = "multipart";
pub const READ: &str = "read";
pub const WRITE: &str = "write";
pub const DELETE: &str = "delete";
pub const TARGET_PATH: &str = "target-path";
pub const ERROR_BUCKET: &str = "--bucket 버킷 이름을 입력해야 합니다.";
pub const ERROR_FILE_SIZE: &str = "--size 파일 크기를 입력해야 합니다.";
pub const ERROR_KEY: &str = "--key 객체 이름을 입력해야 합니다.";
pub const ERROR_CONFIG_PATH: &str = "--file 설정파일 경로를 입력해야 합니다.";
pub const ERROR_FILE_PATH: &str = "--file 파일 경로를 입력해야 합니다.";
pub const ERROR_PATH: &str = "--path 파일 경로를 입력해야 합니다.";
pub const ERROR_VERSION_ID: &str = "--version-id 값이 올바르지 않습니다.";
pub const ERROR_UPLOAD_ID: &str = "--upload-id 업로드 아이디를 입력해야 합니다.";
pub const ERROR_FILE: &str = "파일을 찾을 수 없습니다.";
pub const ERROR_VERSIONING: &str = "--versioning 값을 입력해야 합니다.";
pub const ERROR_PART_NUMBER: &str = "--part-number 파트번호를 입력해야 합니다.";
pub const ERROR_SOURCE_BUCKET: &str = "--source 버킷 이름을 입력해야 합니다.";
pub const ERROR_SOURCE_KEY: &str = "--source-key 객체 이름을 입력해야 합니다.";
pub const ERROR_ID: &str = "--id 아이디를 입력해야 합니다.";
pub const ERROR_THREAD_COUNT: &str = "--thread-count 스레드 개수를 입력해야 합니다.";
pub const ERROR_SIZE: &str = "--size 사이즈를 입력해야 합니다.";
pub const ERROR_SERVICE_TYPE: &str = "--service-type 서비스 타입을 입력해야 합니다.";
pub const ERROR_ADDRESS: &str = "--address 주소를 입력해야 합니다.";
pub const ERROR_PORT: &str = "--port 포트를 입력해야 합니다.";
pub const ERROR_INVALID_CONFIG: &str =
    "설정값이 올바르지 않습니다. -? 옵션을 통해 설정값을 확인하세요.";
pub const ERROR_OWNERSHIP: &str = "--ownership 값을 입력해야 합니다.";
pub const ERROR_TARGET_PATH: &str = "--target-path 타겟 경로를 입력해야 합니다.";
pub const CONFIG_USAGE_PREFIX: &str = "\n----- Config Setting -------\n";
pub const CONFIG_USAGE_SUFFIX: &str = "----------------------------\n";
pub const TEST_USAGE_ACCESS_IPS: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\n[Jenkins]Host = [string] : 호스트\nJenkinsPort = [int] : Jenkins Port\nPort = [int] : 포트\nUser = [string] : 사용자명\nPassword = [string] : 비밀번호\nDatabase = [string] : Database\n[AccessIps]AllFailed = [bool] : 모든 테스트가 실패해야 하는지 여부\nTestListPath = [string] : 테스트 리스트 파일 경로\nJenkinsTestName = [string] : Jenkins Test Name\nS3URLs = [string] : S3 URL\nVolume = [string] : 볼륨 이름\nMainUser = [string] : Main User Name\nSubUser = [string] : Sub User Name\n----------------------------\n";
pub const TEST_USAGE_COMPARE: &str = "\n----- Config Setting -------\n[Compare]SourceBucket = [string] : 원본 버킷명\nTargetBucket = [string] : 대상 버킷명\nDeleteMarker = [bool] : delete marker 비교 여부\nETagCheck = [bool] : etag 비교 여부\nChecksumCheck = [bool] : checksum 비교 여부\nVersionCheck = [bool] : 버전 비교 여부\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_LIST_OBJECT: &str = "\n----- Config Setting -------\n[ListObj]FileCount = [int] : 스레드 당 생성할 파일 개수\nPrefix = [string] : 조회하거나 생성할 오브젝트 접두어\nTimes = [int] : 테스트 할 시간 (단위 : sec)\nPrepare = [bool] : 테스트 시작전 오브젝트 생성 여부\nThreadCount = [int] : 테스트할 쓰레드 갯수\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_UPLOAD: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DOWNLOAD: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_UP_CURL: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DOWN_CURL: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_PREPARE: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_PUT: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_HEAD: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_GET: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DELETE: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DELETE_VERSION: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DELETE_DIRECTORY: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MIX: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\nETagCheck = [bool] : ETag 비교 여부\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MIX_V2: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\nETagCheck = [bool] : ETag 비교 여부\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_PUT_GET: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\nETagCheck = [bool] : ETag 비교 여부\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_ALL: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\nETagCheck = [bool] : ETag 비교 여부\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MOVER: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[Mover]URL = [string] : URL\nUser = [string] : 사용자명\nSourceBucket = [string] : 원본 버킷명\nTargetBucket = [string] : 대상 버킷명\nFileCount = [int] : 스레드 당 생성할 파일 개수\nMaxFileSize = [int] : 이동할 파일의 최대 크기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_PUT_TAG: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_FIND_TAG: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_ONLY_GET: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_ONLY_HEAD: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_ONLY_PUT: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MULTI_DELETE: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\n[UpDown]ThreadCount = [int] : 쓰레드 개수\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_USED_SIZE: &str = "\n----- Config Setting -------\n[UsedSize]BucketPrefix = [string] : 버킷 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nPartSize = [string] : 멀티파트 업로드 시 파트 크기(ex> 128K, 10M, 1G)\n[DB]\nHost = [string] : 호스트\nPort = [int] : 포트\nUser = [string] : 사용자명\nPassword = [string] : 비밀번호\nDatabase = [string] : Database\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_DUPLICATE: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[Duplicate]ObjectPath = [string] : 오브젝트 경로\nLoopCount = [int] : 반복 횟수\nObjectCount = [int] : 오브젝트 생성 개수\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MULTIPART_UPLOAD: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nPartSize = [string] : 멀티파트 업로드 시 파트 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_MULTIPART_UPLOAD_V2: &str = "\n----- Config Setting -------\n[Default]BucketName = [string] : 버킷 이름\nObjectPrefix = [string] : 생성할 오브젝트 접두어\nFileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\nPartSize = [string] : 멀티파트 업로드 시 파트 크기(ex> 128K, 10M, 1G)\nRetry = [int] : 재시도 횟수\n[UpDown]BucketType[int] : 버킷 생성 방식(0:설정없음, 1:단일버킷 업로드, 2: 쓰레드당 버킷 1개로 업로드, 3:시간당 하나의 폴더, 4:단일 버킷. 오브젝트 prefix에 {Random:10} 형식의 변수 추가 가능)\nThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\nDivisionCount[int] : 오브젝트의 폴더 분리 분기\n[Main User]\nAccessKey = [string] : AccessKey\nSecretKey = [string] : SecretKey\n----------------------------\n";
pub const TEST_USAGE_LOCAL_PREPARE: &str = "\n----- Config Setting -------\n[Default]FileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[UpDown]ThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\n----------------------------\n";
pub const TEST_USAGE_LOCAL_PUT: &str = "\n----- Config Setting -------\n[Default]FileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[UpDown]ThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\n----------------------------\n";
pub const TEST_USAGE_LOCAL_GET: &str = "\n----- Config Setting -------\n[Default]FileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[UpDown]ThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\n----------------------------\n";
pub const TEST_USAGE_LOCAL_GET_V2: &str = "\n----- Config Setting -------\n[Default]FileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[UpDown]ThreadCount = [int] : 쓰레드 개수\nFileCount = [int] : 스레드 당 생성할 파일 개수\n----------------------------\n";
pub const TEST_USAGE_LOCAL_PUT_GET: &str = "\n----- Config Setting -------\n[Default]FileSize = [string] : 생성할 파일 크기(ex> 128K, 10M, 1G)\n[UpDown]ThreadCount = [int] : 쓰레드 개수\nTimes = [int] : 테스트 할 시간(sec)\nETagCheck = [bool] : ETag 비교 여부\n----------------------------\n";
pub const TEST_USAGE_LOCAL_DELETE: &str = "\n----- Config Setting -------\n[Default][UpDown]ThreadCount = [int] : 쓰레드 개수\n----------------------------\n";
pub const SERVICE_TYPE: &str = "service-type";
pub const ADDRESS: &str = "address";
pub const PORT: &str = "port";
pub const BACKEND_PAUSE: &str = "pause";
pub const BACKEND_RESUME: &str = "resume";

/// 모든 상수(이름, 값). 원본과 비교하는 테스트에서 쓴다.
pub const CONSTANTS: &[(&str, &str)] = &[
    ("USAGE_BUCKET_NAME", USAGE_BUCKET_NAME),
    ("USAGE_THREAD_COUNT", USAGE_THREAD_COUNT),
    ("USAGE_SIZE", USAGE_SIZE),
    ("USAGE_FILE_COUNT", USAGE_FILE_COUNT),
    ("USAGE_TIMES", USAGE_TIMES),
    ("USAGE_CONFIG", USAGE_CONFIG),
    ("USAGE_PREFIX", USAGE_PREFIX),
    ("USAGE_PATH", USAGE_PATH),
    ("USAGE_START", USAGE_START),
    ("ABORT_MULTIPART_UPLOAD", ABORT_MULTIPART_UPLOAD),
    ("COMPLETE_MULTIPART_UPLOAD", COMPLETE_MULTIPART_UPLOAD),
    ("COPY_OBJECT", COPY_OBJECT),
    ("CREATE_BUCKET", CREATE_BUCKET),
    ("CREATE_MULTIPART_UPLOAD", CREATE_MULTIPART_UPLOAD),
    ("DELETE_BUCKET", DELETE_BUCKET),
    ("DELETE_BUCKET_ANALYTICS", DELETE_BUCKET_ANALYTICS),
    ("DELETE_BUCKET_CORS", DELETE_BUCKET_CORS),
    ("DELETE_BUCKET_ENCRYPTION", DELETE_BUCKET_ENCRYPTION),
    ("DELETE_BUCKET_INVENTORY", DELETE_BUCKET_INVENTORY),
    ("DELETE_BUCKET_LIFECYCLE", DELETE_BUCKET_LIFECYCLE),
    ("DELETE_BUCKET_METRICS", DELETE_BUCKET_METRICS),
    (
        "DELETE_BUCKET_OWNERSHIP_CONTROLS",
        DELETE_BUCKET_OWNERSHIP_CONTROLS,
    ),
    ("DELETE_BUCKET_POLICY", DELETE_BUCKET_POLICY),
    ("DELETE_BUCKET_REPLICATION", DELETE_BUCKET_REPLICATION),
    ("DELETE_BUCKET_TAGGING", DELETE_BUCKET_TAGGING),
    ("DELETE_BUCKET_WEBSITE", DELETE_BUCKET_WEBSITE),
    ("DELETE_OBJECT", DELETE_OBJECT),
    ("DELETE_OBJECT_TAGGING", DELETE_OBJECT_TAGGING),
    ("DELETE_OBJECTS", DELETE_OBJECTS),
    ("DELETE_PUBLIC_ACCESS_BLOCK", DELETE_PUBLIC_ACCESS_BLOCK),
    ("GET_BUCKET_ACL", GET_BUCKET_ACL),
    ("GET_BUCKET_ANALYTICS", GET_BUCKET_ANALYTICS),
    ("GET_BUCKET_CORS", GET_BUCKET_CORS),
    ("GET_BUCKET_ENCRYPTION", GET_BUCKET_ENCRYPTION),
    ("GET_BUCKET_INVENTORY", GET_BUCKET_INVENTORY),
    ("GET_BUCKET_LIFECYCLE", GET_BUCKET_LIFECYCLE),
    ("GET_BUCKET_LOGGING", GET_BUCKET_LOGGING),
    ("GET_BUCKET_METRICS", GET_BUCKET_METRICS),
    ("GET_BUCKET_LOCATION", GET_BUCKET_LOCATION),
    ("GET_BUCKET_NOTIFICATION", GET_BUCKET_NOTIFICATION),
    (
        "GET_BUCKET_OWNERSHIP_CONTROLS",
        GET_BUCKET_OWNERSHIP_CONTROLS,
    ),
    ("GET_BUCKET_POLICY", GET_BUCKET_POLICY),
    ("GET_BUCKET_POLICY_STATUS", GET_BUCKET_POLICY_STATUS),
    ("GET_BUCKET_REPLICATION", GET_BUCKET_REPLICATION),
    ("GET_BUCKET_TAGGING", GET_BUCKET_TAGGING),
    ("GET_BUCKET_VERSIONING", GET_BUCKET_VERSIONING),
    ("GET_BUCKET_WEBSITE", GET_BUCKET_WEBSITE),
    ("GET_OBJECT", GET_OBJECT),
    ("GET_OBJECT_ACL", GET_OBJECT_ACL),
    ("GET_OBJECT_LEGAL_HOLD", GET_OBJECT_LEGAL_HOLD),
    ("GET_OBJECT_LOCK", GET_OBJECT_LOCK),
    ("GET_OBJECT_RETENTION", GET_OBJECT_RETENTION),
    ("GET_OBJECT_TAGGING", GET_OBJECT_TAGGING),
    ("GET_PUBLIC_ACCESS_BLOCK", GET_PUBLIC_ACCESS_BLOCK),
    ("GET_PRESIGNED_URL", GET_PRESIGNED_URL),
    ("HEAD_BUCKET", HEAD_BUCKET),
    ("HEAD_OBJECT", HEAD_OBJECT),
    ("RESTORE_OBJECT", RESTORE_OBJECT),
    ("LIST_BUCKET_ANALYTICS", LIST_BUCKET_ANALYTICS),
    ("LIST_BUCKETS", LIST_BUCKETS),
    ("LIST_DIRECTORY_BUCKETS", LIST_DIRECTORY_BUCKETS),
    ("LIST_BUCKET_INVENTORY", LIST_BUCKET_INVENTORY),
    ("LIST_BUCKET_METRICS", LIST_BUCKET_METRICS),
    ("LIST_MULTIPART_UPLOADS", LIST_MULTIPART_UPLOADS),
    ("LIST_OBJECT_VERSIONS", LIST_OBJECT_VERSIONS),
    ("LIST_OBJECTS", LIST_OBJECTS),
    ("LIST_OBJECTS_V2", LIST_OBJECTS_V2),
    ("LIST_PARTS", LIST_PARTS),
    ("PUT_BUCKET_ACL", PUT_BUCKET_ACL),
    ("PUT_BUCKET_ANALYTICS", PUT_BUCKET_ANALYTICS),
    ("PUT_BUCKET_CORS", PUT_BUCKET_CORS),
    ("PUT_BUCKET_ENCRYPTION", PUT_BUCKET_ENCRYPTION),
    ("PUT_BUCKET_INVENTORY", PUT_BUCKET_INVENTORY),
    ("PUT_BUCKET_LIFECYCLE", PUT_BUCKET_LIFECYCLE),
    ("PUT_BUCKET_LOGGING", PUT_BUCKET_LOGGING),
    ("PUT_BUCKET_METRICS", PUT_BUCKET_METRICS),
    ("PUT_BUCKET_NOTIFICATION", PUT_BUCKET_NOTIFICATION),
    (
        "PUT_BUCKET_OWNERSHIP_CONTROLS",
        PUT_BUCKET_OWNERSHIP_CONTROLS,
    ),
    ("PUT_BUCKET_POLICY", PUT_BUCKET_POLICY),
    ("PUT_BUCKET_REPLICATION", PUT_BUCKET_REPLICATION),
    ("PUT_BUCKET_TAGGING", PUT_BUCKET_TAGGING),
    ("PUT_BUCKET_VERSIONING", PUT_BUCKET_VERSIONING),
    ("PUT_BUCKET_WEBSITE", PUT_BUCKET_WEBSITE),
    ("PUT_OBJECT", PUT_OBJECT),
    ("PUT_OBJECTS", PUT_OBJECTS),
    ("PUT_OBJECT_ACL", PUT_OBJECT_ACL),
    ("PUT_OBJECT_LEGAL_HOLD", PUT_OBJECT_LEGAL_HOLD),
    ("PUT_OBJECT_LOCK", PUT_OBJECT_LOCK),
    ("PUT_OBJECT_RETENTION", PUT_OBJECT_RETENTION),
    ("PUT_OBJECT_TAGGING", PUT_OBJECT_TAGGING),
    ("PUT_PUBLIC_ACCESS_BLOCK", PUT_PUBLIC_ACCESS_BLOCK),
    ("UPLOAD_PART", UPLOAD_PART),
    ("UPLOAD_PART_COPY", UPLOAD_PART_COPY),
    ("DELETE_BUCKET_TAG_INDEX", DELETE_BUCKET_TAG_INDEX),
    ("GET_BUCKET_TAG_INDEX", GET_BUCKET_TAG_INDEX),
    ("PUT_BUCKET_TAG_INDEX", PUT_BUCKET_TAG_INDEX),
    ("LIST_BUCKET_TAG_SEARCH", LIST_BUCKET_TAG_SEARCH),
    ("STORAGE_MOVE", STORAGE_MOVE),
    ("UPLOAD", UPLOAD),
    ("DOWNLOAD", DOWNLOAD),
    ("MANUAL_UPLOAD", MANUAL_UPLOAD),
    ("MANUAL_DOWNLOAD", MANUAL_DOWNLOAD),
    ("TEST_ACCESS_IPS", TEST_ACCESS_IPS),
    ("TEST_REPLICATION", TEST_REPLICATION),
    ("TEST_RANGE_COPY", TEST_RANGE_COPY),
    ("TEST_COMPARE", TEST_COMPARE),
    ("TEST_LIST_OBJECT", TEST_LIST_OBJECT),
    ("TEST_UPLOAD", TEST_UPLOAD),
    ("TEST_DOWNLOAD", TEST_DOWNLOAD),
    ("TEST_PREPARE", TEST_PREPARE),
    ("TEST_PREPARE_DIR", TEST_PREPARE_DIR),
    ("TEST_NEW_PUT", TEST_NEW_PUT),
    ("TEST_PUT", TEST_PUT),
    ("TEST_HEAD", TEST_HEAD),
    ("TEST_GET", TEST_GET),
    ("TEST_GET_V2", TEST_GET_V2),
    ("TEST_NEW_GET", TEST_NEW_GET),
    ("TEST_GET_V3", TEST_GET_V3),
    ("TEST_GET_ALL", TEST_GET_ALL),
    ("TEST_GET_LIST", TEST_GET_LIST),
    ("TEST_DELETE", TEST_DELETE),
    ("TEST_DELETE_V2", TEST_DELETE_V2),
    ("TEST_NEW_DEL", TEST_NEW_DEL),
    ("TEST_DELETE_VERSION", TEST_DELETE_VERSION),
    ("TEST_DELETE_DIRECTORY", TEST_DELETE_DIRECTORY),
    ("TEST_MIX", TEST_MIX),
    ("TEST_MIX_V2", TEST_MIX_V2),
    ("TEST_NEW_MIX", TEST_NEW_MIX),
    ("TEST_PUT_GET", TEST_PUT_GET),
    ("TEST_ALL", TEST_ALL),
    ("TEST_FULL", TEST_FULL),
    ("TEST_IO", TEST_IO),
    ("TEST_MOVER", TEST_MOVER),
    ("TEST_PUT_TAG", TEST_PUT_TAG),
    ("TEST_FIND_TAG", TEST_FIND_TAG),
    ("TEST_MULTI_DELETE", TEST_MULTI_DELETE),
    ("TEST_USED_SIZE", TEST_USED_SIZE),
    ("TEST_DUPLICATE", TEST_DUPLICATE),
    ("TEST_MULTI_UPLOAD", TEST_MULTI_UPLOAD),
    ("TEST_DIRECTORY_DOWNLOAD", TEST_DIRECTORY_DOWNLOAD),
    ("TEST_FILE_LIST_DOWNLOAD", TEST_FILE_LIST_DOWNLOAD),
    ("TEST_MULTIPART_UPLOAD", TEST_MULTIPART_UPLOAD),
    (
        "TEST_MULTIPART_UPLOAD_DOWNLOAD",
        TEST_MULTIPART_UPLOAD_DOWNLOAD,
    ),
    ("TEST_MULTI_SYSTEM_LIST", TEST_MULTI_SYSTEM_LIST),
    ("TEST_MULTI_SYSTEM_UPLOAD", TEST_MULTI_SYSTEM_UPLOAD),
    ("TEST_MULTI_SYSTEM_UP_DOWN", TEST_MULTI_SYSTEM_UP_DOWN),
    ("TEST_MULTI_SYSTEM_ALL", TEST_MULTI_SYSTEM_ALL),
    ("TEST_COMPARE_LIFECYCLE", TEST_COMPARE_LIFECYCLE),
    ("TEST_RANGE_READ", TEST_RANGE_READ),
    ("TEST_AWS", TEST_AWS),
    ("TEST_LOCAL_PREPARE", TEST_LOCAL_PREPARE),
    ("TEST_LOCAL_PUT", TEST_LOCAL_PUT),
    ("TEST_LOCAL_GET", TEST_LOCAL_GET),
    ("TEST_LOCAL_GET_V2", TEST_LOCAL_GET_V2),
    ("TEST_LOCAL_PUT_GET", TEST_LOCAL_PUT_GET),
    ("TEST_LOCAL_DELETE", TEST_LOCAL_DELETE),
    ("TEST_LOCAL_MULTIPART_PREPARE", TEST_LOCAL_MULTIPART_PREPARE),
    ("TEST_LOCAL_MULTIPART_PUT", TEST_LOCAL_MULTIPART_PUT),
    ("TEST_LOCAL_MULTIPART_GET", TEST_LOCAL_MULTIPART_GET),
    ("TEST_LOCAL_MULTIPART_GET_V2", TEST_LOCAL_MULTIPART_GET_V2),
    ("TEST_LOCAL_MULTIPART_PUT_GET", TEST_LOCAL_MULTIPART_PUT_GET),
    ("SET_SSE_S3", SET_SSE_S3),
    ("SET_OBJECT_LOCK", SET_OBJECT_LOCK),
    ("DEL_OBJECT_LOCK", DEL_OBJECT_LOCK),
    ("UTIL_CLEAR", UTIL_CLEAR),
    ("UTIL_BUCKET_CLEAR", UTIL_BUCKET_CLEAR),
    ("UTIL_CURRENT_CLEAR", UTIL_CURRENT_CLEAR),
    ("UTIL_NONCURRENT_CLEAR", UTIL_NONCURRENT_CLEAR),
    ("UTIL_MARKER_CLEAR", UTIL_MARKER_CLEAR),
    ("USE_CHUNK_ENCODING", USE_CHUNK_ENCODING),
    ("CONFIG", CONFIG),
    ("SAVE", SAVE),
    ("ADMIN", ADMIN),
    ("ACCESS_KEY", ACCESS_KEY),
    ("SECRET_KEY", SECRET_KEY),
    ("BUCKET", BUCKET),
    ("KEY", KEY),
    ("FILE", FILE),
    ("PATH", PATH),
    ("BODY", BODY),
    ("VERSION_ID", VERSION_ID),
    ("DAYS", DAYS),
    ("YEARS", YEARS),
    ("DATE", DATE),
    ("UPLOAD_ID", UPLOAD_ID),
    ("PART_SIZE", PART_SIZE),
    ("START_BYTE", START_BYTE),
    ("END_BYTE", END_BYTE),
    ("SOURCE", SOURCE),
    ("TARGET", TARGET),
    ("SOURCE_KEY", SOURCE_KEY),
    ("ACL", ACL),
    ("BYPASS", BYPASS),
    ("ENCRYPTION_KEY", ENCRYPTION_KEY),
    ("QUIET", QUIET),
    ("PREFIX", PREFIX),
    ("SUFFIX", SUFFIX),
    ("CONTINUATION_TOKEN", CONTINUATION_TOKEN),
    ("DELIMITER", DELIMITER),
    ("MARKER", MARKER),
    ("MAX_KEYS", MAX_KEYS),
    ("PART_NUMBER", PART_NUMBER),
    ("VERSIONING", VERSIONING),
    ("TAGS", TAGS),
    ("TAG_SET", TAG_SET),
    ("STORAGE_CLASS", STORAGE_CLASS),
    ("CURRENT", CURRENT),
    ("SIZE", SIZE),
    ("TIMES", TIMES),
    ("ID", ID),
    ("THREAD_COUNT", THREAD_COUNT),
    ("COUNT", COUNT),
    ("START", START),
    ("FLAG", FLAG),
    ("NOT_EMPTY", NOT_EMPTY),
    ("RANGE_LIST", RANGE_LIST),
    ("BUCKET_TYPE", BUCKET_TYPE),
    ("THREAD_PREFIX", THREAD_PREFIX),
    ("PRINT", PRINT),
    ("ALL", ALL),
    ("BULK", BULK),
    ("URL", URL),
    ("USER", USER),
    ("CHECK", CHECK),
    ("OWNERSHIP", OWNERSHIP),
    ("LOCK_ENABLE", LOCK_ENABLE),
    ("LOCK_MODE", LOCK_MODE),
    ("DEBUG", DEBUG),
    ("CHECKSUM", CHECKSUM),
    ("CHECKSUM_TYPE", CHECKSUM_TYPE),
    ("MD5SUM", MD5SUM),
    ("MULTIPART", MULTIPART),
    ("READ", READ),
    ("WRITE", WRITE),
    ("DELETE", DELETE),
    ("TARGET_PATH", TARGET_PATH),
    ("ERROR_BUCKET", ERROR_BUCKET),
    ("ERROR_FILE_SIZE", ERROR_FILE_SIZE),
    ("ERROR_KEY", ERROR_KEY),
    ("ERROR_CONFIG_PATH", ERROR_CONFIG_PATH),
    ("ERROR_FILE_PATH", ERROR_FILE_PATH),
    ("ERROR_PATH", ERROR_PATH),
    ("ERROR_VERSION_ID", ERROR_VERSION_ID),
    ("ERROR_UPLOAD_ID", ERROR_UPLOAD_ID),
    ("ERROR_FILE", ERROR_FILE),
    ("ERROR_VERSIONING", ERROR_VERSIONING),
    ("ERROR_PART_NUMBER", ERROR_PART_NUMBER),
    ("ERROR_SOURCE_BUCKET", ERROR_SOURCE_BUCKET),
    ("ERROR_SOURCE_KEY", ERROR_SOURCE_KEY),
    ("ERROR_ID", ERROR_ID),
    ("ERROR_THREAD_COUNT", ERROR_THREAD_COUNT),
    ("ERROR_SIZE", ERROR_SIZE),
    ("ERROR_SERVICE_TYPE", ERROR_SERVICE_TYPE),
    ("ERROR_ADDRESS", ERROR_ADDRESS),
    ("ERROR_PORT", ERROR_PORT),
    ("ERROR_INVALID_CONFIG", ERROR_INVALID_CONFIG),
    ("ERROR_OWNERSHIP", ERROR_OWNERSHIP),
    ("ERROR_TARGET_PATH", ERROR_TARGET_PATH),
    ("CONFIG_USAGE_PREFIX", CONFIG_USAGE_PREFIX),
    ("CONFIG_USAGE_SUFFIX", CONFIG_USAGE_SUFFIX),
    ("TEST_USAGE_ACCESS_IPS", TEST_USAGE_ACCESS_IPS),
    ("TEST_USAGE_COMPARE", TEST_USAGE_COMPARE),
    ("TEST_USAGE_LIST_OBJECT", TEST_USAGE_LIST_OBJECT),
    ("TEST_USAGE_UPLOAD", TEST_USAGE_UPLOAD),
    ("TEST_USAGE_DOWNLOAD", TEST_USAGE_DOWNLOAD),
    ("TEST_USAGE_UP_CURL", TEST_USAGE_UP_CURL),
    ("TEST_USAGE_DOWN_CURL", TEST_USAGE_DOWN_CURL),
    ("TEST_USAGE_PREPARE", TEST_USAGE_PREPARE),
    ("TEST_USAGE_PUT", TEST_USAGE_PUT),
    ("TEST_USAGE_HEAD", TEST_USAGE_HEAD),
    ("TEST_USAGE_GET", TEST_USAGE_GET),
    ("TEST_USAGE_DELETE", TEST_USAGE_DELETE),
    ("TEST_USAGE_DELETE_VERSION", TEST_USAGE_DELETE_VERSION),
    ("TEST_USAGE_DELETE_DIRECTORY", TEST_USAGE_DELETE_DIRECTORY),
    ("TEST_USAGE_MIX", TEST_USAGE_MIX),
    ("TEST_USAGE_MIX_V2", TEST_USAGE_MIX_V2),
    ("TEST_USAGE_PUT_GET", TEST_USAGE_PUT_GET),
    ("TEST_USAGE_ALL", TEST_USAGE_ALL),
    ("TEST_USAGE_MOVER", TEST_USAGE_MOVER),
    ("TEST_USAGE_PUT_TAG", TEST_USAGE_PUT_TAG),
    ("TEST_USAGE_FIND_TAG", TEST_USAGE_FIND_TAG),
    ("TEST_USAGE_ONLY_GET", TEST_USAGE_ONLY_GET),
    ("TEST_USAGE_ONLY_HEAD", TEST_USAGE_ONLY_HEAD),
    ("TEST_USAGE_ONLY_PUT", TEST_USAGE_ONLY_PUT),
    ("TEST_USAGE_MULTI_DELETE", TEST_USAGE_MULTI_DELETE),
    ("TEST_USAGE_USED_SIZE", TEST_USAGE_USED_SIZE),
    ("TEST_USAGE_DUPLICATE", TEST_USAGE_DUPLICATE),
    ("TEST_USAGE_MULTIPART_UPLOAD", TEST_USAGE_MULTIPART_UPLOAD),
    (
        "TEST_USAGE_MULTIPART_UPLOAD_V2",
        TEST_USAGE_MULTIPART_UPLOAD_V2,
    ),
    ("TEST_USAGE_LOCAL_PREPARE", TEST_USAGE_LOCAL_PREPARE),
    ("TEST_USAGE_LOCAL_PUT", TEST_USAGE_LOCAL_PUT),
    ("TEST_USAGE_LOCAL_GET", TEST_USAGE_LOCAL_GET),
    ("TEST_USAGE_LOCAL_GET_V2", TEST_USAGE_LOCAL_GET_V2),
    ("TEST_USAGE_LOCAL_PUT_GET", TEST_USAGE_LOCAL_PUT_GET),
    ("TEST_USAGE_LOCAL_DELETE", TEST_USAGE_LOCAL_DELETE),
    ("SERVICE_TYPE", SERVICE_TYPE),
    ("ADDRESS", ADDRESS),
    ("PORT", PORT),
    ("BACKEND_PAUSE", BACKEND_PAUSE),
    ("BACKEND_RESUME", BACKEND_RESUME),
];
