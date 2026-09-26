//! `SIGINT`/`SIGTERM` handling (SPC-3, SPC-10).
//!
//! The true handler only writes a byte that an ordinary thread reads: the cleanup, an identity
//! check and an unlink, is not async-signal-safe. That thread then restores the signal's default
//! disposition and re-raises it, so the process ends by the signal.

use std::io;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level;

/// Called at the start of `main`, before any lock file can exist. The instant inside
/// `namespace_lock::acquire`'s creating call stays open (SPC-10).
pub fn install() -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    std::thread::spawn(move || {
        // A second signal waits behind this iteration rather than cutting the cleanup short,
        // and the process ends by the signal at its bottom, so a second iteration never runs.
        for signal in signals.forever() {
            // `NamespaceLock` is not `Send`, so this thread reads typdoc-core's registry of the
            // locks still held, through a fresh `SystemFs`, which has no state of its own.
            let fs = typdoc_fs::SystemFs;
            typdoc_core::release_all_for_signal(&fs);
            // The process ends by the signal, not by an exit code of typdoc's own (SPC-3).
            let _ = low_level::emulate_default_handler(signal);
        }
    });
    Ok(())
}
