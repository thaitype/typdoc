mod cli;

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use typdoc_core::{Deps, Env};

struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        std::env::current_dir()
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let deps = Deps { env: &ProcessEnv };
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
