//! 버킷·ACL·멀티파트 메뉴가 같이 쓰는 출력·오류 도우미.
//!
//! (시각 서식은 `output::invariant_time`·`ko_kr_time`, S3 오류 형식은 `CommandError::s3`로 옮겼다.)

use awscli_rest_s3::S3Error;

use crate::dispatch::CommandError;

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
