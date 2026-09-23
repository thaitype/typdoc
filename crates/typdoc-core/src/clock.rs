//! The time a write stamps into a document, reached only through `deps`.
//!
//! The clock hands back an instant together with the offset to write it in, and the formatting
//! stays this program's: a clock that returned the finished text would leave the formatting
//! with no test, since the test would be reading back a value it had handed in. Keeping the
//! offset on the clock also keeps it out of the machine, so what a test prints does not change
//! with where the machine is.

use chrono::{DateTime, FixedOffset, SubsecRound};

/// What time it is, and the offset to write it in.
pub trait Clock {
    /// The current instant at one second, which is the resolution a `datetime` carries:
    /// `2026-09-19T14:30:00+07:00`. A finer instant would be written away and would not read
    /// back as itself.
    fn now(&self) -> DateTime<FixedOffset>;
}

/// The instant with everything below a second dropped, for a clock to hand back.
pub fn at_one_second(instant: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    instant.trunc_subsecs(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_below_a_second_is_dropped_and_the_offset_is_kept() {
        let instant: DateTime<FixedOffset> = "2026-09-19T14:30:00.987654321+07:00"
            .parse()
            .expect("the text is an ISO 8601 instant with an offset");

        let whole = at_one_second(instant);

        assert_eq!(whole.timestamp_subsec_nanos(), 0);
        assert_eq!(whole.to_rfc3339(), "2026-09-19T14:30:00+07:00");
        assert_eq!(whole.offset(), instant.offset());
    }

    #[test]
    fn an_instant_already_at_one_second_is_left_where_it_is() {
        let instant: DateTime<FixedOffset> = "2026-09-19T14:30:00+07:00"
            .parse()
            .expect("the text is an ISO 8601 instant with an offset");

        assert_eq!(at_one_second(instant), instant);
    }
}
