//! Clock abstraction for production and test time sources.

use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};

/// Time provider abstraction for deterministic testing.
pub trait Clock: Send + Sync {
    /// Returns the current timestamp in UTC.
    fn now(&self) -> DateTime<Utc>;
}

/// Real wall-clock implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Mutable test clock implementation.
#[derive(Debug, Clone)]
pub struct MockClock {
    current: Arc<Mutex<DateTime<Utc>>>,
}

impl MockClock {
    /// Creates a mock clock initialized to `initial`.
    #[must_use]
    pub fn new(initial: DateTime<Utc>) -> Self {
        Self {
            current: Arc::new(Mutex::new(initial)),
        }
    }

    /// Sets the current mock time.
    pub fn set(&self, next: DateTime<Utc>) {
        *lock_recover(&self.current) = next;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *lock_recover(&self.current)
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn mock_clock_returns_set_value() {
        let initial = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("valid timestamp");
        let next = Utc
            .with_ymd_and_hms(2026, 1, 2, 0, 0, 0)
            .single()
            .expect("valid timestamp");

        let clock = MockClock::new(initial);
        assert_eq!(clock.now(), initial);

        clock.set(next);
        assert_eq!(clock.now(), next);
    }
}
