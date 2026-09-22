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
}
