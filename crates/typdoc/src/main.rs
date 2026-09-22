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

    fn hostname(&self) -> String {
        hostname::get()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown-host".to_owned())
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
