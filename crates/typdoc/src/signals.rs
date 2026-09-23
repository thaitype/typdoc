//! Installs the `SIGINT`/`SIGTERM` handling decisions 5 and 6 ask for.
//!
//! The handler itself does nothing: `signal_hook::iterator::Signals` writes a byte from the
//! true signal handler and an ordinary thread reads it, which is what lets the cleanup —
//! comparing file identity and unlinking, neither async-signal-safe — run off the handler
//! rather than in it. That thread then ends the process through the signal's own default
//! disposition, restored and re-raised, so the caller sees a process killed by the signal and
//! not an exit code of typdoc's own.

use std::io;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level;

/// Registers the handler and spawns the cleanup thread. Called once, at the very start of
/// `main`, before any command runs and so before any lock file this process might create
/// exists: decision 6's own "registered before any lock file is created, at the start of the
/// run". What that ordering cannot close is the instant inside `namespace_lock::acquire`'s own
/// creating call, between the kernel making the file and this process recording that it holds
/// it — decision 6 leaves that window open in writing, and this function does not attempt to
/// close it either.
pub fn install() -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    std::thread::spawn(move || {
        // One iteration per signal delivered to this thread rather than to the real handler.
        // While this body is running, further deliveries of the same signal are queued by
        // `signal_hook` rather than reaching a second, concurrent iteration (decision 6: "a
        // second signal ... does not cut it short"); in practice there never is a second
        // iteration, because the process ends by the signal at the bottom of this one.
        for signal in signals.forever() {
            // `SystemFs` carries no state of its own, so a fresh one here reaches the real
            // file system exactly as `main`'s own would, with nothing to share across the
            // thread boundary: `NamespaceLock` itself cannot cross it (it holds `&dyn Fs` and
            // a boxed handle with no `Send` bound), which is why the identity check this thread
            // runs reads typdoc-core's own registry of what is still held, not the lock values
            // themselves.
            let fs = typdoc_fs::SystemFs;
            typdoc_core::release_all_for_signal(&fs);
            // Restores the default disposition for `signal` and re-raises it: the process ends
            // by the signal, not by an exit code typdoc chose (decision 6).
            let _ = low_level::emulate_default_handler(signal);
        }
    });
    Ok(())
}
