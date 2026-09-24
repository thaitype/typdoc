//! The harness for typdoc's shell examples, run for real against a stand-in binary through
//! `sh` and `bash`.
//!
//! Before ticket 12/M-13, this file extracted every example from the design document
//! automatically, through a markdown-walking helper this crate no longer has, so the harness
//! and the document shared one list. That was itself a violation of the same rule that removed
//! the old design-parsing test helper (M-1: no program reads markdown to extract spec as
//! machine data) -- a bigger one, since it covered every worked shell example in the whole
//! document rather than five specific tables. M-13 (ticket 24) resolved it as option 1:
//! hand-list every example directly here, with no fifth catalog document. The list below
//! (`declared_examples`, plus `additional_safe_examples`) is now the *only* list this harness
//! runs -- nothing here reads the design document, or any other markdown, to find an example.
//! See this ticket's report for which further "safe" examples (no shell-unsafe character),
//! previously found only through extraction, were hand-listed here and which were dropped, and
//! why.
//!
//! Process spawning here is the one exception to the CLI tests' spawn helper (`common::Spawn`
//! spawns the built `typdoc` binary directly, with a fixed argument list it controls). What
//! this file tests is what a shell itself does with an example's text before `typdoc` ever
//! sees it, so the helper does not serve it: a shell has to run the example, not the binary.
//! `spawn`, below, is the one place in this file `std::process::Command::new` appears, with
//! one narrow `#[allow(clippy::disallowed_methods, ...)]` on it, the same shape as the
//! existing helper's.
//!
//! An example needs a value declared by hand only when a word in it holds a character
//! outside the safe set (letters, digits, `_ - . / : = , @ % +` and the space): with no such
//! character, no shell can change what it means, so the expected value is just its words
//! split on whitespace, computed here rather than written by hand.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// v1 covers these two, and only these two: this host has `sh` (dash) and `bash`, and has no
/// zsh, which is why the design's own shell list stops at these. A shell named here that the
/// host cannot run must turn the suite red, never skip quietly (see `spawn` below).
const SHELLS: &[&str] = &["sh", "bash"];

/// What the stand-in recorded: its own arguments, and every `TYPDOC_`-prefixed environment
/// variable it saw. Also how an expected value is written by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Recorded {
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

/// Reads the stand-in's `{ "args": [...], "env": {...} }`, without a derive macro so this
/// file needs no dependency beyond `serde_json`, already a dependency of the binary itself.
/// Anything that is not exactly that shape is `None`: certainly not a match for a value
/// declared by hand.
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

/// One example declared in full: what is run (`command`, exactly the example's text, with
/// `typdoc` supplied where the text is a bare `--` fragment) and the value declared by hand
/// from the design's own words about quoting.
struct DeclaredExample {
    /// The example's text, as it appears in `docs/design/design.md` (or its `spec`
    /// replacement) -- used in failure messages, not read back out of any file.
    text: &'static str,
    /// What is actually run through the shell.
    command: &'static str,
    expected: Recorded,
}

/// An example that already begins with `typdoc` (or a `TYPDOC_` assignment naming it): the
/// text is what is run, so there is only one string to write.
fn literal(text: &'static str, expected: Recorded) -> DeclaredExample {
    DeclaredExample {
        text,
        command: text,
        expected,
    }
}

/// Every example that needs a value declared by hand: a literal, standalone invocation with a
/// character outside the safe set. Each value is what "give it to typdoc exactly as written,
/// in single quotes" (Quoting in the shell) means for that one example, worked out from the
/// design's own words, never from running anything.
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

/// The further "safe" examples (no shell-unsafe character) worth keeping now that nothing
/// extracts examples from markdown any more (ticket 12/M-13, option 1). Before this ticket,
/// each was found automatically by walking `docs/design/design.md` and its expected value was
/// computed by splitting it on whitespace, since no unsafe character means no shell can change
/// what it means -- `safe_command_and_expected` below still does exactly that; only the list
/// of texts is now written by hand instead of extracted.
///
/// These nine are every worked, standalone invocation with real arguments that used to reach
/// the harness only this way (drawn from the design's "Worked examples" and "Common tasks"
/// tables). What did *not* make this list, and why, is in this ticket's report: bare mentions
/// of a single flag or command name with no arguments (`--json`, `--where`, `typdoc new` alone,
/// and so on), which are prose references rather than invocations and exercise no shell
/// behavior a one-word, all-alphanumeric string could ever be at risk from; and `typdoc
/// validate` on its own, whose coverage `every_listed_shell_runs` below already gives it
/// verbatim.
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

/// True when `text` holds a character outside the safe set (letters, digits,
/// `_ - . / : = , @ % +` and the space): the one condition under which a shell can change
/// what an example means, and so the one condition under which it needs a value declared by
/// hand rather than one computed by splitting its words.
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

/// A folder holding the stand-in, copied in under the name `typdoc`, once per test binary
/// run: this is what makes `typdoc` on the harness's `PATH` resolve to the stand-in and
/// never to a real build of the CLI, which is not on `PATH` at all here.
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

/// What one run through a shell left behind.
struct ShellRun {
    stdout: String,
    stderr: String,
    /// What the stand-in recorded, if it ran and wrote its output; `None` covers both "the
    /// shell never reached it" (a syntax error, for instance) and "it wrote something that
    /// is not the JSON it always writes", either of which is certainly not a match for a
    /// declared value.
    recorded: Option<Recorded>,
}

/// Runs `command_text` through `shell`, found on the harness's own constant `PATH` (the
/// stand-in's directory, then the real shells' directory), with an empty environment
/// otherwise, a fresh `HOME`, and `cwd` as the working directory. A shell the design lists
/// and this host cannot run ends the process with an error here, which is not caught: the
/// suite goes red for it rather than skipping it (Shells: a listed shell that is missing).
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

/// The words of `text`, split on ASCII whitespace: what a shell gives typdoc for an example
/// with no character outside the safe set, since nothing about quoting can change it. The
/// command to run is `text` itself when it already starts with `typdoc`, and `typdoc ` put
/// in front of it otherwise (a bare `--` fragment).
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

/// For most of `declared_examples`, unquoted `(` and `)` (from a query condition such as
/// `ref.all(blocked_by)`) are shell-reserved characters that a bare word cannot carry at
/// all, so removing the quotes turns the run into a syntax error and `recorded` is `None`
/// (checked with real sh and bash) rather than a value that still parses but differs. That
/// is still "differs from the declared value", the property this asserts, but it is not the
/// quieter danger the Quoting paragraph itself names ("not an error but an expression the
/// shell has changed that still parses"); the examples built on `*` and on `\,` are the ones
/// that exercise that quieter case, since removing their quotes still parses.
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

/// The further hand-listed "safe" examples (`additional_safe_examples`) each reach the
/// stand-in as their own whitespace-split words, through every listed shell -- the same check
/// the pre-M-13 harness ran automatically for every example extraction found with no unsafe
/// character, now run over a fixed, hand-written list instead.
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

/// An example whose quotes were left out on purpose must turn the suite red, which needs
/// every listed shell to actually run: this asserts the harness is not the thing that stayed
/// green by accident, and that no shell above was skipped.
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
