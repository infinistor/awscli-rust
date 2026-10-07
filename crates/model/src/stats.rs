//! TESTCore `Data/Average.cs`, `Data/OperationStats.cs`, `Data/TestStats.cs`, `Enum/SharpPoint.cs`,
//! `Client/ITestClient.cs` 이식.
//!
//! 원본 수치는 모두 .NET `decimal`이라 `rust_decimal::Decimal`(같은 96비트·28자리 10진수)로 계산한다.

use std::sync::atomic::{AtomicI64, Ordering};

use rust_decimal::Decimal;
use tokio_util::sync::CancellationToken;

/// 원본 `SharpPoint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharpPoint {
    None,
    Low,
    High,
}

impl SharpPoint {
    /// 원본 `ToIcon()`.
    pub fn icon(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Low => "▼",
            Self::High => "▲",
        }
    }
}

/// 원본 `Average`: 최근 30개 구간 값으로 초당 평균을 낸다. 구간은 2초(`PER_TIME`)다.
#[derive(Debug, Clone, Default)]
pub struct Average {
    numbers: Vec<Decimal>,
    old_number: Decimal,
}

impl Average {
    pub const MAX_COUNT: usize = 30;
    pub const PER_TIME: i64 = 2;

    /// 누적값을 받아 직전 값과의 차이를 넣는다. 직전 값이 0이면(처음 포함) 받은 값을 그대로 넣는다.
    pub fn add(&mut self, number: Decimal) {
        if self.old_number.is_zero() {
            self.numbers.push(number);
        } else {
            self.numbers.push(number - self.old_number);
        }
        self.old_number = number;
        self.trim();
    }

    /// 구간 값을 그대로 넣는다.
    pub fn increment(&mut self, number: Decimal) {
        self.numbers.push(number);
        self.trim();
    }

    fn trim(&mut self) {
        if self.numbers.len() > Self::MAX_COUNT {
            self.numbers.remove(0);
        }
    }

    /// 초당 평균: 합 / (개수 × 2).
    pub fn get(&self) -> Decimal {
        if self.numbers.is_empty() {
            return Decimal::ZERO;
        }
        let sum: Decimal = self.numbers.iter().sum();
        sum / Decimal::from(self.numbers.len() as i64 * Self::PER_TIME)
    }

    /// 마지막 구간 값.
    pub fn get_last(&self) -> Decimal {
        self.numbers.last().copied().unwrap_or(Decimal::ZERO)
    }

    /// 마지막 구간의 초당 값이 평균과 10% 이상 차이 나면 급변으로 본다.
    pub fn is_sharp_change(&self) -> SharpPoint {
        if self.numbers.len() < 2 {
            return SharpPoint::None;
        }
        let current = self.get_last() / Decimal::from(Self::PER_TIME);
        let average = self.get();
        if (current - average).abs() < average * Decimal::new(1, 1) {
            return SharpPoint::None;
        }
        if current > average {
            SharpPoint::High
        } else {
            SharpPoint::Low
        }
    }
}

/// 원본 `OperationStats`: 성공·실패 횟수. 부하 스레드가 올리고 통계 스레드가 읽는다.
#[derive(Debug, Default)]
pub struct OperationStats {
    success: AtomicI64,
    error: AtomicI64,
}

impl OperationStats {
    pub fn success(&self) -> i64 {
        self.success.load(Ordering::Relaxed)
    }

    pub fn error(&self) -> i64 {
        self.error.load(Ordering::Relaxed)
    }

    pub fn set_success(&self, value: i64) {
        self.success.store(value, Ordering::Relaxed);
    }

    pub fn set_error(&self, value: i64) {
        self.error.store(value, Ordering::Relaxed);
    }

    /// 원본 `Success++`.
    pub fn add_success(&self, count: i64) {
        self.success.fetch_add(count, Ordering::Relaxed);
    }

    /// 원본 `Error++`.
    pub fn add_error(&self, count: i64) {
        self.error.fetch_add(count, Ordering::Relaxed);
    }

    pub fn total(&self) -> i64 {
        self.success() + self.error()
    }

    /// 성공률(%): 성공 / 전체 × 100. 전체가 0이면 0.
    pub fn success_ratio(&self) -> Decimal {
        let total = self.total();
        if total == 0 {
            return Decimal::ZERO;
        }
        Decimal::from(self.success()) / Decimal::from(total) * Decimal::from(100)
    }

    pub fn init(&self) {
        self.set_success(0);
        self.set_error(0);
    }
}

/// 원본 `WriteStats`: 쓰기 통계 + 멀티파트 파트 수.
#[derive(Debug, Default)]
pub struct WriteStats {
    pub ops: OperationStats,
    part: AtomicI64,
}

impl WriteStats {
    pub fn part(&self) -> i64 {
        self.part.load(Ordering::Relaxed)
    }

    pub fn set_part(&self, value: i64) {
        self.part.store(value, Ordering::Relaxed);
    }

    pub fn add_part(&self, count: i64) {
        self.part.fetch_add(count, Ordering::Relaxed);
    }

    pub fn init(&self) {
        self.ops.init();
        self.set_part(0);
    }
}

impl std::ops::Deref for WriteStats {
    type Target = OperationStats;

    fn deref(&self) -> &OperationStats {
        &self.ops
    }
}

/// 원본 `TimeStats`: 여러 클라이언트의 합계와 초당 평균.
#[derive(Debug, Clone, Default)]
pub struct TimeStats {
    pub success: i64,
    pub error: i64,
    one_min_average: Average,
}

impl TimeStats {
    pub fn total(&self) -> i64 {
        self.success + self.error
    }

    pub fn success_ratio(&self) -> Decimal {
        let total = self.total();
        if total == 0 {
            return Decimal::ZERO;
        }
        Decimal::from(self.success) / Decimal::from(total) * Decimal::from(100)
    }

    pub fn init(&mut self) {
        self.success = 0;
        self.error = 0;
    }

    pub fn add(&mut self, stats: &OperationStats) {
        self.success += stats.success();
        self.error += stats.error();
    }

    pub fn get_last_second_count(&self) -> Decimal {
        self.one_min_average.get_last()
    }

    pub fn is_sharp_change(&self) -> SharpPoint {
        self.one_min_average.is_sharp_change()
    }

    pub fn get_average_per_second(&self) -> Decimal {
        self.one_min_average.get()
    }

    pub fn update_average(&mut self) {
        self.one_min_average.add(Decimal::from(self.success));
    }

    pub fn get_bandwidth(&self, file_size: i64) -> Decimal {
        self.get_average_per_second() * Decimal::from(file_size)
    }
}

/// 원본 `WriteTimeStats`: 쓰기 합계 + 파트 합계와 파트 초당 평균.
#[derive(Debug, Clone, Default)]
pub struct WriteTimeStats {
    pub base: TimeStats,
    pub part: Decimal,
    part_average: Average,
}

impl WriteTimeStats {
    pub fn init(&mut self) {
        self.base.init();
        self.part = Decimal::ZERO;
    }

    pub fn add(&mut self, stats: &WriteStats) {
        self.base.add(&stats.ops);
        self.part += Decimal::from(stats.part());
    }

    pub fn update_part_average(&mut self) {
        self.part_average.add(self.part);
    }

    pub fn get_part_average_per_second(&self) -> Decimal {
        self.part_average.get()
    }

    pub fn get_part_last_second_count(&self) -> Decimal {
        self.part_average.get_last()
    }

    pub fn is_part_sharp_change(&self) -> SharpPoint {
        self.part_average.is_sharp_change()
    }

    pub fn get_part_bandwidth(&self, part_size: i64) -> Decimal {
        self.get_part_average_per_second() * Decimal::from(part_size)
    }
}

impl std::ops::Deref for WriteTimeStats {
    type Target = TimeStats;

    fn deref(&self) -> &TimeStats {
        &self.base
    }
}

impl std::ops::DerefMut for WriteTimeStats {
    fn deref_mut(&mut self) -> &mut TimeStats {
        &mut self.base
    }
}

/// 원본 `TestStats`: 클라이언트 하나의 연산별 통계.
#[derive(Debug, Default)]
pub struct TestStats {
    pub write: WriteStats,
    pub read: OperationStats,
    pub head: OperationStats,
    pub delete: OperationStats,
    pub list: OperationStats,
    pub loop_end_count: AtomicI64,
    pub found_count: AtomicI64,
}

/// 원본 `ITestClient`.
pub trait TestClient: Send + Sync {
    fn stats(&self) -> &TestStats;
    fn quit(&self) -> bool;
    fn set_quit(&self, quit: bool);
}

/// `Quit` 플래그 구현을 돕는 값. 원본 `bool Quit { get; set; }`.
///
/// 종료 처리를 `CancellationToken` 하나로 통일한다. 시나리오가 [`QuitFlag::child_of`]로 상위 토큰(테스트·프로세스)에
/// 묶으면, 테스트의 `TestStop`(이 플래그 하나)과 Ctrl+C(상위 토큰)가 같은 경로로 클라이언트를 멈춘다.
#[derive(Debug, Clone, Default)]
pub struct QuitFlag(CancellationToken);

impl QuitFlag {
    /// 상위 토큰이 취소되면 함께 취소되는 플래그.
    pub fn child_of(parent: &CancellationToken) -> Self {
        Self(parent.child_token())
    }

    pub fn get(&self) -> bool {
        self.0.is_cancelled()
    }

    /// 원본 `Quit = value`. 취소는 되돌릴 수 없어 `false`는 무시한다(원본도 `Quit`을 `false`로 되돌리는 곳이 없다).
    pub fn set(&self, value: bool) {
        if value {
            self.0.cancel();
        }
    }

    /// 플래그의 토큰(대기 중인 작업을 취소에 묶을 때).
    pub fn token(&self) -> &CancellationToken {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_flag_follows_parent_token() {
        let parent = CancellationToken::new();
        let a = QuitFlag::child_of(&parent);
        let b = QuitFlag::child_of(&parent);
        a.set(false);
        assert!(!a.get());
        a.set(true);
        assert!(a.get() && !b.get() && !parent.is_cancelled());
        parent.cancel();
        assert!(b.get());
    }

    #[test]
    fn average_uses_differences() {
        let mut average = Average::default();
        average.add(Decimal::from(10));
        average.add(Decimal::from(30));
        // 구간 값 10, 20 → (10 + 20) / (2 × 2) = 7.5
        assert_eq!(average.get(), Decimal::new(75, 1));
        assert_eq!(average.get_last(), Decimal::from(20));
        assert_eq!(average.is_sharp_change(), SharpPoint::High);
    }

    #[test]
    fn average_keeps_last_30() {
        let mut average = Average::default();
        for i in 1..=40 {
            average.increment(Decimal::from(i));
        }
        assert_eq!(average.get_last(), Decimal::from(40));
        // 11..=40의 합 765 / 60
        assert_eq!(average.get(), Decimal::from(765) / Decimal::from(60));
    }

    #[test]
    fn zero_old_number_resets_difference() {
        // 원본은 직전 누적값이 0이면 차이 대신 값을 그대로 넣는다.
        let mut average = Average::default();
        average.add(Decimal::ZERO);
        average.add(Decimal::from(5));
        assert_eq!(average.get_last(), Decimal::from(5));
    }
}
