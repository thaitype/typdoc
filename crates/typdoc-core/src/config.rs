use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{ConfigError, Error};
use crate::namespaces;
use crate::rules::{ALWAYS_ON, CONFIGURABLE};

pub const TYPDOC_DIR: &str = ".typdoc";
pub(crate) const CONFIG_FILE: &str = ".typdoc/config.json";
const LEGACY_FILE: &str = ".typdoc.json";
const COLLECTIONS_DIR: &str = ".typdoc/collections";

/// A name made of ASCII letters, digits, `-` and `_`, as collections and namespaces are.
pub(crate) fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn config_file(project: &Path) -> PathBuf {
    project.join(CONFIG_FILE)
}

/// A namespace of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub name: String,
    /// Relative to the project folder; empty for `default`, which has no folder of its own.
    pub folder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Off,
    Warn,
    Error,
}

/// What one rule is set to. A rule may state only what differs, so each part can be absent.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSetting {
    pub level: Option<Level>,
    /// The options besides `level`, as written.
    pub options: Map<String, Value>,
}

/// Rule settings by rule id.
pub type Rules = BTreeMap<String, RuleSetting>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefBase {
    File,
    Namespace,
}

#[derive(Debug)]
pub struct Collection {
    pub name: String,
    /// The `match` as written.
    pub pattern: String,
    /// Relative to the project folder.
    pub schema: String,
    pub ref_base: RefBase,
    pub validation: Rules,
    /// The collection file, for messages.
    pub file: PathBuf,
    /// The collection file from the project folder, as a config error names it.
    pub path: String,
}

#[derive(Debug)]
pub struct Config {
    pub namespaces: Vec<Namespace>,
    /// `validation.global`.
    pub validation: Rules,
    /// Sorted by name.
    pub collections: Vec<Collection>,
}

/// The config errors found so far.
///
/// Every error added here stops the command today, whichever way it was added: `stop` ends the
/// list at once (`complete: false`, the rest of the config could not be interpreted); `finish`
/// turns whatever `add` collected into the same kind of failure (`complete: true`, everything
/// else was determined). The design's own question for a config error, "does it make checking
/// impossible?", is answered here only once, for the whole type, and not error by error: an
/// error added through `add`, not `stop`, has already been read as one that leaves the rest of
/// the config readable (parsing continues past it), which is the design's condition for a
/// finding in `validate`'s report rather than a stopped command — `config.legacy-file` is a
/// plain example, since a stray `.typdoc.json` beside the folder says nothing about whether
/// `.typdoc/config.json` itself can be read. None of the errors this crate adds answers that
/// question on its own yet, `config.legacy-file` included: `finish` stops the command for all
/// of them alike. Letting one answer it on its own, so it could reach `validate` as a finding
/// instead, is a change to this type and is not made here.
#[derive(Default)]
pub(crate) struct Report {
    errors: Vec<ConfigError>,
}

impl Report {
    pub(crate) fn add(&mut self, id: &'static str, path: &str, message: String) {
        self.errors.push(ConfigError {
            id,
            path: path.to_owned(),
            message,
        });
    }

    /// Adds one error and ends the list there, because the rest of the config cannot be
    /// interpreted.
    fn stop(&mut self, id: &'static str, path: &str, message: String) -> Error {
        self.add(id, path, message);
        Error::ConfigErrors {
            errors: ordered(std::mem::take(self).errors),
            complete: false,
        }
    }

    pub(crate) fn finish(self) -> Result<(), Error> {
        if self.errors.is_empty() {
            return Ok(());
        }
        Err(Error::ConfigErrors {
            errors: ordered(self.errors),
            complete: true,
        })
    }
}

/// By path, then id, then message: the order of findings, with no position to break a tie.
fn ordered(mut errors: Vec<ConfigError>) -> Vec<ConfigError> {
    errors.sort_by(|a, b| (&a.path, a.id, &a.message).cmp(&(&b.path, b.id, &b.message)));
    errors
}

impl Config {
    /// Reads `config.json` and every collection file into `report`, which the caller finishes,
    /// so that the errors of what is read next can join them. Every config error that can be
    /// determined is reported together, and `Err` is a config that cannot be interpreted any
    /// further, with the list ended there.
    pub(crate) fn load(root: &Path, report: &mut Report) -> Result<Config, Error> {
        if root.join(LEGACY_FILE).symlink_metadata().is_ok() {
            report.add(
                "config.legacy-file",
                LEGACY_FILE,
                format!("{LEGACY_FILE} is not read: move it to {CONFIG_FILE}"),
            );
        }
        let top = read_config_json(root, report)?;
        let mut validation = Rules::new();
        let mut entries = None;
        for (key, value) in &top {
            match key.as_str() {
                "version" => {}
                "namespaces" => entries = Some(namespace_entries(value, report)),
                "validation" => validation = global_rules(value, report)?,
                "imports" => {}
                "lock" => check_lock(value, report)?,
                other => report.add(
                    "config.unknown-key",
                    CONFIG_FILE,
                    format!("`{other}` is not a key of config.json"),
                ),
            }
        }
        let namespaces = namespaces::resolve(root, entries.as_deref(), report)?;
        let collections = read_collections(root, report)?;
        Ok(Config {
            namespaces,
            validation,
            collections,
        })
    }
}

/// `config.json` as an object whose `version` is known. Anything else ends the list.
fn read_config_json(root: &Path, report: &mut Report) -> Result<Map<String, Value>, Error> {
    let file = config_file(root);
    let bytes = fs::read(&file).map_err(Error::io_at(&file))?;
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => {
            let message = format!("cannot be parsed: {e}");
            return Err(report.stop("config.parse", CONFIG_FILE, message));
        }
    };
    let Value::Object(top) = value else {
        let message = "must be one JSON object".to_owned();
        return Err(report.stop("config.parse", CONFIG_FILE, message));
    };
    match top.get("version") {
        Some(Value::Number(n)) if n.as_u64() == Some(1) => Ok(top),
        Some(other) => {
            let message = format!("version {other} is not known: it must be 1");
            Err(report.stop("config.version", CONFIG_FILE, message))
        }
        None => {
            let message = "version is missing: it must be 1".to_owned();
            Err(report.stop("config.version", CONFIG_FILE, message))
        }
    }
}

fn namespace_entries(value: &Value, report: &mut Report) -> Vec<String> {
    let mut entries = Vec::new();
    let refuse = |report: &mut Report, message: String| {
        report.add("config.namespaces-entry", CONFIG_FILE, message);
    };
    match value {
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(text) => entries.push(text.clone()),
                    other => refuse(
                        report,
                        format!("the entry {other} of `namespaces` is not text"),
                    ),
                }
            }
        }
        Value::String(text) => entries.push(text.clone()),
        other => refuse(
            report,
            format!("`namespaces` is {other}: it is a name, a glob or an array of them"),
        ),
    }
    entries
}

fn global_rules(value: &Value, report: &mut Report) -> Result<Rules, Error> {
    let Value::Object(validation) = value else {
        let message = "`validation` must be an object".to_owned();
        return Err(report.stop("config.parse", CONFIG_FILE, message));
    };
    let mut global = Rules::new();
    for (key, value) in validation {
        if key != "global" {
            report.add(
                "config.unknown-key",
                CONFIG_FILE,
                format!("`{key}` is not a key of `validation`: only `global` is"),
            );
            continue;
        }
        global = match rules(value, CONFIG_FILE, report) {
            Ok(found) => found,
            Err(message) => return Err(report.stop("config.parse", CONFIG_FILE, message)),
        };
    }
    Ok(global)
}

fn check_lock(value: &Value, report: &mut Report) -> Result<(), Error> {
    if matches!(value.as_str(), Some("local" | "git-common")) {
        return Ok(());
    }
    let message = format!("`lock` is {value}: it is `local` or `git-common`");
    Err(report.stop("config.parse", CONFIG_FILE, message))
}

/// The rule settings of a `validation` object. A setting that names an unknown rule or option
/// is reported and left out; an object of another shape is returned as the reason.
fn rules(value: &Value, path: &str, report: &mut Report) -> Result<Rules, String> {
    let Value::Object(settings) = value else {
        return Err("`validation` holds an object of rules".to_owned());
    };
    let mut found = Rules::new();
    for (rule, setting) in settings {
        let Value::Object(setting) = setting else {
            return Err(format!("the setting of `{rule}` must be an object"));
        };
        if ALWAYS_ON.contains(&rule.as_str()) {
            report.add(
                "config.rule-always-on",
                path,
                format!("`{rule}` is always on and is not configured"),
            );
        } else if let Some((_, allowed)) = CONFIGURABLE.iter().find(|(name, _)| name == rule) {
            if let Some(setting) = rule_setting(rule, setting, allowed, path, report) {
                found.insert(rule.clone(), setting);
            }
        } else {
            report.add(
                "config.rule-unknown",
                path,
                format!("`{rule}` is not a rule"),
            );
        }
    }
    Ok(found)
}

fn rule_setting(
    rule: &str,
    setting: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
    report: &mut Report,
) -> Option<RuleSetting> {
    let mut level = None;
    let mut options = Map::new();
    let mut valid = true;
    for (key, value) in setting {
        let problem = match key.as_str() {
            "level" => match value.as_str().and_then(level_named) {
                Some(named) => {
                    level = Some(named);
                    None
                }
                None => Some(format!(
                    "the level of `{rule}` is {value}: it is `off`, `warn` or `error`"
                )),
            },
            _ if !allowed.contains(&key.as_str()) => {
                Some(format!("`{rule}` has no option `{key}`"))
            }
            _ if !option_fits(key, value) => Some(format!(
                "the option `{key}` of `{rule}` is {value}, which it does not accept"
            )),
            _ => {
                options.insert(key.clone(), value.clone());
                None
            }
        };
        if let Some(problem) = problem {
            report.add("config.rule-unknown", path, problem);
            valid = false;
        }
    }
    valid.then_some(RuleSetting { level, options })
}

fn level_named(name: &str) -> Option<Level> {
    match name {
        "off" => Some(Level::Off),
        "warn" => Some(Level::Warn),
        "error" => Some(Level::Error),
        _ => None,
    }
}

/// The type of each option, from the table of rules in the design.
fn option_fits(option: &str, value: &Value) -> bool {
    match option {
        "ignore" => matches!(value, Value::Array(items) if items.iter().all(Value::is_string)),
        _ => value.is_boolean(),
    }
}

/// Every `*.json` file in `.typdoc/collections/`, sorted by name; other files are ignored.
fn read_collections(root: &Path, report: &mut Report) -> Result<Vec<Collection>, Error> {
    let dir = root.join(COLLECTIONS_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { file: dir, source }),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(Error::io_at(&dir))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file_name.strip_suffix(".json") else {
            continue;
        };
        let path = format!("{COLLECTIONS_DIR}/{file_name}");
        if !plain_name(name) {
            report.add(
                "config.collection-name",
                &path,
                format!(
                    "the collection file name `{name}` uses something other than ASCII letters, digits, `-` and `_`"
                ),
            );
        }
        let file = entry.path();
        let bytes = fs::read(&file).map_err(Error::io_at(&file))?;
        if let Some(collection) = read_collection(name, &path, file, &bytes, report) {
            found.push(collection);
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

fn read_collection(
    name: &str,
    path: &str,
    file: PathBuf,
    bytes: &[u8],
    report: &mut Report,
) -> Option<Collection> {
    let parse_error = |report: &mut Report, message: String| {
        report.add("config.collection-parse", path, message);
    };
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(e) => {
            parse_error(report, format!("cannot be parsed: {e}"));
            return None;
        }
    };
    let Value::Object(top) = value else {
        parse_error(report, "must be one JSON object".to_owned());
        return None;
    };
    let mut pattern = None;
    let mut schema = None;
    let mut ref_base = RefBase::File;
    let mut validation = Rules::new();
    let mut valid = true;
    for (key, value) in &top {
        match (key.as_str(), value) {
            ("match", Value::String(text)) => pattern = Some(text.clone()),
            ("schema", Value::String(text)) => schema = Some(text.clone()),
            ("refBase", Value::String(text)) if text == "file" => ref_base = RefBase::File,
            ("refBase", Value::String(text)) if text == "namespace" => {
                ref_base = RefBase::Namespace;
            }
            ("validation", value) => match rules(value, path, report) {
                Ok(found) => validation = found,
                Err(message) => {
                    parse_error(report, message);
                    valid = false;
                }
            },
            ("match" | "schema" | "refBase", other) => {
                let expected = if key == "refBase" {
                    "`file` or `namespace`"
                } else {
                    "text"
                };
                parse_error(report, format!("`{key}` is {other}: it is {expected}"));
                valid = false;
            }
            (other, _) => report.add(
                "config.unknown-key",
                path,
                format!("`{other}` is not a key of a collection file"),
            ),
        }
    }
    for key in ["match", "schema"] {
        if !top.contains_key(key) {
            parse_error(report, format!("`{key}` is missing"));
            valid = false;
        }
    }
    let (Some(pattern), Some(schema), true) = (pattern, schema, valid) else {
        return None;
    };
    Some(Collection {
        name: name.to_owned(),
        pattern,
        schema,
        ref_base,
        validation,
        file,
        path: path.to_owned(),
    })
}
