//! 버킷·ACL·멀티파트 메뉴가 같이 쓰는 출력·오류 도우미.
//!
//! - 시각: AWS SDK(.NET v4)는 응답 XML의 시각을 UTC `DateTime`으로 읽는다. `ToString(...)`은 값을 그대로
//!   서식으로 바꾸므로 UTC 시각이 출력된다(`output::invariant_time`·`ko_kr_time`은 로컬 시각으로 바꾼다).
//! - 오류: .NET SDK는 연산마다 모델링한 오류 코드만 전용 예외 형식(`NoSuchUploadException` 등)으로 던지고,
//!   그 밖의 코드는 모두 `AmazonS3Exception`이다. S3 클라이언트는 코드만으로 형식을 정하므로, 연산이 모델링한
//!   코드가 아니면 `AmazonS3Exception`으로 바꾼다.

use aws_sdk_s3::primitives::DateTime;
use awscli_rest_s3::S3Error;
use chrono::{DateTime as ChronoDateTime, Utc};

use crate::dispatch::CommandError;

fn utc(time: &DateTime) -> Option<ChronoDateTime<Utc>> {
    ChronoDateTime::<Utc>::from_timestamp(time.secs(), time.subsec_nanos())
}

/// `ToString("yyyy-MM-dd HH:mm:ss", InvariantInfo)`(UTC).
pub(in crate::dispatch) fn invariant_time(time: &DateTime) -> String {
    utc(time).map_or_else(String::new, |t| t.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// `modeled`에 든 오류 코드는 전용 예외 형식으로, 나머지는 `AmazonS3Exception`으로 바꾼다.
pub(in crate::dispatch) fn op_error(error: S3Error, modeled: &[&str]) -> CommandError {
    match &error {
        S3Error::Service { code, .. } if !modeled.contains(&code.as_str()) => {
            CommandError::new("Amazon.S3.AmazonS3Exception", error.to_string())
        }
        _ => CommandError::from(error),
    }
}
