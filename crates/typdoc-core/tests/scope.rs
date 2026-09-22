//! The scope of a command: what a prefix, `--namespace`, `TYPDOC_NAMESPACE` and the current
//! directory each choose, and which of them wins.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use typdoc_core::{Env, Error, Project, Scope, Source};
use typdoc_testkit::fixtures::path;

struct FakeEnv {
    cwd: PathBuf,
    vars: HashMap<&'static str, OsString>,
}

impl Env for FakeEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        self.vars.get(name).cloned()
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        Ok(self.cwd.clone())
    }

    fn hostname(&self) -> io::Result<String> {
        Ok("test-host".to_owned())
    }
}

fn env(cwd: &str, vars: &[(&'static str, &str)]) -> FakeEnv {
    FakeEnv {
        cwd: path(cwd),
        vars: vars.iter().map(|(k, v)| (*k, OsString::from(*v))).collect(),
    }
}

const SEVERAL: &str = "valid/several-namespaces";

fn no_vars() -> FakeEnv {
    env(SEVERAL, &[])
}

fn several() -> Project {
    Project::load(&path(SEVERAL), &no_vars()).expect("the fixture loads")
}

fn scope(prefix: Option<&str>, flag: Option<&str>, env: &FakeEnv) -> Result<Scope, Error> {
    several().scope(prefix, flag, env)
}

fn chosen(source: Source, names: &[&str]) -> Scope {
    Scope {
        source,
        namespaces: names.iter().map(|n| n.to_string()).collect(),
        imports: Vec::new(),
    }
}

#[test]
fn with_nothing_to_narrow_it_the_scope_is_every_namespace_of_the_project() {
    let outside = env(SEVERAL, &[]);

    assert_eq!(
        scope(None, None, &outside).unwrap(),
        chosen(Source::Everything, &["archive", "story-1", "story-2"])
    );
}

#[test]
fn a_current_directory_that_is_not_in_a_namespace_folder_does_not_narrow_it() {
    for cwd in [
        "valid/several-namespaces/other/notes",
        "valid/several-namespaces/.typdoc/collections",
        "valid/several-namespaces/schemas",
    ] {
        assert_eq!(
            scope(None, None, &env(cwd, &[])).unwrap().source,
            Source::Everything,
            "{cwd}"
        );
    }
}

#[test]
fn a_current_directory_in_a_namespace_folder_or_below_one_is_that_namespace() {
    for (cwd, name) in [
        ("valid/several-namespaces/story-2", "story-2"),
        ("valid/several-namespaces/story-1/notes", "story-1"),
        ("valid/several-namespaces/story-1/notes/deeper", "story-1"),
        ("valid/several-namespaces/archive", "archive"),
    ] {
        assert_eq!(
            scope(None, None, &env(cwd, &[])).unwrap(),
            chosen(Source::CurrentDirectory, &[name]),
            "{cwd}"
        );
    }
}

#[test]
fn a_variable_beats_the_current_directory() {
    let inside = env(
        "valid/several-namespaces/story-1",
        &[("TYPDOC_NAMESPACE", "story-2")],
    );

    assert_eq!(
        scope(None, None, &inside).unwrap(),
        chosen(Source::Variable, &["story-2"])
    );
}

#[test]
fn the_flag_beats_the_variable_and_a_prefix_beats_the_flag() {
    let both = env(
        "valid/several-namespaces/story-1",
        &[("TYPDOC_NAMESPACE", "story-2")],
    );

    assert_eq!(
        scope(None, Some("archive"), &both).unwrap(),
        chosen(Source::Flag, &["archive"])
    );
    assert_eq!(
        scope(Some("story-2"), Some("archive"), &both).unwrap(),
        chosen(Source::Prefix, &["story-2"])
    );
}

#[test]
fn a_list_and_a_glob_choose_namespaces_sorted_by_name_and_each_once() {
    let outside = env(SEVERAL, &[]);

    for (list, names) in [
        ("story-2,archive", &["archive", "story-2"][..]),
        ("story-*", &["story-1", "story-2"]),
        ("*", &["archive", "story-1", "story-2"]),
        ("story-1,story-*", &["story-1", "story-2"]),
        ("*-2,archive", &["archive", "story-2"]),
        ("nothing-*", &[]),
    ] {
        assert_eq!(
            scope(None, Some(list), &outside).unwrap(),
            chosen(Source::Flag, names),
            "{list}"
        );
    }
}

#[test]
fn the_variable_has_the_syntax_of_the_flag() {
    let with = |value: &str| env(SEVERAL, &[("TYPDOC_NAMESPACE", value)]);

    assert_eq!(
        scope(None, None, &with("story-*,archive")).unwrap(),
        chosen(Source::Variable, &["archive", "story-1", "story-2"])
    );
}

#[test]
fn an_empty_variable_is_not_set() {
    let empty = env(
        "valid/several-namespaces/story-1",
        &[("TYPDOC_NAMESPACE", "")],
    );

    assert_eq!(
        scope(None, None, &empty).unwrap(),
        chosen(Source::CurrentDirectory, &["story-1"])
    );
}

#[test]
fn a_name_that_is_no_namespace_of_the_project_is_bad_arguments_and_names_the_origin() {
    let outside = env(SEVERAL, &[]);
    let bad_variable = env(SEVERAL, &[("TYPDOC_NAMESPACE", "nosuch")]);

    let flag = scope(None, Some("story-1,nosuch"), &outside).unwrap_err();
    let variable = scope(None, None, &bad_variable).unwrap_err();
    let prefix = scope(Some("nosuch"), None, &outside).unwrap_err();

    for (error, origin) in [
        (flag, "--namespace"),
        (variable, "TYPDOC_NAMESPACE"),
        (prefix, "prefix"),
    ] {
        assert!(matches!(error, Error::BadArgument(_)), "{origin}: {error}");
        let text = error.to_string();
        assert!(text.contains(origin) && text.contains("nosuch"), "{text}");
        assert!(
            text.contains("story-1"),
            "the message lists the namespaces: {text}"
        );
    }
}

#[test]
fn a_list_that_is_malformed_is_bad_arguments() {
    let outside = env(SEVERAL, &[]);

    for list in [
        "",
        ",",
        "story-1,",
        "a b",
        "story/1",
        "**",
        "a**",
        "story-1;story-2",
        "\u{e9}",
    ] {
        let error = scope(None, Some(list), &outside).unwrap_err();

        assert!(matches!(error, Error::BadArgument(_)), "{list:?}: {error}");
    }
}

#[test]
fn a_name_that_is_no_import_of_the_project_is_bad_arguments() {
    let error = scope(None, Some("other::*"), &env(SEVERAL, &[])).unwrap_err();

    assert!(matches!(error, Error::BadArgument(_)), "{error}");
    assert!(error.to_string().contains("other"), "{error}");
}

#[test]
fn a_project_with_one_namespace_is_in_scope_whatever_narrows_it() {
    let project = Project::load(&path("valid/minimal"), &no_vars()).unwrap();
    let inside = env("valid/minimal/schemas", &[]);

    assert_eq!(
        project.scope(None, None, &inside).unwrap(),
        chosen(Source::Everything, &["default"])
    );
    assert_eq!(
        project.scope(None, Some("default"), &inside).unwrap(),
        chosen(Source::Flag, &["default"])
    );
    assert!(project.scope(None, Some("story-1"), &inside).is_err());
}

#[test]
fn a_scope_says_whether_it_holds_a_namespace() {
    let scope = chosen(Source::Flag, &["a", "b"]);

    assert!(scope.contains("a"));
    assert!(!scope.contains("c"));
}
