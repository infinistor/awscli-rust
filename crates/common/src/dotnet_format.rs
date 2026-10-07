//! .NET 복합 서식(`{value,9:F3}` 등)을 흉내 낸다. 통계·진행 상황 출력이 원본과 같아야 하므로
//! `decimal` 반올림(0에서 먼 쪽)과 정렬 폭 규칙을 그대로 따른다.

use rust_decimal::{Decimal, RoundingStrategy};

/// `{value,width}`: 양수면 오른쪽 정렬, 음수면 왼쪽 정렬. 내용이 더 길면 자르지 않는다.
pub fn align(text: impl AsRef<str>, width: i32) -> String {
    let text = text.as_ref();
    let len = text.chars().count();
    let target = width.unsigned_abs() as usize;
    if len >= target {
        return text.to_string();
    }
    let padding = " ".repeat(target - len);
    if width >= 0 {
        format!("{padding}{text}")
    } else {
        format!("{text}{padding}")
    }
}

/// `decimal.ToString("F{decimals}")`: 소수 `decimals`자리, 0에서 먼 쪽으로 반올림.
pub fn fixed(value: Decimal, decimals: u32) -> String {
    let rounded = value.round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero);
    let mut text = format!("{:.*}", decimals as usize, rounded);
    // .NET은 반올림 결과가 0이면 부호를 붙이지 않는다("-0.000" 아님).
    if rounded.is_zero() && text.starts_with('-') {
        text.remove(0);
    }
    text
}

/// `{value,width:F{decimals}}`.
pub fn fixed_aligned(value: Decimal, width: i32, decimals: u32) -> String {
    align(fixed(value, decimals), width)
}

/// `decimal.ToString()`: 값이 가진 소수 자릿수(scale)를 그대로 쓴다(`2.50`은 `2.50`).
pub fn decimal_text(value: Decimal) -> String {
    let text = value.to_string();
    if value.is_zero() && text.starts_with('-') {
        text[1..].to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    #[test]
    fn fixed_rounds_away_from_zero() {
        assert_eq!(fixed(d("2.0005"), 3), "2.001");
        assert_eq!(fixed(d("-2.0005"), 3), "-2.001");
        assert_eq!(fixed(d("66.66666666666666666666666667"), 3), "66.667");
        assert_eq!(fixed(d("0.05"), 1), "0.1");
        assert_eq!(fixed(d("-0.0001"), 3), "0.000");
        assert_eq!(fixed(Decimal::from(12), 3), "12.000");
    }

    #[test]
    fn alignment() {
        assert_eq!(align("12", 5), "   12");
        assert_eq!(align("12", -5), "12   ");
        assert_eq!(align("123456", 3), "123456");
        assert_eq!(fixed_aligned(d("1.5"), 9, 3), "    1.500");
    }

    #[test]
    fn decimal_scale_is_kept() {
        assert_eq!(decimal_text(d("2.50")), "2.50");
        assert_eq!(decimal_text(Decimal::from(5)), "5");
    }
}
