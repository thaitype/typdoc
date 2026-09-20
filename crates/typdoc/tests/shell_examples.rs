//! The harness for `docs/design.md`'s shell examples: every one is read from the document
//! itself (`typdoc_testkit::shell_examples`), so this file and the design share one list.
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
//! split on whitespace, computed here rather than written by hand. An example that is not a
//! literal, standalone invocation (a placeholder, a runtime value, a line of a longer script,
//! illustrative output) is named as a template instead, with the reason.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use typdoc_testkit::fixtures::design_text;
use typdoc_testkit::shell_examples::{examples, quoting_paragraph_spans};

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

/// One example the design states in full: what is run (`command`, exactly the example's
/// text, with `typdoc` supplied where the text is a bare `--` fragment) and the value
/// declared by hand from the design's own words about quoting.
struct DeclaredExample {
    /// The example's text, exactly as `typdoc_testkit::shell_examples::examples` reads it
    /// from the design; this is the key an orphan or a missing declaration is named by.
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

/// Every example in the design that needs a value declared by hand: a literal, standalone
/// invocation with a character outside the safe set. Each value is what "give it to typdoc
/// exactly as written, in single quotes" (Quoting in the shell) means for that one example,
/// worked out from the design's own words, never from running anything.
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

/// Every example in the design that is not a literal, standalone invocation, with the
/// reason: a placeholder, a value only known at runtime, a line of a longer script, or
/// illustrative output rather than something typed.
fn templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("--namespace <list>", "<list> is a placeholder"),
        ("typdoc new <path>", "<path> is a placeholder"),
        (
            r#"typdoc new <CODE> "<title>" [--set k=v ...]"#,
            "<CODE> is a redirection in a shell, and \"<title>\" and [--set k=v ...] are placeholders",
        ),
        (
            "typdoc new <path> [--set k=v ...]",
            "<path> and [--set k=v ...] are placeholders",
        ),
        (
            "typdoc get <key|path> [--json]",
            "<key|path> is a placeholder",
        ),
        (
            "typdoc list [--collection c[,c]] [--code C[,C]] [--where EXPR ...] [--fields f,...]",
            "the synopsis of typdoc list: every bracketed part is a placeholder",
        ),
        (
            "--sort field:dir",
            "field and dir are placeholder names, not literal text",
        ),
        (
            "typdoc set <key|path> k=v [k=v ...] [--if EXPR ...]",
            "the synopsis of typdoc set: every bracketed part is a placeholder",
        ),
        (
            "typdoc toc <key|path> [--depth n] [--json]",
            "<key|path> is a placeholder",
        ),
        (
            "typdoc refs <key|path> [--field f|$body] [--reverse] [--json]",
            "<key|path> is a placeholder",
        ),
        (
            "typdoc mv <from> <to> [--renumber]",
            "<from> and <to> are placeholders",
        ),
        (
            "typdoc pull [<url> ...] [--check]",
            "<url> is a placeholder",
        ),
        (
            "typdoc validate [<key|path> ...] [--schemas] [--strict] [--audit]",
            "<key|path> is a placeholder",
        ),
        (
            "typdoc audit: 3 collections, 214 files (12 in no collection)",
            "illustrative command output, not something typed",
        ),
        (
            r#"typdoc new WF "..." --set kind=grilling"#,
            "\"...\" stands for an omitted title, not literal text",
        ),
        (
            r#"typdoc new WF "..." --set kind=feature --set blocked_by=WF-3"#,
            "\"...\" stands for an omitted title, not literal text",
        ),
        (
            "typdoc set WF-3 status=claimed owner=$SID --if status=open",
            "$SID is a value the caller sets at runtime, not literal text",
        ),
        (
            "typdoc set WF-3 status=resolved --if owner=$SID",
            "$SID is a value the caller sets at runtime, not literal text",
        ),
        (
            "--where status=open --where 'ref.all(blocked_by).status=resolved' --limit 1 --ids) && [ -n \"$key\" ]; do",
            "a continuation line of a multi-line while loop, not a standalone invocation",
        ),
        (
            "typdoc set \"$key\" status=claimed owner=\"$SID\" --if status=open && break",
            "a line of the same while loop; $key and $SID are runtime values",
        ),
        (
            "TYPDOC_NAMESPACE='*'",
            "an assignment's right-hand side is never split or glob-expanded, quoted or not \
             (checked with * against files present, in both sh and bash), so removing the \
             quotes here would not change what typdoc receives; shown for the style, not \
             because this one is quote-sensitive",
        ),
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

fn declared_texts() -> std::collections::BTreeSet<&'static str> {
    declared_examples().iter().map(|e| e.text).collect()
}

fn template_texts() -> std::collections::BTreeSet<&'static str> {
    templates().iter().map(|(text, _)| *text).collect()
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
fn every_unsafe_example_has_a_declared_value_and_every_declared_value_matches_an_example() {
    let design = design_text();
    let extracted = examples(&design);
    let extracted_set: std::collections::BTreeSet<&str> =
        extracted.iter().map(String::as_str).collect();
    let declared = declared_texts();
    let templated = template_texts();

    let mut problems = Vec::new();
    for text in &extracted {
        if templated.contains(text.as_str()) {
            continue;
        }
        if has_unsafe_char(text) && !declared.contains(text.as_str()) {
            problems.push(format!("no declared value for the example: {text}"));
        }
    }
    for text in declared.iter().chain(templated.iter()) {
        if !extracted_set.contains(text) {
            problems.push(format!(
                "a declared value or template names no example in the design: {text}"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
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

#[test]
fn every_example_with_no_unsafe_character_reaches_the_stand_in_as_its_own_words() {
    let design = design_text();
    let declared = declared_texts();
    let templated = template_texts();
    let dir = star_matching_dir();

    for text in examples(&design) {
        if declared.contains(text.as_str()) || templated.contains(text.as_str()) {
            continue;
        }
        assert!(
            !has_unsafe_char(&text),
            "example has a character outside the safe set and no declared value: {text}"
        );
        let (command, expected) = safe_command_and_expected(&text);
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

#[test]
fn every_code_span_of_the_quoting_paragraph_is_classified() {
    let design = design_text();
    let spans = quoting_paragraph_spans(&design);

    let expected_spans = [
        "--where",
        "--if",
        "--set",
        "--namespace",
        r"--where 'title=Cosmos\, or SQL'",
        "--namespace '*'",
        "--namespace 'chief::*'",
        "TYPDOC_NAMESPACE='*'",
        "\\",
        "*",
        "<",
        ">",
        "!",
    ];
    assert_eq!(spans, expected_spans, "the paragraph's spans changed");

    let flag_names = ["--where", "--if", "--set", "--namespace"];
    let single_characters = ["\\", "*", "<", ">", "!"];
    let declared = declared_texts();
    let templated = template_texts();

    for span in &spans {
        let classified = flag_names.contains(&span.as_str())
            || single_characters.contains(&span.as_str())
            || declared.contains(span.as_str())
            || templated.contains(span.as_str());
        assert!(
            classified,
            "unclassified span in the Quoting paragraph: {span}"
        );
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
