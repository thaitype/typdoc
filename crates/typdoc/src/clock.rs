//! The clock the shipped binary runs on.

use chrono::{DateTime, FixedOffset, Local};
use typdoc_core::{Clock, at_one_second};

/// The machine's clock, in the machine's own offset.
///
/// The offset is the machine's rather than UTC, so that a stamped time says where it was
/// written, which is what carrying an offset is for. It changes no comparison, since a
/// `datetime` compares as an instant.
pub struct MachineClock;

impl Clock for MachineClock {
    fn now(&self) -> DateTime<FixedOffset> {
        at_one_second(Local::now().fixed_offset())
    }
}

/// Not part of the documented command line: it lets a golden fixture pin what an
/// `auto: create` or `auto: update` field holds, which the machine's moving clock cannot.
pub const FIXED_CLOCK_VAR: &str = "TYPDOC_FIXED_CLOCK";

/// A clock that always answers the one instant it was built with.
pub struct FixedEnvClock(DateTime<FixedOffset>);

impl Clock for FixedEnvClock {
    fn now(&self) -> DateTime<FixedOffset> {
        self.0
    }
}

/// The machine's clock, unless `FIXED_CLOCK_VAR` names an instant with an offset.
pub enum ShippedClock {
    Machine(MachineClock),
    Fixed(FixedEnvClock),
}

impl ShippedClock {
    /// Panics on a value that is not an instant with an offset, rather than falling back to the
    /// machine's clock and hiding the mistake in whatever a golden then records.
    pub fn from_var(value: Option<String>) -> ShippedClock {
        match value {
            None => ShippedClock::Machine(MachineClock),
            Some(text) => {
                let instant: DateTime<FixedOffset> = text.parse().unwrap_or_else(|e| {
                    panic!("{FIXED_CLOCK_VAR} is `{text}`, not an instant with an offset: {e}")
                });
                ShippedClock::Fixed(FixedEnvClock(at_one_second(instant)))
            }
        }
    }
}

impl Clock for ShippedClock {
    fn now(&self) -> DateTime<FixedOffset> {
        match self {
            ShippedClock::Machine(clock) => clock.now(),
            ShippedClock::Fixed(clock) => clock.now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_clock_reports_at_one_second() {
        assert_eq!(MachineClock.now().timestamp_subsec_nanos(), 0);
    }

    #[test]
    fn the_shipped_clock_moves() {
        let first = MachineClock.now();

        assert!(
            MachineClock.now() >= first,
            "the machine's clock went backwards between two readings"
        );
    }

    #[test]
    fn with_no_variable_set_the_shipped_clock_reads_the_machine() {
        let clock = ShippedClock::from_var(None);

        assert!(matches!(clock, ShippedClock::Machine(_)));
    }

    #[test]
    fn with_the_variable_set_the_shipped_clock_reports_that_one_instant_every_time() {
        let clock = ShippedClock::from_var(Some("2001-02-03T04:05:06+07:00".to_owned()));

        let expected: DateTime<FixedOffset> = "2001-02-03T04:05:06+07:00".parse().unwrap();
        assert_eq!(clock.now(), expected);
        assert_eq!(
            clock.now(),
            expected,
            "the same instant every call, not the machine's own"
        );
    }

    #[test]
    fn the_fixed_instant_is_truncated_to_one_second_the_same_as_the_machine_clock() {
        let clock = ShippedClock::from_var(Some("2001-02-03T04:05:06.987654321+07:00".to_owned()));

        assert_eq!(clock.now().timestamp_subsec_nanos(), 0);
        assert_eq!(clock.now().to_rfc3339(), "2001-02-03T04:05:06+07:00");
    }

    #[test]
    #[should_panic(expected = "TYPDOC_FIXED_CLOCK is `not-an-instant`")]
    fn a_variable_set_to_text_that_is_not_an_instant_with_an_offset_panics_naming_the_variable() {
        ShippedClock::from_var(Some("not-an-instant".to_owned()));
    }
}
