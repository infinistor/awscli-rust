//! .NET `DateTime`(값 + `Kind`)을 흉내 낸다. `XmlSerializer`가 읽은 `xs:dateTime`을
//! `System.Text.Json`이 쓰는 형식 그대로 출력하기 위해 필요하다.
//!
//! - `XmlConvert.ToDateTime(s, RoundtripKind)` 규칙으로 읽는다.
//!   `Z`로 끝나면 `Utc`, 오프셋(`+09:00`)이 있으면 그 시각을 이 PC의 현지 시각으로 바꾼 `Local`,
//!   둘 다 없으면 `Unspecified`.
//! - JSON 출력: `yyyy-MM-ddTHH:mm:ss[.fffffff]` 뒤에 `Utc`면 `Z`, `Local`이면 현지 오프셋(`+09:00`),
//!   `Unspecified`면 아무것도 붙이지 않는다. 소수부는 끝의 0을 지우고, 0이면 생략한다.

use std::fmt;

use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeKind {
    Unspecified,
    Utc,
    /// 현지 시각과 그때의 UTC 오프셋(초).
    Local(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotnetDateTime {
    pub value: NaiveDateTime,
    pub kind: DateTimeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("The string '{0}' is not a valid AllXsd value.")]
pub struct DateTimeParseError(pub String);

impl Default for DotnetDateTime {
    /// `DateTime.MinValue` (`0001-01-01T00:00:00`, `Unspecified`).
    fn default() -> Self {
        Self {
            value: NaiveDate::from_ymd_opt(1, 1, 1)
                .expect("유효한 날짜")
                .and_hms_opt(0, 0, 0)
                .expect("유효한 시각"),
            kind: DateTimeKind::Unspecified,
        }
    }
}

impl DotnetDateTime {
    pub fn utc(value: DateTime<Utc>) -> Self {
        Self {
            value: value.naive_utc(),
            kind: DateTimeKind::Utc,
        }
    }

    /// 지정한 시각을 이 PC의 현지 시각(`Kind = Local`)으로 바꾼다.
    pub fn local<Tz: TimeZone>(value: DateTime<Tz>) -> Self {
        let local = value.with_timezone(&Local);
        Self {
            value: local.naive_local(),
            kind: DateTimeKind::Local(local.offset().fix().local_minus_utc()),
        }
    }

    /// `XmlConvert.ToDateTime(text, XmlDateTimeSerializationMode.RoundtripKind)`.
    pub fn parse_xml(text: &str) -> Result<Self, DateTimeParseError> {
        let err = || DateTimeParseError(text.to_string());
        let trimmed = text.trim();
        let (body, zone) = split_zone(trimmed);
        let value = parse_naive(body).ok_or_else(err)?;
        match zone {
            None => Ok(Self {
                value,
                kind: DateTimeKind::Unspecified,
            }),
            Some(Zone::Utc) => Ok(Self {
                value,
                kind: DateTimeKind::Utc,
            }),
            Some(Zone::Offset(seconds)) => {
                let offset = FixedOffset::east_opt(seconds).ok_or_else(err)?;
                let instant = offset
                    .from_local_datetime(&value)
                    .single()
                    .ok_or_else(err)?;
                Ok(Self::local(instant))
            }
        }
    }

    /// `System.Text.Json` 출력 문자열.
    pub fn to_json_text(&self) -> String {
        let mut text = self.value.format("%Y-%m-%dT%H:%M:%S").to_string();
        let ticks = self.value.and_utc().timestamp_subsec_nanos() / 100;
        if ticks > 0 {
            let fraction = format!("{ticks:07}");
            text.push('.');
            text.push_str(fraction.trim_end_matches('0'));
        }
        match self.kind {
            DateTimeKind::Unspecified => {}
            DateTimeKind::Utc => text.push('Z'),
            DateTimeKind::Local(seconds) => {
                let sign = if seconds < 0 { '-' } else { '+' };
                let minutes = seconds.abs() / 60;
                text.push_str(&format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60));
            }
        }
        text
    }

    /// `DateTime.Now`.
    pub fn now() -> Self {
        Self::local(Local::now())
    }

    /// `new DateTimeOffset(value).ToUnixTimeSeconds()`. `Unspecified`는 이 PC의 현지 시각으로 본다.
    /// 오프셋을 적용한 UTC 시각이 1년보다 앞서면 .NET처럼 실패한다(예: KST에서 `DateTime.MinValue`).
    pub fn to_unix_seconds(&self) -> Result<i64, String> {
        let offset_seconds = match self.kind {
            DateTimeKind::Utc => 0,
            DateTimeKind::Local(seconds) => seconds,
            DateTimeKind::Unspecified => Local
                .offset_from_local_datetime(&self.value)
                .earliest()
                .map(|o| o.fix().local_minus_utc())
                .unwrap_or(0),
        };
        let utc = self.value - chrono::Duration::seconds(i64::from(offset_seconds));
        if chrono::Datelike::year(&utc) < 1 {
            return Err(
                "The UTC time represented when the offset is applied must be between year 0 and 10,000. (Parameter 'offset')"
                    .to_string(),
            );
        }
        Ok(utc.and_utc().timestamp())
    }

    /// .NET 사용자 지정 형식 문자열 일부(`yyyy`, `MM`, `dd`, `HH`, `mm`, `ss`)로 출력한다.
    /// 원본 코드가 쓰는 형식만 지원한다. 그 밖의 문자는 그대로 둔다.
    pub fn format(&self, pattern: &str) -> String {
        let v = &self.value;
        let mut out = String::new();
        let mut rest = pattern;
        while !rest.is_empty() {
            let token: Option<(usize, String)> = [
                ("yyyy", format!("{:04}", chrono::Datelike::year(v))),
                ("MM", format!("{:02}", chrono::Datelike::month(v))),
                ("dd", format!("{:02}", chrono::Datelike::day(v))),
                ("HH", format!("{:02}", chrono::Timelike::hour(v))),
                ("mm", format!("{:02}", chrono::Timelike::minute(v))),
                ("ss", format!("{:02}", chrono::Timelike::second(v))),
            ]
            .into_iter()
            .find(|(t, _)| rest.starts_with(t))
            .map(|(t, s)| (t.len(), s));
            match token {
                Some((len, text)) => {
                    out.push_str(&text);
                    rest = &rest[len..];
                }
                None => {
                    let c = rest.chars().next().expect("비어 있지 않음");
                    out.push(c);
                    rest = &rest[c.len_utf8()..];
                }
            }
        }
        out
    }
}

impl fmt::Display for DotnetDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json_text())
    }
}

/// `System.Text.Json`의 `DateTime` 변환기는 문자열 이스케이프를 거치지 않는다(`+09:00`의 `+`를
/// `+`로 바꾸지 않는다). 값에는 숫자와 `-:T.Z+`만 있으므로 그대로 쓴다.
impl Serialize for DotnetDateTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let raw = serde_json::value::RawValue::from_string(format!("\"{}\"", self.to_json_text()))
            .map_err(serde::ser::Error::custom)?;
        raw.serialize(serializer)
    }
}

enum Zone {
    Utc,
    Offset(i32),
}

fn split_zone(text: &str) -> (&str, Option<Zone>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(Zone::Utc));
    }
    // 날짜 부분의 '-'와 헷갈리지 않도록 시각(T) 뒤에서만 오프셋을 찾는다.
    if let Some(t) = text.find('T')
        && let Some(index) = text[t..].rfind(['+', '-']).map(|i| i + t)
    {
        let zone = &text[index + 1..];
        if let Some((h, m)) = zone.split_once(':')
            && let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>())
            && h.to_string().len() <= 2
        {
            let seconds = (h * 60 + m) * 60;
            let seconds = if text.as_bytes()[index] == b'-' {
                -seconds
            } else {
                seconds
            };
            return (&text[..index], Some(Zone::Offset(seconds)));
        }
    }
    (text, None)
}

fn parse_naive(text: &str) -> Option<NaiveDateTime> {
    let (main, fraction) = match text.split_once('.') {
        Some((main, fraction)) => (main, Some(fraction)),
        None => (text, None),
    };
    let mut value = NaiveDateTime::parse_from_str(main, "%Y-%m-%dT%H:%M:%S").ok()?;
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // .NET DateTime 해상도(100ns, 7자리)까지만 쓴다.
        let digits: String = fraction.chars().take(7).collect();
        let ticks: u32 = format!("{digits:0<7}").parse().ok()?;
        value += chrono::Duration::nanoseconds(i64::from(ticks) * 100);
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_and_unspecified() {
        let utc = DotnetDateTime::parse_xml("2024-03-05T07:08:09.123Z").unwrap();
        assert_eq!(utc.to_json_text(), "2024-03-05T07:08:09.123Z");
        let plain = DotnetDateTime::parse_xml("2024-03-05T07:08:09").unwrap();
        assert_eq!(plain.to_json_text(), "2024-03-05T07:08:09");
        assert_eq!(
            DotnetDateTime::default().to_json_text(),
            "0001-01-01T00:00:00"
        );
        let fraction = DotnetDateTime::parse_xml("2024-03-05T07:08:09.1234567Z").unwrap();
        assert_eq!(fraction.to_json_text(), "2024-03-05T07:08:09.1234567Z");
    }

    #[test]
    fn offset_becomes_local() {
        let value = DotnetDateTime::parse_xml("2024-12-31T23:59:59+09:00").unwrap();
        let expected = DotnetDateTime::local(
            FixedOffset::east_opt(9 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 12, 31, 23, 59, 59)
                .unwrap(),
        );
        assert_eq!(value, expected);
        assert!(matches!(value.kind, DateTimeKind::Local(_)));
    }

    #[test]
    fn invalid() {
        assert!(DotnetDateTime::parse_xml("yesterday").is_err());
        assert!(DotnetDateTime::parse_xml("2024-03-05T07:08:09.").is_err());
    }

    #[test]
    fn custom_format() {
        let value = DotnetDateTime::parse_xml("2024-03-05T07:08:09Z").unwrap();
        // 원본 ListBucketTagSearch 출력 형식(월·분 자리가 뒤바뀐 형식 그대로).
        assert_eq!(value.format("yyyy-mm-dd HH:MM:ss"), "2024-08-05 07:03:09");
    }
}
