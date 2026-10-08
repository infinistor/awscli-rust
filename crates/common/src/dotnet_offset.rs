//! .NET `DateTimeOffset`(UTC)의 JSON·`"O"` 문자열. 분산 실행 계약(`StartAtUtc` 등)과 CSV 시각에 쓴다.
//!
//! - 쓰기: `yyyy-MM-ddTHH:mm:ss.fffffff+00:00`(`ToUniversalTime().ToString("O")`). System.Text.Json은 소수 끝의 0을
//!   지우므로([`DotnetDateTimeOffset::to_json_text`]) 둘을 구분한다.
//! - 읽기: RFC 3339(오프셋·`Z`, 소수 0~7자리)를 UTC로 바꾼다. .NET 틱(100ns) 단위로 자른다.

use std::fmt;

use chrono::{DateTime, SubsecRound, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DotnetDateTimeOffset(DateTime<Utc>);

impl DotnetDateTimeOffset {
    /// 틱(100ns) 단위로 자른 시각.
    pub fn new(value: DateTime<Utc>) -> Self {
        let nanos = value.timestamp_subsec_nanos() / 100 * 100;
        Self(value.trunc_subsecs(0) + chrono::Duration::nanoseconds(i64::from(nanos)))
    }

    /// `default(DateTimeOffset)`(`0001-01-01T00:00:00+00:00`).
    pub fn min_value() -> Self {
        Self(
            chrono::NaiveDate::from_ymd_opt(1, 1, 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .expect("유효한 날짜")
                .and_utc(),
        )
    }

    /// `DateTimeOffset.UtcNow`.
    pub fn now() -> Self {
        Self::new(Utc::now())
    }

    pub fn value(&self) -> DateTime<Utc> {
        self.0
    }

    /// `ToString("O")`: 소수 7자리, `+00:00`.
    pub fn to_o_text(&self) -> String {
        let ticks = self.0.timestamp_subsec_nanos() / 100;
        format!("{}.{ticks:07}+00:00", self.0.format("%Y-%m-%dT%H:%M:%S"))
    }

    /// System.Text.Json 출력: 소수 끝의 0을 지우고(모두 0이면 소수점도), `+00:00`.
    pub fn to_json_text(&self) -> String {
        let ticks = self.0.timestamp_subsec_nanos() / 100;
        let mut text = self.0.format("%Y-%m-%dT%H:%M:%S").to_string();
        if ticks > 0 {
            text.push('.');
            text.push_str(format!("{ticks:07}").trim_end_matches('0'));
        }
        text.push_str("+00:00");
        text
    }

    /// RFC 3339 문자열을 읽는다.
    pub fn parse(text: &str) -> Result<Self, chrono::ParseError> {
        DateTime::parse_from_rfc3339(text).map(|t| Self::new(t.with_timezone(&Utc)))
    }
}

impl From<DateTime<Utc>> for DotnetDateTimeOffset {
    fn from(value: DateTime<Utc>) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for DotnetDateTimeOffset {
    /// `ToString("O")`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_o_text())
    }
}

impl Serialize for DotnetDateTimeOffset {
    /// System.Text.Json은 날짜를 인코더를 거치지 않고 쓴다(`+`를 `002B`로 바꾸지 않는다).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::dotnet_json::raw_string(&self.to_json_text(), serializer)
    }
}

impl<'de> Deserialize<'de> for DotnetDateTimeOffset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_round_trips() {
        let t = DotnetDateTimeOffset::parse("2026-10-08T01:02:03.1234567+09:00").unwrap();
        assert_eq!(t.to_o_text(), "2026-10-07T16:02:03.1234567+00:00");
        assert_eq!(t.to_json_text(), "2026-10-07T16:02:03.1234567+00:00");
        let whole = DotnetDateTimeOffset::parse("2026-10-07T16:02:03Z").unwrap();
        assert_eq!(whole.to_o_text(), "2026-10-07T16:02:03.0000000+00:00");
        assert_eq!(whole.to_json_text(), "2026-10-07T16:02:03+00:00");
        let half = DotnetDateTimeOffset::parse("2026-10-07T16:02:03.5+00:00").unwrap();
        assert_eq!(half.to_json_text(), "2026-10-07T16:02:03.5+00:00");
        assert_eq!(DotnetDateTimeOffset::parse(&t.to_o_text()).unwrap(), t);
    }
}
