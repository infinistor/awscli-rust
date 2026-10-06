//! TESTCore `Data/Config/EnumBucketTypes.cs` 이식: 버킷 생성 방식과 오브젝트 이름 생성 규칙.

use std::sync::LazyLock;

use chrono::{Datelike, Duration, Local, NaiveDate, Timelike};
use rand::Rng;
use regex::Regex;
use serde::Serialize;

use crate::util::{UtilError, fmt5, get_random_object_name};

const DIVISION_COUNT: i32 = 1000;

/// 버킷 생성 방식.
///
/// C# `enum`은 정의되지 않은 정수도 담을 수 있고 `JsonSerializer`(문자열 변환기 없음)가 숫자로 내보내므로,
/// 원본과 같이 정수 값을 그대로 보관하는 새타입으로 만든다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct EnumBucketTypes(pub i32);

#[allow(non_upper_case_globals)]
impl EnumBucketTypes {
    /// 빈값
    pub const Empty: Self = Self(-1);
    /// 기본 설정. `ThreadPrefix/{ObjectCount/DivisionCount}/ObjectPrefix_ObjectCount`
    pub const None: Self = Self(0);
    /// 단일버킷 업로드. `ThreadPrefix-ObjectPrefix_ObjectCount`
    pub const One: Self = Self(1);
    /// 쓰레드당 버킷 1개로 업로드.
    pub const Thread: Self = Self(2);
    /// 날짜별 하나의 폴더. `ThreadPrefix/Year/Month/Day/ObjectPrefix_ObjectCount`
    pub const Time: Self = Self(3);
    /// 단일 버킷에 생성. `{Random}`, `{ObjectCount}`, 경로 패턴(`DIR{0..9}`)을 처리한다.
    pub const Prefix: Self = Self(4);
    /// 현재시간 기준 생성. `ThreadPrefix/Year/Month/Day/Hour/Minute/ObjectPrefix_ObjectCount`
    pub const Now: Self = Self(5);

    /// 확장 메서드 `ToString(this EnumBucketTypes)`. 정의되지 않은 값은 `"Unknown"`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::None => "None",
            Self::One => "One",
            Self::Thread => "Thread",
            Self::Time => "Time",
            Self::Prefix => "Prefix",
            Self::Now => "Now",
            _ => "Unknown",
        }
    }

    /// `ToEnumBucketTypes(this string)`: 이름이 맞지 않으면 `Empty`.
    pub fn from_name(name: &str) -> Self {
        match name {
            "None" => Self::None,
            "One" => Self::One,
            "Thread" => Self::Thread,
            "Time" => Self::Time,
            "Prefix" => Self::Prefix,
            "Now" => Self::Now,
            _ => Self::Empty,
        }
    }

    /// `ToEnumBucketTypes(this int)`: 범위를 벗어나면 `Empty`.
    pub fn from_int(value: i32) -> Self {
        match value {
            0..=5 => Self(value),
            _ => Self::Empty,
        }
    }

    /// 타입에 따라 다음 오브젝트 이름을 만든다(`GetNextObjectName`). `division_count`는 0이 아니어야 한다.
    pub fn next_object_name(
        self,
        thread_prefix: &str,
        object_prefix: &str,
        object_count: i32,
        division_count: i32,
    ) -> Result<String, UtilError> {
        let count5 = fmt5(object_count.into());
        match self {
            Self::One => Ok(format!("{thread_prefix}-{object_prefix}_{count5}")),
            Self::Thread => {
                if division_count == 0 {
                    return Err(UtilError::DivideByZero);
                }
                // 원본은 double로 나눈 뒤 Truncate한 값을 그대로 문자열로 만든다.
                let q = (f64::from(object_count) / f64::from(division_count)).trunc();
                Ok(format!("{q}/{object_prefix}_{count5}"))
            }
            Self::Time => {
                let rem = checked_rem(object_count, division_count)?;
                let day = NaiveDate::from_ymd_opt(2025, 1, 1)
                    .and_then(|d| d.checked_add_signed(Duration::days(rem.into())))
                    .ok_or(UtilError::OutOfRange("AddDays"))?;
                Ok(format!(
                    "{thread_prefix}/{}/{:02}/{:02}/{object_prefix}_{}",
                    day.year(),
                    day.month(),
                    day.day(),
                    fmt5(rem.into())
                ))
            }
            Self::Prefix => prefix_object_name(object_prefix, thread_prefix, object_count),
            Self::Now => {
                let now = Local::now();
                Ok(format!(
                    "{thread_prefix}/{}/{:02}/{:02}/{:02}/{:02}/{object_prefix}_{count5}",
                    now.year(),
                    now.month(),
                    now.day(),
                    now.hour(),
                    now.minute()
                ))
            }
            _ => {
                let q = checked_div(object_count, division_count)?;
                Ok(format!(
                    "{thread_prefix}/{}/{object_prefix}_{count5}",
                    fmt5(q.into())
                ))
            }
        }
    }

    /// 타입에 따라 임의의 오브젝트 이름을 만든다(`GetRandomObjectName`).
    pub fn random_object_name(
        self,
        object_prefix: &str,
        thread_prefix: &str,
        object_count: i32,
        division_count: i32,
    ) -> Result<String, UtilError> {
        let mut rng = rand::rng();
        if self == Self::Prefix {
            // 경로 패턴의 경우 랜덤한 objectCount 생성: Next(0, Max(1, objectCount))
            let random_count = rng.random_range(0..object_count.max(1));
            return prefix_object_name(object_prefix, thread_prefix, random_count);
        }
        // Random.Next(0, n): n == 0이면 0, 음수면 예외.
        let random_count = match object_count {
            0 => 0,
            n if n > 0 => rng.random_range(0..n),
            _ => return Err(UtilError::OutOfRange("Next")),
        };
        self.next_object_name(thread_prefix, object_prefix, random_count, division_count)
    }
}

/// `DIVISION_COUNT` 기본값.
pub const DEFAULT_DIVISION_COUNT: i32 = DIVISION_COUNT;

fn checked_rem(a: i32, b: i32) -> Result<i32, UtilError> {
    if b == 0 {
        return Err(UtilError::DivideByZero);
    }
    a.checked_rem(b).ok_or(UtilError::OutOfRange("Overflow"))
}

fn checked_div(a: i32, b: i32) -> Result<i32, UtilError> {
    if b == 0 {
        return Err(UtilError::DivideByZero);
    }
    a.checked_div(b).ok_or(UtilError::OutOfRange("Overflow"))
}

static PATH_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\{(\d+)\.\.(\d+)(?::(\d+))?\}").expect("정규식"));
static PATTERN_SEGMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([^/{}]+)\{(\d+)\.\.(\d+)(?::(\d+))?\}").expect("정규식"));

/// Prefix 타입의 오브젝트 이름: 경로 패턴이면 패턴을 처리하고, 아니면 `Utility.GetRandomObjectName`.
fn prefix_object_name(
    object_prefix: &str,
    thread_prefix: &str,
    object_count: i32,
) -> Result<String, UtilError> {
    if !object_prefix.trim().is_empty() && PATH_PATTERN.is_match(object_prefix) {
        return generate_path_from_pattern(object_prefix, thread_prefix, object_count);
    }
    get_random_object_name(object_prefix, thread_prefix, object_count)
}

fn parse_i32(s: &str) -> Result<i32, UtilError> {
    s.parse()
        .map_err(|_| UtilError::InvalidNumber(s.to_string()))
}

/// 경로 패턴(`BUCKET{0..9}/DIR{0000..9999}`)에서 `object_count`로 경로를 만든다.
/// 각 세그먼트 값은 왼쪽부터 `object_count`를 범위 크기로 나눠가며 정한다.
fn generate_path_from_pattern(
    path_pattern: &str,
    thread_prefix: &str,
    object_count: i32,
) -> Result<String, UtilError> {
    let with_thread = |path: String| {
        if thread_prefix.trim().is_empty() {
            path
        } else {
            format!("{thread_prefix}/{path}")
        }
    };

    let matches: Vec<_> = PATTERN_SEGMENT.captures_iter(path_pattern).collect();
    if matches.is_empty() {
        // 매칭 실패 시 threadPrefix만 추가하여 반환
        return Ok(with_thread(path_pattern.to_string()));
    }

    let mut parts = Vec::with_capacity(matches.len());
    let mut remaining = object_count;
    for caps in &matches {
        let name = &caps[1];
        let start = parse_i32(&caps[2])?;
        let end_text = &caps[3];
        let end = parse_i32(end_text)?;
        let padding = match caps.get(4) {
            Some(m) => parse_i32(m.as_str())?,
            None => end.to_string().len() as i32,
        };
        let range = end.wrapping_sub(start).wrapping_add(1);
        let value = start.wrapping_add(checked_rem(remaining, range)?);
        remaining = checked_div(remaining, range)?;
        let padding = usize::try_from(padding).unwrap_or(0);
        parts.push(format!("{name}{value:0>padding$}"));
    }
    Ok(with_thread(parts.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(EnumBucketTypes::Prefix.name(), "Prefix");
        assert_eq!(EnumBucketTypes(99).name(), "Unknown");
        assert_eq!(EnumBucketTypes::from_name("Now"), EnumBucketTypes::Now);
        assert_eq!(EnumBucketTypes::from_name("x"), EnumBucketTypes::Empty);
        assert_eq!(EnumBucketTypes::from_int(3), EnumBucketTypes::Time);
        assert_eq!(EnumBucketTypes::from_int(6), EnumBucketTypes::Empty);
        assert_eq!(serde_json::to_string(&EnumBucketTypes(99)).unwrap(), "99");
    }

    #[test]
    fn next_names() {
        let n = |t: EnumBucketTypes, c| t.next_object_name("TH", "F", c, 1000).unwrap();
        assert_eq!(n(EnumBucketTypes::One, 7), "TH-F_00007");
        assert_eq!(n(EnumBucketTypes::Thread, 2500), "2/F_02500");
        assert_eq!(n(EnumBucketTypes::Time, 31), "TH/2025/02/01/F_00031");
        assert_eq!(n(EnumBucketTypes::None, 2500), "TH/00002/F_02500");
        assert_eq!(n(EnumBucketTypes(99), 5), "TH/00000/F_00005");
        assert_eq!(n(EnumBucketTypes::Prefix, 5), "TH/F00005");
    }

    #[test]
    fn path_pattern() {
        let g = |p: &str, c| generate_path_from_pattern(p, "TH", c).unwrap();
        assert_eq!(g("B{0..9}/D{0000..9999}", 0), "TH/B0/D0000");
        assert_eq!(g("B{0..9}/D{0000..9999}", 12345), "TH/B5/D1234");
        assert_eq!(g("D{1..3:4}", 4), "TH/D0002");
        assert_eq!(
            generate_path_from_pattern("{0..3}", "", 1).unwrap(),
            "{0..3}"
        );
    }
}
