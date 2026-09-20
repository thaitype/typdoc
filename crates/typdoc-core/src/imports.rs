//! Machine-specific imports: `imports.json`, found through `TYPDOC_CONFIG_DIR`, then
//! `XDG_CONFIG_HOME`, then the platform default, merged under the project's own `imports`; and
//! `${NAME}` substitution in an import path, the same rule for both sources. Every reading of
//! the environment here goes through `Env`, never the standard library directly (contract item
//! 6; `clippy.toml` bans `std::env::var`, `std::env::var_os` and `std::env::home_dir`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::{CONFIG_FILE, Report};
use crate::env::Env;
use crate::error::Error;

const IMPORTS_FILE_NAME: &str = "imports.json";

/// Why an import cannot be read on this machine right now, for `imports.absent`'s message and
/// for the `import-absent` reason a ref into it gets.
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

/// `${NAME}` in `raw` replaced by the environment. A variable that is unset or set to an empty
/// value is never replaced by an empty string (design, "Environment variables in import paths"):
/// `Err` names the first such variable, left to right, for the caller to report as `imports.
/// absent`. A `${` with no closing `}` is not a variable reference and is kept as written, since
/// the design gives no syntax for that case and treating it as one would refuse a path that
/// happens to contain a literal `${`.
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

/// The path of the machine file `imports.json`, found in the order the design gives: `Ok(None)`
/// when no step applies (there are no machine-specific imports); a `config.config-dir` finding,
/// added to `report`, when `TYPDOC_CONFIG_DIR` is set but is not an absolute path to a directory
/// that exists — "it was set on purpose". Neither `TYPDOC_CONFIG_DIR` nor `XDG_CONFIG_HOME` is
/// read as set when its value is empty (the design states this for `XDG_CONFIG_HOME`; the same
/// reading is carried to `TYPDOC_CONFIG_DIR` for consistency with how this crate already treats
/// every other environment variable that chooses something, `TYPDOC_DIR` and `TYPDOC_NAMESPACE`
/// included).
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

/// The machine file's own `imports`, read from the path `imports_file_path` already found (found
/// separately, and before this runs, so a `config.config-dir` fault joins the project's other
/// config errors in one `Report` before any file is read — `Report::finish` must not have run
/// yet when that search happens). `None` (the search found no path) or a file that does not
/// exist both mean there are no machine-specific imports, not an error. A file that exists and
/// cannot be read as an object of alias to text path has no id of its own in the design's table
/// (unlike `config.json`, it is never committed, so no fixture in a public repository exercises
/// it); it is reported the same way a config fault with no id yet already is elsewhere in this
/// crate (ticket 4's report on `collections.overlap` and a missing schema): `Error::Config`, exit
/// 2, `details: []`.
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

/// The message for an alias that names no import of this project: shared so the wording agrees
/// wherever one is refused — a `project::` argument prefix (`Project::imported`) and a
/// `--namespace`/`TYPDOC_NAMESPACE` item (`scope::select`) both read an unknown alias the same
/// way, and now say so in the same words as each other too.
pub(crate) fn unknown_alias_message(alias: &str, known: &[&str]) -> String {
    format!(
        "`{alias}` is not an import of this project, which has: {}",
        known.join(", ")
    )
}

/// The project's own `imports` (`config.json`, committed) and the machine file's (uncommitted),
/// merged: the design says the machine file is "merged under the project's own imports", read
/// here as the project's own committed entry winning when both name the same alias, since it is
/// the one every teammate and CI sees and the machine file exists only to add what is not
/// committed, not to override what is.
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
