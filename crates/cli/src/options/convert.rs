//! Mono.Options가 `TypeDescriptor.GetConverter(T).ConvertFromString`으로 값을 바꾸는 규칙.

/// `Int32Converter.ConvertFromString`.
///
/// 앞뒤 공백을 자른 뒤 `#`·`0x`·`&h`로 시작하면 16진수(`Convert.ToInt32(s, 16)`), 아니면
/// `int.Parse(s, NumberStyles.Integer)`. 16진수는 32비트를 부호 없이 읽어 그대로 `int`로 바꾼다.
pub(crate) fn to_int32(value: &str) -> Option<i32> {
    let text = value.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(rest) = text.strip_prefix('#') {
        return hex_to_int32(rest);
    }
    let lower = text.get(..2).map(str::to_ascii_lowercase);
    if matches!(lower.as_deref(), Some("0x" | "&h")) {
        return hex_to_int32(&text[2..]);
    }
    parse_integer(text)
}

/// `Convert.ToInt32(value, 16)`(`ParseNumbers.StringToInt`, `IsTight`): 공백 없이 `+`와 `0x`를 한 번
/// 허용하고, `-`는 받지 않는다.
fn hex_to_int32(value: &str) -> Option<i32> {
    let bytes = value.as_bytes();
    let mut i = 0;
    match bytes.first()? {
        b'-' => return None,
        b'+' => i += 1,
        _ => {}
    }
    if i + 1 < bytes.len() && bytes[i] == b'0' && matches!(bytes[i + 1], b'x' | b'X') {
        i += 2;
    }
    let start = i;
    let mut result: u32 = 0;
    while let Some(digit) = bytes.get(i).and_then(|b| (*b as char).to_digit(16)) {
        if result > 0x0FFF_FFFF {
            return None;
        }
        result = result * 16 + digit;
        i += 1;
    }
    if i == start || i < bytes.len() {
        return None;
    }
    Some(result as i32)
}

/// `int.Parse(s, NumberStyles.Integer)`: 앞뒤 공백(0x09~0x0D, 0x20), 앞 부호, ASCII 숫자,
/// 끝의 `'\0'`을 허용한다.
fn parse_integer(value: &str) -> Option<i32> {
    let text = value
        .trim_end_matches('\0')
        .trim_matches(|c: char| matches!(c, '\u{9}'..='\u{D}' | ' '));
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut result: i64 = 0;
    for b in digits.bytes() {
        result = result * 10 + i64::from(b - b'0');
        if result > i64::from(i32::MAX) + 1 {
            return None;
        }
    }
    i32::try_from(if negative { -result } else { result }).ok()
}

/// `BooleanConverter.ConvertFromString`(`bool.Parse`): 앞뒤 공백과 `'\0'`을 무시하고 대소문자 구분 없이
/// `true`/`false`만 받는다.
pub(crate) fn to_bool(value: &str) -> Option<bool> {
    let text = value.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if text.eq_ignore_ascii_case("true") {
        Some(true)
    } else if text.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// `long.Parse` 실패를 .NET 예외 형식과 메시지로 바꾼다(`Utility.SizeToLong`이 던지는 예외).
pub(crate) fn long_parse_exception(input: &str) -> (&'static str, String) {
    let text = input
        .trim_end_matches('\0')
        .trim_matches(|c: char| matches!(c, '\u{9}'..='\u{D}' | ' '));
    let digits = text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('+'))
        .unwrap_or(text);
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        (
            "System.OverflowException",
            "Value was either too large or too small for an Int64.".to_string(),
        )
    } else {
        (
            "System.FormatException",
            format!("The input string '{input}' was not in a correct format."),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int32() {
        for (text, expected) in [
            ("12", Some(12)),
            (" 12 ", Some(12)),
            ("0x1F", Some(31)),
            ("#ff", Some(255)),
            ("&H10", Some(16)),
            ("0xFFFFFFFF", Some(-1)),
            ("0x100000000", None),
            ("#+a", Some(10)),
            ("0x0x10", Some(16)),
            ("#-1", None),
            ("-2147483648", Some(i32::MIN)),
            ("2147483648", None),
            ("- 5", None),
            ("12\0", Some(12)),
            ("１２", None),
            ("", None),
        ] {
            assert_eq!(to_int32(text), expected, "{text:?}");
        }
    }

    #[test]
    fn boolean() {
        assert_eq!(to_bool(" True "), Some(true));
        assert_eq!(to_bool("fAlSe"), Some(false));
        assert_eq!(to_bool("1"), None);
    }
}
