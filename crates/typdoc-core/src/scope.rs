//! The namespaces a command reads: chosen from a prefix on an argument, `--namespace`,
//! `TYPDOC_NAMESPACE` and the current directory, in that order.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, plain_name};
use crate::env::Env;
use crate::error::Error;
use crate::template::Segment;

/// What decided the scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Prefix,
    Flag,
    Variable,
    CurrentDirectory,
    /// Nothing narrowed it: reads span every namespace of the project.
    Everything,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub source: Source,
    /// The namespaces in scope, sorted by name.
    pub namespaces: Vec<String>,
}

impl Scope {
    pub fn contains(&self, namespace: &str) -> bool {
        self.namespaces.iter().any(|name| name == namespace)
    }
}

/// The scope of a command in the project at `root`. `prefix` is the namespace an argument
/// names, and `flag` is the value of `--namespace`.
pub(crate) fn choose(
    namespaces: &[Namespace],
    root: &Path,
    prefix: Option<&str>,
    flag: Option<&str>,
    env: &dyn Env,
) -> Result<Scope, Error> {
    if let Some(prefix) = prefix {
        return Ok(Scope {
            source: Source::Prefix,
            namespaces: select(namespaces, prefix, "a prefix")?,
        });
    }
    if let Some(list) = flag {
        return Ok(Scope {
            source: Source::Flag,
            namespaces: select(namespaces, list, "--namespace")?,
        });
    }
    if let Some(list) = env.var("TYPDOC_NAMESPACE").filter(|v| !v.is_empty()) {
        let list = list.into_string().map_err(|value| {
            Error::BadArgument(format!("TYPDOC_NAMESPACE is not valid UTF-8: {value:?}"))
        })?;
        return Ok(Scope {
            source: Source::Variable,
            namespaces: select(namespaces, &list, "TYPDOC_NAMESPACE")?,
        });
    }
    if let Some(inside) = inside(namespaces, root, env) {
        return Ok(Scope {
            source: Source::CurrentDirectory,
            namespaces: vec![inside],
        });
    }
    Ok(Scope {
        source: Source::Everything,
        namespaces: names(namespaces),
    })
}

fn names(namespaces: &[Namespace]) -> Vec<String> {
    let mut names: Vec<String> = namespaces.iter().map(|n| n.name.clone()).collect();
    names.sort();
    names
}

/// The names in `list`, `,` between them, as names and globs with `*` only, that fit the
/// namespaces of the project. A name that is none of them is refused; a glob may match none.
fn select(namespaces: &[Namespace], list: &str, origin: &str) -> Result<Vec<String>, Error> {
    let bad = |why: String| Error::BadArgument(format!("{origin} `{list}`: {why}"));
    let mut chosen = BTreeSet::new();
    for item in list.split(',') {
        if item.contains("::") {
            return Err(bad(format!(
                "`{item}` names an imported project, which is not read yet"
            )));
        }
        let names_only = item
            .split('*')
            .all(|part| part.is_empty() || plain_name(part));
        if item.is_empty() || !names_only || item.contains("**") {
            return Err(bad(format!(
                "`{item}` is not a namespace name or a glob: a name uses ASCII letters, digits, `-` and `_`, and a glob only `*`"
            )));
        }
        let glob = Segment::parse_glob(item).map_err(&bad)?;
        let fitting: Vec<&Namespace> = namespaces
            .iter()
            .filter(|space| glob.matches(&space.name))
            .collect();
        if fitting.is_empty() && !item.contains('*') {
            let known = names(namespaces).join(", ");
            return Err(bad(format!(
                "`{item}` is not a namespace of this project, which has: {known}"
            )));
        }
        chosen.extend(fitting.into_iter().map(|space| space.name.clone()));
    }
    Ok(chosen.into_iter().collect())
}

/// The namespace whose folder holds the current directory, or the current directory itself.
fn inside(namespaces: &[Namespace], root: &Path, env: &dyn Env) -> Option<String> {
    let cwd = canonical(&env.current_dir().ok()?);
    namespaces
        .iter()
        .filter(|space| !space.folder.is_empty())
        .find(|space| cwd.starts_with(canonical(&root.join(&space.folder))))
        .map(|space| space.name.clone())
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}
