//! `SIGINT`/`SIGTERM` handling on Unix, Ctrl+C on Windows (SPC-3, SPC-10).
//!
//! The true handler only writes a byte that an ordinary thread reads: the cleanup, an identity
//! check and an unlink, is not async-signal-safe. That thread then restores the signal's default
//! disposition and re-raises it, so the process ends by the signal.

use std::io;

/// What `install` puts in place, for the message when it cannot.
#[cfg(unix)]
pub const HANDLER: &str = "SIGINT/SIGTERM";
#[cfg(windows)]
pub const HANDLER: &str = "Ctrl+C";

#[cfg(unix)]
use signal_hook::consts::{SIGINT, SIGTERM};
#[cfg(unix)]
use signal_hook::iterator::Signals;
#[cfg(unix)]
use signal_hook::low_level;

/// Called at the start of `main`, before any lock file can exist. The instant inside
/// `namespace_lock::acquire`'s creating call stays open (SPC-10).
#[cfg(unix)]
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

/// Ctrl+C, Ctrl+Break and closing the console (SPC-3, SPC-10). Windows runs the handler on a
/// thread of its own, and it ends the process there, with `STATUS_CONTROL_C_EXIT`: a Windows
/// process has no "ended by a signal", and that is the code Ctrl+C ends a process with.
#[cfg(windows)]
pub fn install() -> io::Result<()> {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    // SAFETY: `on_console_event` has the signature the call asks for and lives for the whole
    // process.
    if unsafe { SetConsoleCtrlHandler(Some(on_console_event), 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn on_console_event(_event: u32) -> windows_sys::core::BOOL {
    use windows_sys::Win32::Foundation::STATUS_CONTROL_C_EXIT;
    use windows_sys::Win32::System::Threading::ExitProcess;

    let fs = typdoc_fs::SystemFs;
    typdoc_core::release_all_for_signal(&fs);
    // SAFETY: ending the process is what the event asks for, and the locks are released first.
    unsafe { ExitProcess(STATUS_CONTROL_C_EXIT as u32) }
}
