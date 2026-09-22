use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use typdoc::cli;
use typdoc::clock::MachineClock;
use typdoc_core::{Deps, Env};
use typdoc_fs::SystemFs;

struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        std::env::current_dir()
    }

    // Linux is the only supported platform (design, Concurrency), so the kernel's own record
    // of the machine's name is read directly rather than through a crate: a plain file, not a
    // call the write ban has any reason to cover. Read infallibly (`pid_alive`'s own shape, for
    // the same reason): the hostname only ever decorates a message, so a read that fails or
    // finds an empty file falls back to a placeholder rather than refusing to take the lock.
    fn hostname(&self) -> String {
        std::fs::read_to_string("/proc/sys/kernel/hostname")
            .ok()
            .map(|raw| raw.trim().to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "unknown-host".to_owned())
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let deps = Deps {
        env: &ProcessEnv,
        fs: &SystemFs,
        clock: &MachineClock,
    };
    let outcome = cli::run(&args, &deps);
    write_all(&mut io::stdout(), &outcome.stdout);
    write_all(&mut io::stderr(), &outcome.stderr);
    ExitCode::from(outcome.code)
}

fn write_all(to: &mut impl Write, text: &str) {
    if !text.is_empty() {
        // A closed pipe is the reader's choice, not a failure of the command.
        let _ = to.write_all(text.as_bytes());
    }
}
