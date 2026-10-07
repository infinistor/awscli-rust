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

/// ko-KR `DateTime.ToString()`: `yyyy-MM-dd tt h:mm:ss`(UTC).
pub(in crate::dispatch) fn ko_kr_time(time: &DateTime) -> String {
    use chrono::Timelike;
    utc(time).map_or_else(String::new, |t| {
        let (pm, hour) = t.hour12();
        format!(
            "{} {} {}:{:02}:{:02}",
            t.format("%Y-%m-%d"),
            if pm { "오후" } else { "오전" },
            hour,
            t.minute(),
            t.second()
        )
    })
}

/// `File.ReadAllText(path)`: BOM으로 UTF-8/UTF-16/UTF-32를 판별하고, 없으면 UTF-8로 읽는다(잘못된 바이트는 U+FFFD).
pub(in crate::dispatch) fn read_all_text(path: &str) -> Result<String, CommandError> {
    fn utf16(bytes: &[u8], to_u16: fn([u8; 2]) -> u16) -> String {
        let units: Vec<u16> = bytes
            .chunks(2)
            .map(|pair| match *pair {
                [a, b] => to_u16([a, b]),
                _ => 0xFFFD,
            })
            .collect();
        String::from_utf16_lossy(&units)
    }
    fn utf32(bytes: &[u8], to_u32: fn([u8; 4]) -> u32) -> String {
        bytes
            .chunks(4)
            .map(|quad| match *quad {
                [a, b, c, d] => char::from_u32(to_u32([a, b, c, d])).unwrap_or('\u{FFFD}'),
                _ => '\u{FFFD}',
            })
            .collect()
    }
    let bytes = std::fs::read(path)
        .map_err(|e| CommandError::from(S3Error::io(std::path::Path::new(path), &e)))?;
    Ok(
        if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE\x00\x00") {
            utf32(rest, u32::from_le_bytes)
        } else if let Some(rest) = bytes.strip_prefix(b"\x00\x00\xFE\xFF") {
            utf32(rest, u32::from_be_bytes)
        } else if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
            String::from_utf8_lossy(rest).into_owned()
        } else if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
            utf16(rest, u16::from_le_bytes)
        } else if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
            utf16(rest, u16::from_be_bytes)
        } else {
            String::from_utf8_lossy(&bytes).into_owned()
        },
    )
}

/// `System.Text.Json.JsonException`.
pub(in crate::dispatch) fn json_error(error: awscli_rest_common::json::JsonError) -> CommandError {
    CommandError::new("System.Text.Json.JsonException", error.0)
}

/// `System.NullReferenceException`.
pub(in crate::dispatch) fn null_reference() -> CommandError {
    CommandError::new(
        "System.NullReferenceException",
        "Object reference not set to an instance of an object.",
    )
}
