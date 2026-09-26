//! The machine file `imports.json`, and `${NAME}` substitution in an import path (SPC-14).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::{CONFIG_FILE, Report};
use crate::env::Env;
use crate::error::Error;

const IMPORTS_FILE_NAME: &str = "imports.json";

/// Why an import is absent on this machine, for `imports.absent`'s message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Absence {
    /// `${name}` in the import's path is unset or empty.
    Variable(String),
    /// The path substituted cleanly but names no project: no `.typdoc/config.json` there.
    NoProject(PathBuf),
}

impl Absence {
    pub(crate) fn message(&self) -> String {
        match self {
            Absence::Variable(name) => format!("{name} is not set"),
            Absence::NoProject(path) => {
                format!(
                    "{} has no {CONFIG_FILE}: not set up on this machine",
                    path.display()
                )
            }
        }
    }
}

/// A variable that is unset or empty is never replaced by an empty string: `Err` names the first
/// such variable. A `${` with no closing `}` is kept as written, so that a path holding a literal
/// `${` is not refused.
pub(crate) fn substitute(raw: &str, env: &dyn Env) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("${") {
        let Some(end) = rest[start + 2..].find('}') else {
            out.push_str(rest);
            rest = "";
            break;
        };
        let end = start + 2 + end;
        out.push_str(&rest[..start]);
        let name = &rest[start + 2..end];
        let value = env
            .var(name)
            .filter(|v| !v.is_empty())
            .and_then(|v| v.into_string().ok())
            .ok_or_else(|| name.to_owned())?;
        out.push_str(&value);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Where the machine file is (SPC-14); `None` when no step applies. A `TYPDOC_CONFIG_DIR` that is
/// not an absolute path to an existing directory is `config.config-dir`, since it was set on
/// purpose. An empty `TYPDOC_CONFIG_DIR` counts as unset, as an empty `XDG_CONFIG_HOME`,
/// `TYPDOC_DIR` or `TYPDOC_NAMESPACE` does.
pub(crate) fn imports_file_path(env: &dyn Env, report: &mut Report) -> Option<PathBuf> {
    if let Some(dir) = env.var("TYPDOC_CONFIG_DIR").filter(|v| !v.is_empty()) {
        let dir = PathBuf::from(dir);
        if !dir.is_absolute() || !dir.is_dir() {
            report.add(
                "config.config-dir",
                CONFIG_FILE,
                format!(
                    "TYPDOC_CONFIG_DIR is `{}`: it must be an absolute path to a directory that exists",
                    dir.display()
                ),
            );
            return None;
        }
        return Some(dir.join(IMPORTS_FILE_NAME));
    }
    if let Some(xdg) = env.var("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        let xdg = PathBuf::from(xdg);
        if xdg.is_absolute() {
            return Some(xdg.join("typdoc").join(IMPORTS_FILE_NAME));
        }
    }
    let home = env.var("HOME").filter(|v| !v.is_empty())?;
    Some(
        PathBuf::from(home)
            .join(".config/typdoc")
            .join(IMPORTS_FILE_NAME),
    )
}

/// `path` is found by `imports_file_path` while the project's `Report` is still open, so that a
/// `config.config-dir` joins its other config errors. A machine file that cannot be read as an
/// object of alias to text path has no id in the config errors catalog (SPC-6), and is
/// `Error::Config`.
pub(crate) fn read_machine_file(path: Option<&PathBuf>) -> Result<BTreeMap<String, String>, Error> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => {
            return Err(Error::Io {
                file: path.clone(),
                source,
            });
        }
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| Error::Config {
        file: path.clone(),
        message: format!("cannot be parsed: {e}"),
    })?;
    let serde_json::Value::Object(entries) = value else {
        return Err(Error::Config {
            file: path.clone(),
            message: "must be one JSON object of alias to path".to_owned(),
        });
    };
    let mut imports = BTreeMap::new();
    for (alias, text) in entries {
        let Some(text) = text.as_str() else {
            return Err(Error::Config {
                file: path.clone(),
                message: format!("the import `{alias}` is {text}: it must be a text path"),
            });
        };
        imports.insert(alias, text.to_owned());
    }
    Ok(imports)
}

/// Shared so that a `project::` argument prefix and a `--namespace` item refuse an unknown alias
/// in the same words.
pub(crate) fn unknown_alias_message(alias: &str, known: &[&str]) -> String {
    format!(
        "`{alias}` is not an import of this project, which has: {}",
        known.join(", ")
    )
}

/// The project's own entry wins over the machine file's for the same alias (SPC-14): it is the
/// one every teammate and CI sees, and the machine file only adds what is not committed.
pub(crate) fn merge(
    project: &BTreeMap<String, String>,
    machine: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = machine;
    merged.extend(project.iter().map(|(k, v)| (k.clone(), v.clone())));
    merged
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io;

    use super::*;

    struct FakeEnv {
        vars: BTreeMap<&'static str, String>,
    }

    impl Env for FakeEnv {
        fn var(&self, name: &str) -> Option<OsString> {
            self.vars.get(name).map(OsString::from)
        }

        fn current_dir(&self) -> io::Result<PathBuf> {
            Ok(PathBuf::from("."))
        }

        fn hostname(&self) -> String {
            "fake-host".to_owned()
        }
    }

    fn env(vars: &[(&'static str, &str)]) -> FakeEnv {
        FakeEnv {
            vars: vars.iter().map(|(k, v)| (*k, (*v).to_owned())).collect(),
        }
    }

    fn fixtures_root() -> PathBuf {
        typdoc_testkit::fixtures::path("")
    }

    #[test]
    fn a_variable_present_and_non_empty_is_substituted() {
        let env = env(&[("TYPMEM_DIR", "/home/x/memory")]);

        assert_eq!(
            substitute("${TYPMEM_DIR}/learnings", &env).unwrap(),
            "/home/x/memory/learnings"
        );
    }

    #[test]
    fn an_unset_variable_is_never_replaced_by_an_empty_string() {
        let env = env(&[]);

        let missing = substitute("${TYPMEM_DIR}/learnings", &env).unwrap_err();

        assert_eq!(missing, "TYPMEM_DIR");
    }

    #[test]
    fn an_empty_variable_counts_as_unset() {
        let env = env(&[("TYPMEM_DIR", "")]);

        let missing = substitute("${TYPMEM_DIR}/learnings", &env).unwrap_err();

        assert_eq!(missing, "TYPMEM_DIR");
    }

    #[test]
    fn a_path_with_no_variable_is_returned_as_written() {
        let env = env(&[]);

        assert_eq!(substitute("../memory", &env).unwrap(), "../memory");
    }

    #[test]
    fn typdoc_config_dir_beats_a_simultaneously_set_xdg_config_home() {
        let mut report = Report::default();
        let root = fixtures_root();
        let env = env(&[
            ("TYPDOC_CONFIG_DIR", root.to_str().unwrap()),
            ("XDG_CONFIG_HOME", "/should-not-be-read"),
        ]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, Some(root.join("imports.json")));
        assert!(report.finish().is_ok());
    }

    #[test]
    fn xdg_config_home_alone_gives_that_path_and_not_the_platform_default() {
        let mut report = Report::default();
        let root = fixtures_root();
        let env = env(&[
            ("XDG_CONFIG_HOME", root.to_str().unwrap()),
            ("HOME", "/should-not-be-read"),
        ]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, Some(root.join("typdoc/imports.json")));
    }

    #[test]
    fn with_neither_set_a_fake_home_gives_the_platform_default() {
        let mut report = Report::default();
        let root = fixtures_root();
        let env = env(&[("HOME", root.to_str().unwrap())]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, Some(root.join(".config/typdoc/imports.json")));
    }

    #[test]
    fn an_empty_xdg_config_home_falls_to_the_platform_default() {
        let mut report = Report::default();
        let root = fixtures_root();
        let env = env(&[("XDG_CONFIG_HOME", ""), ("HOME", root.to_str().unwrap())]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, Some(root.join(".config/typdoc/imports.json")));
    }

    #[test]
    fn a_relative_xdg_config_home_falls_to_the_platform_default() {
        let mut report = Report::default();
        let root = fixtures_root();
        let env = env(&[
            ("XDG_CONFIG_HOME", "relative/place"),
            ("HOME", root.to_str().unwrap()),
        ]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, Some(root.join(".config/typdoc/imports.json")));
    }

    #[test]
    fn with_nothing_set_there_is_no_machine_file_and_no_error() {
        let mut report = Report::default();
        let env = env(&[]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, None);
        assert!(report.finish().is_ok());
    }

    fn config_dir_error_id(report: Report) -> String {
        match report.finish().unwrap_err() {
            Error::ConfigErrors { errors, .. } => {
                assert_eq!(errors.len(), 1, "{errors:?}");
                errors[0].id.to_owned()
            }
            other => panic!("expected ConfigErrors, got {other}"),
        }
    }

    #[test]
    fn typdoc_config_dir_that_is_relative_is_config_dot_config_dir() {
        let mut report = Report::default();
        let env = env(&[("TYPDOC_CONFIG_DIR", "relative")]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, None);
        assert_eq!(config_dir_error_id(report), "config.config-dir");
    }

    #[test]
    fn typdoc_config_dir_naming_a_directory_that_does_not_exist_is_config_dot_config_dir() {
        let mut report = Report::default();
        let missing = fixtures_root().join("no-such-directory-at-all");
        let env = env(&[("TYPDOC_CONFIG_DIR", missing.to_str().unwrap())]);

        let path = imports_file_path(&env, &mut report);

        assert_eq!(path, None);
        assert_eq!(config_dir_error_id(report), "config.config-dir");
    }

    #[test]
    fn merge_prefers_the_projects_own_entry_over_the_machine_files() {
        let project: BTreeMap<String, String> =
            [("memory".to_owned(), "./committed".to_owned())].into();
        let machine: BTreeMap<String, String> =
            [("memory".to_owned(), "./local-machine".to_owned())].into();

        let merged = merge(&project, machine);

        assert_eq!(merged.get("memory"), Some(&"./committed".to_owned()));
    }

    #[test]
    fn merge_adds_an_alias_the_project_does_not_have() {
        let project: BTreeMap<String, String> = BTreeMap::new();
        let machine: BTreeMap<String, String> =
            [("memory".to_owned(), "./local-machine".to_owned())].into();

        let merged = merge(&project, machine);

        assert_eq!(merged.get("memory"), Some(&"./local-machine".to_owned()));
    }
}
