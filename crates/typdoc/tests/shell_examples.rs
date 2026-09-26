//! Covers SPC-13.
//!
//! Every example is listed here by hand, since no test reads design prose as data (SPC-11).
//!
//! This file spawns a shell rather than going through `common::Spawn`, which starts the `typdoc`
//! binary directly: what it tests is what a shell does with an example's text before `typdoc`
//! sees it.
//!
//! An example needs a value declared by hand only when a word in it holds a character outside
//! the safe set (letters, digits, `_ - . / : = , @ % +` and the space): with no such character,
//! no shell can change what it means, so the expected value is its words split on whitespace.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// The shells SPC-13 lists. One that the host cannot run turns the suite red, never a skip.
const SHELLS: &[&str] = &["sh", "bash"];

/// What the stand-in recorded: its arguments and every `TYPDOC_` variable it saw.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Recorded {
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

/// Read by hand rather than with a derive, so this file needs no dependency beyond `serde_json`.
fn parse_recorded(text: &str) -> Option<Recorded> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let args = value
        .get("args")?
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    let env = value
        .get("env")?
        .as_object()?
        .iter()
        .map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
        .collect::<Option<BTreeMap<_, _>>>()?;
    Some(Recorded { args, env })
}

fn args(items: &[&str]) -> Recorded {
    Recorded {
        args: items.iter().map(|s| (*s).to_owned()).collect(),
        env: BTreeMap::new(),
    }
}

fn args_env(items: &[&str], env: &[(&str, &str)]) -> Recorded {
    Recorded {
        args: items.iter().map(|s| (*s).to_owned()).collect(),
        env: env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    }
}

/// `command` is the example's text, with `typdoc` put in front of a bare `--` fragment.
struct DeclaredExample {
    /// Used in failure messages only, never read from a file.
    text: &'static str,
    command: &'static str,
    expected: Recorded,
}

fn literal(text: &'static str, expected: Recorded) -> DeclaredExample {
    DeclaredExample {
        text,
        command: text,
        expected,
    }
}

/// Each value is what the quoting rule means for that one example, worked out by hand, never by
/// running anything.
fn declared_examples() -> Vec<DeclaredExample> {
    vec![
        literal(
            r#"typdoc new WF "Cosmos or SQL?" --set kind=grilling --set blocked_by=WF-1"#,
            args(&[
                "new",
                "WF",
                "Cosmos or SQL?",
                "--set",
                "kind=grilling",
                "--set",
                "blocked_by=WF-1",
            ]),
        ),
        literal(
            "typdoc list --code WF --where kind=research,prototype,grilling,task --where status=open --where 'ref.all(blocked_by).status=resolved'",
            args(&[
                "list",
                "--code",
                "WF",
                "--where",
                "kind=research,prototype,grilling,task",
                "--where",
                "status=open",
                "--where",
                "ref.all(blocked_by).status=resolved",
            ]),
        ),
        literal(
            "typdoc list --code WF --where kind=research,prototype,grilling,task --where status=open --where 'ref.any(blocked_by).status!=resolved'",
            args(&[
                "list",
                "--code",
                "WF",
                "--where",
                "kind=research,prototype,grilling,task",
                "--where",
                "status=open",
                "--where",
                "ref.any(blocked_by).status!=resolved",
            ]),
        ),
        literal(
            "typdoc list --where status=open --where 'refby.any(blocked_by).status=open'",
            args(&[
                "list",
                "--where",
                "status=open",
                "--where",
                "refby.any(blocked_by).status=open",
            ]),
        ),
        literal(
            "typdoc list --collection precedents --where 'ref.any(sources).status=retired'",
            args(&[
                "list",
                "--collection",
                "precedents",
                "--where",
                "ref.any(sources).status=retired",
            ]),
        ),
        literal(
            "typdoc list --collection learnings --where 'refby.none(sources)' --where 'refby.none($body)'",
            args(&[
                "list",
                "--collection",
                "learnings",
                "--where",
                "refby.none(sources)",
                "--where",
                "refby.none($body)",
            ]),
        ),
        literal(
            "typdoc list --collection proposals --where 'refby.none(proposal)'",
            args(&[
                "list",
                "--collection",
                "proposals",
                "--where",
                "refby.none(proposal)",
            ]),
        ),
        literal(
            "typdoc list --collection wayfinder --where status=open --where 'ref.any(context).collection=precedents'",
            args(&[
                "list",
                "--collection",
                "wayfinder",
                "--where",
                "status=open",
                "--where",
                "ref.any(context).collection=precedents",
            ]),
        ),
        literal(
            "TYPDOC_DIR=.chief typdoc list --namespace '*' --where status=open",
            args_env(
                &["list", "--namespace", "*", "--where", "status=open"],
                &[("TYPDOC_DIR", ".chief")],
            ),
        ),
        DeclaredExample {
            text: r"--where 'title=Cosmos\, or SQL'",
            command: r"typdoc --where 'title=Cosmos\, or SQL'",
            expected: args(&["--where", r"title=Cosmos\, or SQL"]),
        },
        DeclaredExample {
            text: "--namespace '*'",
            command: "typdoc --namespace '*'",
            expected: args(&["--namespace", "*"]),
        },
        DeclaredExample {
            text: "--namespace 'chief::*'",
            command: "typdoc --namespace 'chief::*'",
            expected: args(&["--namespace", "chief::*"]),
        },
    ]
}

/// Examples with no character outside the safe set. A bare mention of one flag or command
/// (`--json`, `typdoc new`) is left out, since no shell can change it, and `typdoc validate` is
/// run by `every_listed_shell_runs`.
fn additional_safe_examples() -> Vec<&'static str> {
    vec![
        "typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc",
        "typdoc set WF-3 status=claimed owner=zeldia-7a2f --if status=open",
        "typdoc refs precedents/secret-handling.md --reverse",
        "typdoc mv WF-2 --renumber story-3",
        "typdoc set WF-5 blocked_by=WF-3,WF-4",
        "typdoc toc WF-3 --json",
        "typdoc list --where blocked_by=WF-3",
        "typdoc refs learnings/never-send-secrets-over-ship.md --reverse",
        "typdoc get story-2:WF-5",
    ]
}

fn has_unsafe_char(text: &str) -> bool {
    !text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-./:=,@%+ ".contains(c))
}

/// A directory holding files an unquoted `*` and an unquoted `chief::*` can each match, so
/// removing the quotes changes what reaches typdoc rather than leaving it unchanged: a glob
/// that matches nothing is left as the literal characters by both shells, which would make
/// the quotes-removed case indistinguishable from the quoted one.
fn star_matching_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a directory for the unquoted-star runs");
    std::fs::write(dir.path().join("a.md"), "").expect("a.md");
    std::fs::write(dir.path().join("b.md"), "").expect("b.md");
    std::fs::write(dir.path().join("chief::wf-1"), "").expect("chief::wf-1");
    dir
}

/// Put on `PATH` so that `typdoc` resolves to the stand-in; the real binary is never on `PATH`
/// here.
fn stand_in_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().expect("a directory for the stand-in");
        std::fs::copy(
            env!("CARGO_BIN_EXE_typdoc_stand_in"),
            dir.path().join("typdoc"),
        )
        .expect("copy the stand-in binary in under the name typdoc");
        dir
    })
    .path()
}

struct ShellRun {
    stdout: String,
    stderr: String,
    /// `None` when the shell never reached the stand-in, a syntax error for one, or when what
    /// it wrote is not the JSON it always writes.
    recorded: Option<Recorded>,
}

/// A listed shell this host cannot run panics in `spawn`, uncaught: the suite goes red rather
/// than skipping it.
fn run_in_shell(shell: &str, command_text: &str, cwd: &Path) -> ShellRun {
    let home = tempfile::tempdir().expect("a fresh HOME");
    let out_dir = tempfile::tempdir().expect("a folder for the stand-in's output");
    let out_file = out_dir.path().join("recorded.json");
    let path = format!("{}:/usr/bin:/bin", stand_in_dir().display());

    let output = spawn(shell, command_text, cwd, home.path(), &path, &out_file);
    let recorded = std::fs::read_to_string(&out_file)
        .ok()
        .and_then(|text| parse_recorded(&text));
    ShellRun {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        recorded,
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "the shell examples harness has to run a real shell, to test what the shell itself does with an example's quoting; the CLI spawn helper only ever starts the typdoc binary directly, so it does not serve this, and this is the one place in this file a process is started"
)]
fn spawn(
    shell: &str,
    command_text: &str,
    cwd: &Path,
    home: &Path,
    path: &str,
    out_file: &Path,
) -> std::process::Output {
    Command::new(shell)
        .arg("-c")
        .arg(command_text)
        .env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("STAND_IN_OUT_FILE", out_file)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "{shell} is one of the shells the design lists and this run could not start it: {e}. \
                 Install {shell} on this host, or remove its row from the design's shell list \
                 (Quoting in the shell) if it is not meant to be covered."
            )
        })
}

fn safe_command_and_expected(text: &str) -> (String, Recorded) {
    if let Some(rest) = text.strip_prefix("typdoc") {
        (
            text.to_owned(),
            args(&rest.split_whitespace().collect::<Vec<_>>()),
        )
    } else {
        (
            format!("typdoc {text}"),
            args(&text.split_whitespace().collect::<Vec<_>>()),
        )
    }
}

#[test]
fn every_declared_example_matches_its_declared_value_quoted_in_every_listed_shell() {
    let dir = star_matching_dir();

    for example in declared_examples() {
        for shell in SHELLS {
            let run = run_in_shell(shell, example.command, dir.path());
            assert_eq!(
                run.recorded,
                Some(example.expected.clone()),
                "{shell} on {:?} (ran {:?}): stderr={:?}",
                example.text,
                example.command,
                run.stderr
            );
        }
    }
}

/// Without quotes, `(` and `)` make most of these a syntax error rather than a value that parses
/// and differs. The examples built on `*` and on `\,` are the ones that still parse, the quieter
/// danger the quoting rule warns of.
#[test]
fn removing_the_quotes_from_a_declared_example_changes_what_the_shell_passes() {
    let dir = star_matching_dir();

    for example in declared_examples() {
        let unquoted: String = example
            .command
            .chars()
            .filter(|&c| c != '\'' && c != '"')
            .collect();
        for shell in SHELLS {
            let run = run_in_shell(shell, &unquoted, dir.path());
            assert_ne!(
                run.recorded,
                Some(example.expected.clone()),
                "{shell}: removing the quotes from {:?} (ran {:?}) still matched the declared \
                 value; stdout={:?} stderr={:?}",
                example.text,
                unquoted,
                run.stdout,
                run.stderr
            );
        }
    }
}

#[test]
fn every_hand_listed_safe_example_reaches_the_stand_in_as_its_own_words() {
    let dir = star_matching_dir();

    for text in additional_safe_examples() {
        assert!(
            !has_unsafe_char(text),
            "a hand-listed safe example has a character outside the safe set and belongs in \
             declared_examples instead: {text}"
        );
        let (command, expected) = safe_command_and_expected(text);
        for shell in SHELLS {
            let run = run_in_shell(shell, &command, dir.path());
            assert_eq!(
                run.recorded,
                Some(expected.clone()),
                "{shell} on {text:?} (ran {command:?}): stderr={:?}",
                run.stderr
            );
        }
    }
}

/// A missing quote turns the suite red only if every listed shell runs, so this checks that
/// none was skipped.
#[test]
fn every_listed_shell_runs() {
    let dir = star_matching_dir();
    for shell in SHELLS {
        let run = run_in_shell(shell, "typdoc validate", dir.path());
        assert_eq!(
            run.recorded,
            Some(args(&["validate"])),
            "{shell} did not run typdoc validate cleanly: stderr={:?}",
            run.stderr
        );
    }
}
