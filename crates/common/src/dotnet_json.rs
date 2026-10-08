//! TESTCore `JsonExtensions.ToJsonString()`(`System.Text.Json`, `WriteIndented = true`, 기본 인코더)과
//! 같은 문자열을 만든다. 콘솔·로그에 그대로 찍히는 출력이라 바이트 단위로 맞춘다.
//!
//! - 들여쓰기 2칸, 줄바꿈은 `Environment.NewLine`(Windows `\r\n`, 그 밖 `\n`), `"키": 값`.
//! - 빈 배열·객체는 `[]`, `{}`.
//! - 문자열: `\\`, `\r`, `\n`, `\t`, `\b`, `\f`는 짧은 형식, 그 밖의 제어 문자와 `"<>&'+``, 비 ASCII 문자는
//!   `\uXXXX`(대문자 16진수, BMP 밖은 서로게이트 쌍)로 쓴다.
//! - 실수: 가장 짧은 왕복 표현. 10의 지수가 17 이상이거나 -5 이하이면 `1E+17`, `1E-05` 형식.

use std::io;

use serde::Serialize;
use serde_json::ser::{CharEscape, Formatter, Serializer};

#[cfg(windows)]
pub const NEW_LINE: &str = "\r\n";
#[cfg(not(windows))]
pub const NEW_LINE: &str = "\n";

/// `ToJsonString()`과 같은 형식으로 직렬화한다.
pub fn to_dotnet_json<T: Serialize + ?Sized>(value: &T) -> String {
    write_json(value, DotnetFormatter::indented())
}

/// ASP.NET·`HttpClient` JSON 확장(`JsonSerializerDefaults.Web`)의 출력: 속성 이름 camelCase(`JsonNamingPolicy.CamelCase`),
/// 들여쓰기 없음, 문자열 이스케이프·실수 형식은 [`to_dotnet_json`]과 같다. 형식의 serde 이름은 .NET 속성 이름(PascalCase)으로 둔다.
pub fn to_web_json<T: Serialize + ?Sized>(value: &T) -> String {
    write_json(
        value,
        DotnetFormatter {
            compact: true,
            camel_case: true,
            ..DotnetFormatter::default()
        },
    )
}

/// 웹 JSON(camelCase)을 읽는다. 속성 이름의 첫 글자를 대문자로 바꿔 .NET 속성 이름(serde 이름)에 맞춘다
/// (`PropertyNameCaseInsensitive` 중 실제로 오가는 camelCase·PascalCase를 다룬다).
pub fn from_web_json<T: serde::de::DeserializeOwned>(text: &str) -> serde_json::Result<T> {
    let mut value: serde_json::Value = serde_json::from_str(text)?;
    pascalize_keys(&mut value);
    serde_json::from_value(value)
}

fn pascalize_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let entries = std::mem::take(map);
            for (key, mut item) in entries {
                pascalize_keys(&mut item);
                let mut chars = key.chars();
                let key = match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => key,
                };
                map.insert(key, item);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(pascalize_keys),
        _ => {}
    }
}

/// `JsonNamingPolicy.CamelCase.ConvertName`: 앞쪽 대문자 연속을 소문자로(다음 글자가 소문자인 마지막 대문자는 남긴다).
pub fn camel_case(name: &str) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    if chars.first().is_none_or(|c| !c.is_uppercase()) {
        return name.to_string();
    }
    for i in 0..chars.len() {
        if i == 1 && !chars[i].is_uppercase() {
            break;
        }
        let has_next = i + 1 < chars.len();
        if i > 0 && has_next && !chars[i + 1].is_uppercase() {
            if chars[i + 1] == ' ' {
                chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
            }
            break;
        }
        chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
    }
    chars.into_iter().collect()
}

fn write_json<T: Serialize + ?Sized>(value: &T, formatter: DotnetFormatter) -> String {
    let mut out = Vec::new();
    let mut serializer = Serializer::with_formatter(&mut out, formatter);
    value
        .serialize(&mut serializer)
        .expect("메모리 직렬화는 실패하지 않는다");
    String::from_utf8(out).expect("출력은 ASCII다")
}

#[derive(Default)]
struct DotnetFormatter {
    indent: usize,
    has_value: bool,
    /// 들여쓰기·줄바꿈 없이 쓴다.
    compact: bool,
    /// 속성 이름을 camelCase로 쓴다.
    camel_case: bool,
    /// 속성 이름을 쓰는 중.
    in_key: bool,
}

impl DotnetFormatter {
    fn indented() -> Self {
        Self::default()
    }

    fn newline<W: ?Sized + io::Write>(&self, writer: &mut W) -> io::Result<()> {
        if self.compact {
            return Ok(());
        }
        writer.write_all(NEW_LINE.as_bytes())?;
        for _ in 0..self.indent {
            writer.write_all(b"  ")?;
        }
        Ok(())
    }
}

fn must_escape(c: char) -> bool {
    !c.is_ascii() || c.is_ascii_control() || matches!(c, '"' | '<' | '>' | '&' | '\'' | '+' | '`')
}

fn write_unicode<W: ?Sized + io::Write>(writer: &mut W, c: char) -> io::Result<()> {
    let mut units = [0u16; 2];
    for unit in c.encode_utf16(&mut units) {
        write!(writer, "\\u{unit:04X}")?;
    }
    Ok(())
}

/// 이스케이프하지 않고 그대로 쓰는 JSON 문자열(System.Text.Json이 인코더 없이 쓰는 날짜 등). `text`에 `"`·`\`·제어 문자가 없어야 한다.
pub fn raw_string<S: serde::Serializer>(text: &str, serializer: S) -> Result<S::Ok, S::Error> {
    let raw = serde_json::value::RawValue::from_string(format!("\"{text}\""))
        .map_err(serde::ser::Error::custom)?;
    raw.serialize(serializer)
}

/// .NET `double.ToString("R")` 형식(가장 짧은 왕복 표현).
pub fn double_text(value: f64) -> String {
    format_f64(value)
}

fn format_f64(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.into();
    }
    // `{:e}`는 가장 짧은 왕복 자릿수를 `d.ddde±x` 형식으로 준다.
    let sci = format!("{value:e}");
    let (mantissa, exponent) = sci.split_once('e').expect("지수 표기");
    let exponent: i32 = exponent.parse().expect("지수");
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    if !(-4..17).contains(&exponent) {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let exp_sign = if exponent < 0 { '-' } else { '+' };
        return format!("{sign}{first}{fraction}E{exp_sign}{:02}", exponent.abs());
    }
    let point = exponent + 1; // 정수부 자릿수
    let body = if point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        let (int, frac) = digits.split_at(point as usize);
        format!("{int}.{frac}")
    };
    format!("{sign}{body}")
}

impl Formatter for DotnetFormatter {
    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(format_f64(value).as_bytes())
    }

    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        // .NET float도 가장 짧은 왕복 표현을 쓴다. f32 자릿수로 만든 뒤 f64로 다시 읽어 형식만 맞춘다.
        let shortest: f64 = format!("{value:e}").parse().expect("실수");
        writer.write_all(format_f64(shortest).as_bytes())
    }

    fn write_string_fragment<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        let renamed;
        let fragment = if self.in_key && self.camel_case {
            renamed = camel_case(fragment);
            renamed.as_str()
        } else {
            fragment
        };
        let mut start = 0;
        for (index, c) in fragment.char_indices() {
            if must_escape(c) {
                writer.write_all(&fragment.as_bytes()[start..index])?;
                write_unicode(writer, c)?;
                start = index + c.len_utf8();
            }
        }
        writer.write_all(&fragment.as_bytes()[start..])
    }

    fn write_char_escape<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        escape: CharEscape,
    ) -> io::Result<()> {
        let text: &[u8] = match escape {
            CharEscape::Quote => b"\\u0022",
            CharEscape::ReverseSolidus => b"\\\\",
            CharEscape::Solidus => b"/",
            CharEscape::Backspace => b"\\b",
            CharEscape::FormFeed => b"\\f",
            CharEscape::LineFeed => b"\\n",
            CharEscape::CarriageReturn => b"\\r",
            CharEscape::Tab => b"\\t",
            CharEscape::AsciiControl(byte) => {
                return write!(writer, "\\u{byte:04X}");
            }
        };
        writer.write_all(text)
    }

    fn begin_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.indent += 1;
        self.has_value = false;
        writer.write_all(b"[")
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.indent -= 1;
        if self.has_value {
            self.newline(writer)?;
        }
        writer.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if !first {
            writer.write_all(b",")?;
        }
        self.newline(writer)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, _writer: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.indent -= 1;
        if self.has_value {
            self.newline(writer)?;
        }
        writer.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.in_key = true;
        if !first {
            writer.write_all(b",")?;
        }
        self.newline(writer)
    }

    fn end_object_key<W: ?Sized + io::Write>(&mut self, _writer: &mut W) -> io::Result<()> {
        self.in_key = false;
        Ok(())
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(if self.compact { b":" } else { b": " })
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, _writer: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        let cases = [
            (1.5, "1.5"),
            (0.1, "0.1"),
            (1e20, "1E+20"),
            (1e-7, "1E-07"),
            (1e16, "10000000000000000"),
            (1e17, "1E+17"),
            (0.0001, "0.0001"),
            (1e-5, "1E-05"),
            (-2.5e-10, "-2.5E-10"),
            (100.0, "100"),
            (-0.0, "-0"),
            (123456789.125, "123456789.125"),
            (1.7976931348623157e308, "1.7976931348623157E+308"),
            (5e-324, "5E-324"),
        ];
        for (value, expected) in cases {
            assert_eq!(format_f64(value), expected, "{value:e}");
        }
    }

    #[test]
    fn camel_case_like_dotnet() {
        for (name, expected) in [
            ("URL", "url"),
            ("ETagCheck", "eTagCheck"),
            ("IsAdmin", "isAdmin"),
            ("StartAtUtc", "startAtUtc"),
            ("runId", "runId"),
            ("A", "a"),
            ("ABCDef", "abcDef"),
        ] {
            assert_eq!(camel_case(name), expected);
        }
    }

    #[test]
    fn web_json_round_trip() {
        #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Inner {
            #[serde(rename = "URL", alias = "Url")]
            url: String,
        }
        #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Outer {
            run_id: String,
            #[serde(rename = "ETagCheck")]
            e_tag_check: bool,
            elapsed_seconds: f64,
            user: Option<Inner>,
        }
        let value = Outer {
            run_id: "r".into(),
            e_tag_check: true,
            elapsed_seconds: 12.0,
            user: Some(Inner {
                url: "http://x".into(),
            }),
        };
        let text = to_web_json(&value);
        assert_eq!(
            text,
            r#"{"runId":"r","eTagCheck":true,"elapsedSeconds":12,"user":{"url":"http://x"}}"#
        );
        assert_eq!(from_web_json::<Outer>(&text).unwrap(), value);
    }

    #[test]
    fn nested_empty_containers() {
        #[derive(Serialize)]
        struct S {
            a: Vec<i32>,
            b: Vec<Vec<i32>>,
        }
        let json = to_dotnet_json(&S {
            a: vec![],
            b: vec![vec![]],
        });
        let expected = ["{", "  \"a\": [],", "  \"b\": [", "    []", "  ]", "}"].join(NEW_LINE);
        assert_eq!(json, expected);
    }
}
