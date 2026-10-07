//! TESTCore `Util/TimeWatcher.cs` 이식: 테스트 경과 시간, 2초마다 진행 상황 출력 시점, 종료 시각.

use std::time::{Duration, Instant};

use rust_decimal::Decimal;

/// 원본 `NEXT_PROGRESS`(초).
const NEXT_PROGRESS: u64 = 2;

#[derive(Debug, Clone)]
pub struct TimeWatcher {
    started: Option<Instant>,
    interval: u64,
    end_time: i32,
    next_timeout: u64,
}

impl TimeWatcher {
    /// 원본 `TimeWatcher(int endTime = 0)`.
    pub fn new(end_time: i32) -> Self {
        Self {
            started: None,
            interval: NEXT_PROGRESS,
            end_time,
            next_timeout: 0,
        }
    }

    pub fn interval(&self) -> u64 {
        self.interval
    }

    pub fn end_time(&self) -> i32 {
        self.end_time
    }

    pub fn start(&mut self) {
        self.started = Some(Instant::now());
        self.next_timeout = self.interval;
    }

    fn elapsed(&self) -> Duration {
        self.started.map(|s| s.elapsed()).unwrap_or_default()
    }

    /// 원본 `Now`: 경과 초(`(decimal)Elapsed.TotalSeconds`).
    pub fn now(&self) -> Decimal {
        seconds(self.elapsed())
    }

    /// 원본 `IsNext`: 다음 출력 시각이 지났으면 다음 시각을 정하고 `true`.
    pub fn is_next(&mut self) -> bool {
        if self.elapsed().as_secs_f64() >= self.next_timeout as f64 {
            self.next_timeout += self.interval;
            return true;
        }
        false
    }

    /// 원본 `IsEnd`: 경과 시간이 `EndTime`(초)을 넘었으면 `true`.
    pub fn is_end(&self) -> bool {
        self.elapsed().as_secs_f64() > f64::from(self.end_time)
    }
}

/// `TimeSpan.TotalSeconds`(double)를 `decimal`로 바꾼 값과 같게 만든다.
/// .NET의 `(decimal)double`은 유효 숫자 15자리로 반올림한다.
pub fn seconds(duration: Duration) -> Decimal {
    // TimeSpan은 100ns 단위(tick)다.
    let ticks = duration.as_nanos() / 100;
    let total_seconds = ticks as f64 / 10_000_000.0;
    double_to_decimal(total_seconds)
}

/// .NET `(decimal)double`: 유효 숫자 15자리로 반올림해 변환한다.
pub fn double_to_decimal(value: f64) -> Decimal {
    if value == 0.0 || !value.is_finite() {
        return Decimal::ZERO;
    }
    let text = format!("{value:.14e}");
    let decimal = Decimal::from_scientific(&text).unwrap_or_default();
    decimal.normalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn double_conversion_uses_15_digits() {
        assert_eq!(
            double_to_decimal(0.1 + 0.2),
            Decimal::from_str("0.3").unwrap()
        );
        assert_eq!(
            double_to_decimal(12.345_678_901_234_567),
            Decimal::from_str("12.3456789012346").unwrap()
        );
        assert_eq!(double_to_decimal(2.0), Decimal::from(2));
    }

    #[test]
    fn is_next_every_interval() {
        let mut watcher = TimeWatcher::new(0);
        watcher.start();
        assert!(!watcher.is_next());
    }
}
