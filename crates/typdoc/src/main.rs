use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use typdoc::cli;
use typdoc::clock::{FIXED_CLOCK_VAR, ShippedClock};
use typdoc::process_env::ProcessEnv;
use typdoc_core::{Deps, Env};
use typdoc_fs::SystemFs;

mod signals;

fn main() -> ExitCode {
    // Registered before any command runs, so before any lock file exists (SPC-10). Not
    // verified: `Signals::new` cannot be made to fail here, so the branch below is not shown to
    // run. Exit 6 is the closest code to a startup failure: a failure of the environment (SPC-3).
    if let Err(source) = signals::install() {
        eprintln!("typdoc: could not install the SIGINT/SIGTERM handler: {source}");
        return ExitCode::from(6);
    }
    let args: Vec<OsString> = std::env::args_os().collect();
    let env = ProcessEnv;
    let fixed_clock = env.var(FIXED_CLOCK_VAR).map(|value| {
        value
            .into_string()
            .unwrap_or_else(|raw| panic!("{FIXED_CLOCK_VAR} is not UTF-8: {raw:?}"))
    });
    let clock = ShippedClock::from_var(fixed_clock);
    let deps = Deps {
        env: &env,
        fs: &SystemFs,
        clock: &clock,
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
