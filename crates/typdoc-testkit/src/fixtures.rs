use std::path::{Path, PathBuf};

/// The repository root, found from the manifest folder of a crate under `crates/`.
pub fn locate(manifest_dir: &Path) -> Result<PathBuf, String> {
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("{} is not a folder of a crate", manifest_dir.display()))?
        .to_owned();
    for (what, missing) in [
        ("fixtures folder", root.join("fixtures")),
        // A marker every checkout has, and not a design document: no test reads design prose
        // (SPC-11).
        ("workspace manifest", root.join("Cargo.toml")),
    ] {
        if !missing.exists() {
            return Err(format!(
                "the {what} {} is missing: the tests run from a checkout of the repository",
                missing.display()
            ));
        }
    }
    Ok(root)
}

pub fn root() -> PathBuf {
    match locate(Path::new(env!("CARGO_MANIFEST_DIR"))) {
        Ok(root) => root,
        Err(message) => panic!("{message}"),
    }
}

pub fn path(relative: &str) -> PathBuf {
    let found = root().join("fixtures").join(relative);
    assert!(found.exists(), "the fixture {} is missing", found.display());
    found
}

/// `relative` is a path from the repository root, such as `docs/design/catalog/rules.md`.
pub fn read_catalog<T: serde::de::DeserializeOwned>(relative: &str) -> T {
    let path = root().join(relative);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    typdoc_core::read_json_body(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A checkout with no `fixtures/broken/` has no broken fixtures, and that is red for as long as
/// a rule needs one.
pub fn broken_entries() -> Vec<String> {
    let dir = root().join("fixtures/broken");
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} cannot be read: {e}", dir.display()))
        .map(|entry| {
            let entry = entry.unwrap_or_else(|e| panic!("{} cannot be read: {e}", dir.display()));
            entry
                .file_name()
                .into_string()
                .unwrap_or_else(|name| panic!("{name:?} in {} is not UTF-8", dir.display()))
        })
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_a_checkout_the_loader_says_the_suite_runs_from_a_checkout() {
        let error = locate(Path::new("/no/such/place/crates/typdoc")).unwrap_err();

        assert!(error.contains("checkout of the repository"), "{error}");
        assert!(error.contains("/no/such/place/fixtures"), "{error}");
    }

    #[test]
    fn a_checkout_with_fixtures_and_no_manifest_is_also_refused() {
        let scratch = tempfile::tempdir().unwrap();
        let crate_dir = scratch.path().join("crates/x");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::create_dir_all(scratch.path().join("fixtures")).unwrap();

        let error = locate(&crate_dir).unwrap_err();

        assert!(error.contains("Cargo.toml"), "{error}");
    }

    #[test]
    fn inside_the_checkout_the_root_holds_fixtures_and_the_workspace_manifest() {
        let root = root();

        assert!(root.join("fixtures/valid/minimal/note.md").is_file());
        assert!(root.join("Cargo.toml").is_file());
    }
}
