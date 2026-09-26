//! The namespaces a command reads, chosen in the order SPC-7 gives.

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
    /// The namespaces of this project in scope, sorted by name.
    pub namespaces: Vec<String>,
    /// The imports in scope, each with its chosen namespaces, sorted by alias. Only `--namespace`
    /// and `TYPDOC_NAMESPACE` can name one (`'chief::*'`); every other source leaves this empty.
    pub imports: Vec<(String, Vec<String>)>,
}

impl Scope {
    pub fn contains(&self, namespace: &str) -> bool {
        self.namespaces.iter().any(|name| name == namespace)
    }
}

/// An import `--namespace`/`TYPDOC_NAMESPACE` can name, with `None` for one absent on this
/// machine. Naming an absent import is refused rather than giving no documents: the request is
/// explicit, unlike a ref, which `imports.absent` reports as a warning by default (SPC-14).
pub(crate) struct ImportListing<'a> {
    pub alias: &'a str,
    pub namespaces: Option<&'a [String]>,
}

/// `prefix` is the namespace an argument names, and `flag` the value of `--namespace`. `imports`
/// is read only for `--namespace` and `TYPDOC_NAMESPACE`: an import prefix on an argument is
/// resolved against that import directly, never through a `Scope`.
pub(crate) fn choose(
    namespaces: &[Namespace],
    root: &Path,
    prefix: Option<&str>,
    flag: Option<&str>,
    env: &dyn Env,
    imports: &[ImportListing],
) -> Result<Scope, Error> {
    if let Some(prefix) = prefix {
        return Ok(Scope {
            source: Source::Prefix,
            namespaces: select(namespaces, prefix, "a prefix", &[])?.0,
            imports: Vec::new(),
        });
    }
    if let Some(list) = flag {
        let (own, other) = select(namespaces, list, "--namespace", imports)?;
        return Ok(Scope {
            source: Source::Flag,
            namespaces: own,
            imports: other,
        });
    }
    if let Some(list) = env.var("TYPDOC_NAMESPACE").filter(|v| !v.is_empty()) {
        let list = list.into_string().map_err(|value| {
            Error::BadArgument(format!("TYPDOC_NAMESPACE is not valid UTF-8: {value:?}"))
        })?;
        let (own, other) = select(namespaces, &list, "TYPDOC_NAMESPACE", imports)?;
        return Ok(Scope {
            source: Source::Variable,
            namespaces: own,
            imports: other,
        });
    }
    if let Some(inside) = inside(namespaces, root, env) {
        return Ok(Scope {
            source: Source::CurrentDirectory,
            namespaces: vec![inside],
            imports: Vec::new(),
        });
    }
    Ok(Scope {
        source: Source::Everything,
        namespaces: names(namespaces),
        imports: Vec::new(),
    })
}

fn names(namespaces: &[Namespace]) -> Vec<String> {
    let mut names: Vec<String> = namespaces.iter().map(|n| n.name.clone()).collect();
    names.sort();
    names
}

/// `select`'s result: this project's own namespaces chosen, and, for each import chosen
/// (`alias::pattern`), its alias with the namespace names chosen inside it.
type Selected = (Vec<String>, Vec<(String, Vec<String>)>);

/// The namespaces and the imports (`alias::pattern`, patterns merged per alias) that `list`
/// names. The `prefix` origin passes no `imports`, since an argument's import prefix never
/// reaches a `Scope`.
fn select(
    namespaces: &[Namespace],
    list: &str,
    origin: &str,
    imports: &[ImportListing],
) -> Result<Selected, Error> {
    let bad = |why: String| Error::BadArgument(format!("{origin} `{list}`: {why}"));
    let mut chosen = BTreeSet::new();
    let mut chosen_imports: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for item in list.split(',') {
        if let Some((alias, pattern)) = item.split_once("::") {
            let Some(listing) = imports.iter().find(|listing| listing.alias == alias) else {
                let known: Vec<&str> = imports.iter().map(|listing| listing.alias).collect();
                return Err(bad(crate::imports::unknown_alias_message(alias, &known)));
            };
            let Some(import_namespaces) = listing.namespaces else {
                return Err(bad(format!(
                    "the import `{alias}` is absent on this machine"
                )));
            };
            let matched = glob_match(pattern, import_namespaces, &bad)?;
            chosen_imports
                .entry(alias.to_owned())
                .or_default()
                .extend(matched);
            continue;
        }
        let fitting = glob_match(item, &names(namespaces), &bad)?;
        chosen.extend(fitting);
    }
    let imports = chosen_imports
        .into_iter()
        .map(|(alias, names)| (alias, names.into_iter().collect()))
        .collect();
    Ok((chosen.into_iter().collect(), imports))
}

fn glob_match(
    item: &str,
    available: &[String],
    bad: &dyn Fn(String) -> Error,
) -> Result<Vec<String>, Error> {
    let names_only = item
        .split('*')
        .all(|part| part.is_empty() || plain_name(part));
    if item.is_empty() || !names_only || item.contains("**") {
        return Err(bad(format!(
            "`{item}` is not a namespace name or a glob: a name uses ASCII letters, digits, `-` and `_`, and a glob only `*`"
        )));
    }
    let glob = Segment::parse_glob(item).map_err(bad)?;
    let fitting: Vec<String> = available
        .iter()
        .filter(|name| glob.matches(name))
        .cloned()
        .collect();
    if fitting.is_empty() && !item.contains('*') {
        return Err(bad(format!(
            "`{item}` is not a namespace of this project, which has: {}",
            available.join(", ")
        )));
    }
    Ok(fitting)
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
