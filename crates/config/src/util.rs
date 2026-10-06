//! TESTCore `Util/Utility.cs` 중 설정 계층이 쓰는 도우미만 옮긴 모듈.
//!
//! 옮긴 것: `SizeToLong`, `GetRandomObjectName`(과 `GetRandomName`, `ExistRandomString`, `RandomTextLong`)
//! 그리고 `Config`의 `ReadKeyToInt`/`ReadKeyToBoolean`이 쓰는 .NET `TryParse` 동작.

use rand::Rng;

/// 도우미 함수의 오류. 원본에서는 예외(`FormatException`, `ArgumentOutOfRangeException`, ...)로 던져지던 경우다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UtilError {
    /// `long.Parse`/`int.Parse` 실패.
    #[error("입력 문자열의 형식이 올바르지 않습니다: '{0}'")]
    InvalidNumber(String),
    /// 0으로 나누기.
    #[error("0으로 나눌 수 없습니다.")]
    DivideByZero,
    /// 원본에서 `ArgumentOutOfRangeException`이 나던 입력.
    #[error("인수가 허용 범위를 벗어났습니다: {0}")]
    OutOfRange(&'static str),
}

pub const KB: i64 = 1000;
pub const MB: i64 = 1000 * KB;
pub const GB: i64 = 1000 * MB;
pub const TB: i64 = 1000 * GB;
pub const KIB: i64 = 1024;
pub const MIB: i64 = 1024 * KIB;
pub const GIB: i64 = 1024 * MIB;
pub const TIB: i64 = 1024 * GIB;

const TEXT_LONG: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// .NET `NumberStyles.Integer`가 허용하는 앞뒤 공백(0x09~0x0D, 0x20)만 제거한다.
fn trim_number_ws(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, '\u{9}'..='\u{D}' | ' '))
}

/// .NET `long.TryParse`(기본 스타일, 부호와 앞뒤 공백 허용). 실패하면 `None`.
pub fn try_parse_i64(s: &str) -> Option<i64> {
    trim_number_ws(s).parse().ok()
}

/// .NET `int.TryParse`.
pub fn try_parse_i32(s: &str) -> Option<i32> {
    trim_number_ws(s).parse().ok()
}

/// .NET `bool.TryParse`: 앞뒤 공백을 무시하고 `true`/`false`를 대소문자 구분 없이 인식한다.
pub fn try_parse_bool(s: &str) -> Option<bool> {
    let t = trim_number_ws(s);
    if t.eq_ignore_ascii_case("true") {
        Some(true)
    } else if t.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn parse_i64(s: &str) -> Result<i64, UtilError> {
    try_parse_i64(s).ok_or_else(|| UtilError::InvalidNumber(s.to_string()))
}

fn parse_i32(s: &str) -> Result<i32, UtilError> {
    try_parse_i32(s).ok_or_else(|| UtilError::InvalidNumber(s.to_string()))
}

/// 대소문자를 무시하고(ASCII) `suffix`로 끝나는지 확인한다.
fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    let (s, suffix) = (s.as_bytes(), suffix.as_bytes());
    s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// `"10M"`, `"1KiB"`와 같은 크기 문자열을 바이트 수로 바꾼다(`Utility.SizeToLong`).
///
/// K/M/G/T는 1000 단위, Ki/Mi/Gi/Ti는 1024 단위다. 비어 있으면 0이고, 숫자 부분이 잘못되면 오류다.
/// 곱셈은 원본처럼 오버플로를 검사하지 않고 감싼다(`unchecked`).
pub fn size_to_long(size: &str) -> Result<i64, UtilError> {
    if size.trim().is_empty() {
        return Ok(0);
    }
    // 원본의 검사 순서를 그대로 따른다.
    const UNITS: &[(&str, i64)] = &[
        ("K", KB),
        ("M", MB),
        ("G", GB),
        ("T", TB),
        ("KB", KB),
        ("MB", MB),
        ("GB", GB),
        ("TB", TB),
        ("Ki", KIB),
        ("Mi", MIB),
        ("Gi", GIB),
        ("Ti", TIB),
        ("KiB", KIB),
        ("MiB", MIB),
        ("GiB", GIB),
        ("TiB", TIB),
    ];
    for (suffix, unit) in UNITS {
        if ends_with_ignore_case(size, suffix) {
            let number = &size[..size.len() - suffix.len()];
            return Ok(parse_i64(number)?.wrapping_mul(*unit));
        }
    }
    parse_i64(size)
}

/// `Utility.RandomTextLong`: 영문 대소문자와 숫자로 된 임의의 문자열.
pub fn random_text_long(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| TEXT_LONG[rng.random_range(0..TEXT_LONG.len())] as char)
        .collect()
}

/// 문자열에 `{Random` 패턴이 (대소문자 무시로) 포함되어 있는지 확인한다(`Utility.ExistRandomString`).
pub fn exist_random_string(prefix: &str) -> bool {
    prefix.to_ascii_lowercase().contains("{random")
}

/// C# 사용자 지정 서식 `00000`: 최소 5자리로 0을 채우고, 음수는 `-`를 앞에 붙인다.
pub(crate) fn fmt5(n: i64) -> String {
    if n < 0 {
        format!("-{:05}", n.unsigned_abs())
    } else {
        format!("{n:05}")
    }
}

/// prefix 안의 `{Random}`, `{Random:N}`, `{ObjectCount}`를 실제 값으로 바꾼다(`Utility.GetRandomName`).
///
/// 원본처럼 `{Random`이 없으면 빈 문자열을 돌려준다.
pub fn get_random_name(prefix: &str, object_count: i32) -> Result<String, UtilError> {
    let mut result = String::new();
    // ASCII 소문자화는 바이트 길이를 바꾸지 않으므로 위치를 그대로 쓸 수 있다.
    if let Some(start) = prefix.to_ascii_lowercase().find("{random") {
        let end = prefix[start..]
            .find('}')
            .map(|i| i + start)
            .ok_or(UtilError::OutOfRange("Remove"))?;
        let mut random_length = 10;
        // {Random:10} 형식으로 길이를 지정할 수 있다.
        let marker = prefix
            .get(start + 7..)
            .and_then(|s| s.chars().next())
            .ok_or(UtilError::OutOfRange("Substring"))?;
        if marker == ':' {
            let digits = prefix
                .get(start + 8..end)
                .ok_or(UtilError::OutOfRange("Substring"))?;
            random_length = parse_i32(digits)?;
        }
        let random_length =
            usize::try_from(random_length).map_err(|_| UtilError::OutOfRange("Range"))?;

        result = format!(
            "{}{}{}",
            &prefix[..start],
            random_text_long(random_length),
            &prefix[end + 1..]
        );
        if exist_random_string(&result) {
            result = get_random_name(&result, object_count)?;
        }
    }
    if result.to_ascii_lowercase().contains("{objectcount}") {
        result = result.replace("{ObjectCount}", &format!("_{}", fmt5(object_count.into())));
    }
    Ok(result.replace("//", "/-"))
}

/// prefix 설정에 따라 스레드 접두어와 오브젝트 순번을 조합한 이름을 만든다(`Utility.GetRandomObjectName`).
pub fn get_random_object_name(
    prefix: &str,
    thread_prefix: &str,
    object_count: i32,
) -> Result<String, UtilError> {
    let count = fmt5(object_count.into());
    if prefix.trim().is_empty() {
        Ok(format!("{thread_prefix}/FILE_{count}"))
    } else if exist_random_string(prefix) {
        let name = get_random_name(prefix, object_count)?;
        if thread_prefix.trim().is_empty() {
            Ok(name)
        } else {
            Ok(format!("{thread_prefix}/{name}"))
        }
    } else {
        Ok(format!("{thread_prefix}/{prefix}{count}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_units() {
        assert_eq!(size_to_long("").unwrap(), 0);
        assert_eq!(size_to_long("  ").unwrap(), 0);
        assert_eq!(size_to_long("1K").unwrap(), 1000);
        assert_eq!(size_to_long("1k").unwrap(), 1000);
        assert_eq!(size_to_long("10M").unwrap(), 10_000_000);
        assert_eq!(size_to_long("1G").unwrap(), 1_000_000_000);
        assert_eq!(size_to_long("2TB").unwrap(), 2_000_000_000_000);
        assert_eq!(size_to_long("1Ki").unwrap(), 1024);
        assert_eq!(size_to_long("1MiB").unwrap(), 1024 * 1024);
        assert_eq!(size_to_long("1024").unwrap(), 1024);
        assert_eq!(size_to_long("-5").unwrap(), -5);
        assert_eq!(size_to_long("1 K").unwrap(), 1000);
        assert!(size_to_long("abc").is_err());
        assert!(size_to_long("K").is_err());
        assert!(size_to_long("1.5M").is_err());
    }

    #[test]
    fn dotnet_parsers() {
        assert_eq!(try_parse_i32("+5"), Some(5));
        assert_eq!(try_parse_i32(" 7 "), Some(7));
        assert_eq!(try_parse_i32("2147483648"), None);
        assert_eq!(try_parse_i32(""), None);
        assert_eq!(try_parse_bool(" TRUE "), Some(true));
        assert_eq!(try_parse_bool("False"), Some(false));
        assert_eq!(try_parse_bool("yes"), None);
        assert_eq!(try_parse_bool("1"), None);
    }

    #[test]
    fn random_names() {
        assert_eq!(
            get_random_object_name("", "TH", 7).unwrap(),
            "TH/FILE_00007"
        );
        assert_eq!(get_random_object_name("a_", "TH", 7).unwrap(), "TH/a_00007");
        let name = get_random_object_name("x/{Random:5}/{ObjectCount}", "TH", 3).unwrap();
        assert!(
            name.starts_with("TH/x/") && name.ends_with("/_00003"),
            "{name}"
        );
        assert_eq!(name.len(), "TH/x/".len() + 5 + "/_00003".len());
        let name = get_random_object_name("{random}", "", 3).unwrap();
        assert_eq!(name.len(), 10);
        assert!(get_random_name("{Random", 1).is_err());
    }
}
