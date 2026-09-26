//! Where a fixture's command actually runs. A declared read runs in the fixture's own folder.
//! A declared write runs on a copy, made in a temporary
//! folder, so that a write never reaches the repository's own tree. No command a fixture can
//! declare writes yet, but the loader enforces the placement ahead of one existing, so no later
//! ticket has to add the enforcement together with the command.

use std::path::{Path, PathBuf};

use crate::spec::FixtureSpec;

/// Held for its lifetime so the copy of a declared write's fixture is not removed while a run
/// still needs it.
pub struct StagedFixture {
    dir: PathBuf,
    _copy: Option<tempfile::TempDir>,
}

impl StagedFixture {
    /// The folder to run the command in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Stages `dir`'s fixture for a run: unchanged for a declared read, copied to a temporary
/// folder for a declared write. A test that goes through this never has to remember which
/// fixtures write, and a write fixture can never run in the repository's own tree by a test
/// forgetting to copy it, because there is no path to a run that skips this decision.
pub fn stage(dir: &Path, spec: &FixtureSpec) -> Result<StagedFixture, String> {
    stage_command(dir, &spec.command)
}

/// [`stage`], for a caller that has a bare command rather than a loaded `FixtureSpec`, such as
/// `golden::Case`. Whether the command writes is read through `crate::spec::is_write_command`,
/// the same as `FixtureSpec::is_write`, so the two callers cannot disagree.
pub fn stage_command(dir: &Path, command: &[String]) -> Result<StagedFixture, String> {
    if !crate::spec::is_write_command(command) {
        return Ok(StagedFixture {
            dir: dir.to_owned(),
            _copy: None,
        });
    }
    let copy = tempfile::tempdir()
        .map_err(|e| format!("a temporary folder for the copy of {}: {e}", dir.display()))?;
    copy_tree(dir, copy.path())?;
    let fixtures_root = crate::fixtures::root().join("fixtures");
    refuse_if_in_repository(copy.path(), &fixtures_root)?;
    Ok(StagedFixture {
        dir: copy.path().to_owned(),
        _copy: Some(copy),
    })
}

/// The harness's own guard: a declared write must never run inside the repository's own
/// `fixtures/` tree. Firing means the copy did not leave the tree, which is a fault of the
/// loader and not of the fixture.
fn refuse_if_in_repository(run_dir: &Path, fixtures_root: &Path) -> Result<(), String> {
    if run_dir.starts_with(fixtures_root) {
        return Err(format!(
            "a fixture that declares a write would run at {}, inside the repository's own \
             fixtures tree {}: it must run on a copy",
            run_dir.display(),
            fixtures_root.display()
        ));
    }
    Ok(())
}

/// Copies `src`'s tree onto `dst`, which must already exist. A plain file's bytes and
/// permissions are copied; a folder is walked; a symbolic link is recreated as one, never
/// followed, so a fixture is copied exactly as it is checked in.
fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", src.display()))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", from.display()))?;
        if kind.is_dir() {
            std::fs::create_dir(&to).map_err(|e| format!("{}: {e}", to.display()))?;
            copy_tree(&from, &to)?;
        } else if kind.is_symlink() {
            let target =
                std::fs::read_link(&from).map_err(|e| format!("{}: {e}", from.display()))?;
            std::os::unix::fs::symlink(&target, &to)
                .map_err(|e| format!("{}: {e}", to.display()))?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("{} -> {}: {e}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_spec() -> FixtureSpec {
        FixtureSpec::parse(
            "a.b",
            r#"{ "command": ["set", "note.md", "title=Changed"], "trips": ["a.b"] }"#,
        )
        .unwrap()
    }

    #[test]
    fn a_write_pointed_at_the_repository_tree_is_refused_by_the_guard() {
        let fixtures_root = Path::new("/repo/fixtures");
        let run_dir = fixtures_root.join("broken/example");

        let error = refuse_if_in_repository(&run_dir, fixtures_root).unwrap_err();

        assert!(error.contains("fixtures tree"), "{error}");
        assert!(error.contains("/repo/fixtures/broken/example"), "{error}");
    }

    #[test]
    fn a_write_pointed_outside_the_repository_tree_is_allowed_by_the_guard() {
        let fixtures_root = Path::new("/repo/fixtures");
        let run_dir = Path::new("/tmp/some-copy");

        assert_eq!(refuse_if_in_repository(run_dir, fixtures_root), Ok(()));
    }

    // A read never reaches the guard; `a_read_fixture_stages_in_its_own_folder_unchanged`
    // covers reads.

    /// A fixture folder under a temporary directory, standing in for one committed under
    /// `fixtures/broken/`: a document and the `fixture.json` that declares the command.
    fn temp_fixture(command_json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "---\ntitle: Before\n---\n").unwrap();
        std::fs::create_dir(dir.path().join(".typdoc")).unwrap();
        std::fs::write(
            dir.path().join(".typdoc/config.json"),
            r#"{ "version": 1 }"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("fixture.json"),
            format!(r#"{{ "command": {command_json}, "trips": ["a.b"] }}"#),
        )
        .unwrap();
        dir
    }

    #[test]
    fn a_read_fixture_stages_in_its_own_folder_unchanged() {
        let fixture = temp_fixture(r#"["get", "note.md"]"#);
        let spec = FixtureSpec::load(fixture.path(), "a.b").unwrap();

        let staged = stage(fixture.path(), &spec).unwrap();

        assert_eq!(staged.dir(), fixture.path());
    }

    #[test]
    fn a_write_fixture_stages_to_a_fresh_copy_and_a_write_into_it_leaves_the_original_untouched() {
        let fixture = temp_fixture(r#"["set", "note.md", "title=Changed"]"#);
        let spec = FixtureSpec::load(fixture.path(), "a.b").unwrap();
        let original = fixture.path().join("note.md");
        let before = std::fs::read(&original).unwrap();

        let staged = stage(fixture.path(), &spec).unwrap();

        assert_ne!(staged.dir(), fixture.path());
        assert!(
            !staged.dir().starts_with(fixture.path()),
            "the copy must not sit inside the original folder"
        );

        // No write command exists yet to perform this for real, so the effect a `set` would
        // have on the copy is simulated directly: the point under test is which folder holds
        // the change afterward, not what a real `set` would write into it.
        std::fs::write(staged.dir().join("note.md"), "---\ntitle: Changed\n---\n").unwrap();

        assert_eq!(
            std::fs::read(&original).unwrap(),
            before,
            "the fixture's own folder must read exactly as it did before the run"
        );
        assert_eq!(
            std::fs::read(staged.dir().join("note.md")).unwrap(),
            b"---\ntitle: Changed\n---\n"
        );
    }

    #[test]
    fn a_write_fixture_declared_from_a_real_repository_folder_stages_outside_it_and_the_folder_stays_untouched()
     {
        let dir = crate::fixtures::path("valid/minimal");
        let original = dir.join("note.md");
        let before = std::fs::read(&original).unwrap();
        let spec = write_spec();

        let staged = stage(&dir, &spec).unwrap();

        assert_ne!(staged.dir(), dir);
        assert!(!staged.dir().starts_with(&dir));

        std::fs::write(
            staged.dir().join("note.md"),
            "mutated by a simulated write\n",
        )
        .unwrap();

        assert_eq!(
            std::fs::read(&original).unwrap(),
            before,
            "the repository's own fixture file must be unchanged"
        );
    }
}
