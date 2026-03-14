#![forbid(unsafe_code)]

//! Time abstractions for production code and deterministic tests.

use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Duration, Utc};

/// A source of the current time.
pub trait Clock: Send + Sync {
    /// Returns the current timestamp in UTC.
    fn now(&self) -> DateTime<Utc>;
}

/// A clock backed by the system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealClock;

impl RealClock {
    /// Creates a new system-backed clock.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Clock for RealClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A manually controlled clock for deterministic tests.
///
/// Clones of a `MockClock` share the same underlying state.
#[derive(Debug, Clone)]
pub struct MockClock {
    current: Arc<Mutex<DateTime<Utc>>>,
}

impl MockClock {
    /// Creates a new mock clock fixed at the provided timestamp.
    #[must_use]
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            current: Arc::new(Mutex::new(now)),
        }
    }

    /// Replaces the current mock time with the provided timestamp.
    pub fn set(&self, now: DateTime<Utc>) {
        *self.lock_current() = now;
    }

    /// Advances the current mock time by the provided duration.
    ///
    /// Passing a negative duration moves the clock backwards.
    pub fn advance(&self, duration: Duration) {
        let mut current = self.lock_current();
        *current = current.to_owned() + duration;
    }

    fn lock_current(&self) -> MutexGuard<'_, DateTime<Utc>> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        self.lock_current().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn real_clock_returns_a_timestamp_between_two_observations() {
        let before = Utc::now();
        let now = RealClock::new().now();
        let after = Utc::now();

        assert!(now >= before);
        assert!(now <= after);
    }

    #[test]
    fn mock_clock_returns_its_seed_time() {
        let seeded = Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap();
        let clock = MockClock::new(seeded);

        assert_eq!(clock.now(), seeded);
    }

    #[test]
    fn mock_clock_can_be_set_to_a_new_time() {
        let original = Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap();
        let updated = Utc.with_ymd_and_hms(2026, 3, 14, 12, 30, 0).unwrap();
        let clock = MockClock::new(original);

        clock.set(updated);

        assert_eq!(clock.now(), updated);
    }

    #[test]
    fn mock_clock_can_advance_forward() {
        let original = Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap();
        let clock = MockClock::new(original);

        clock.advance(Duration::minutes(45));

        assert_eq!(
            clock.now(),
            Utc.with_ymd_and_hms(2026, 3, 14, 12, 45, 0).unwrap()
        );
    }

    #[test]
    fn mock_clock_can_advance_backward_with_negative_durations() {
        let original = Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap();
        let clock = MockClock::new(original);

        clock.advance(Duration::minutes(-15));

        assert_eq!(
            clock.now(),
            Utc.with_ymd_and_hms(2026, 3, 14, 11, 45, 0).unwrap()
        );
    }

    #[test]
    fn cloned_mock_clocks_share_the_same_state() {
        let original = Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap();
        let clock = MockClock::new(original);
        let cloned = clock.clone();

        cloned.advance(Duration::seconds(30));

        assert_eq!(
            clock.now(),
            Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 30).unwrap()
        );
    }
}
