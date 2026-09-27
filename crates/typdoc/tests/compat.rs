//! Covers SPC-7: `version: 1` keeps its meaning, so a project made for an earlier release still
//! validates.
//!
//! Every project under `fixtures/compat/<version>/` stands for what a project made for that
//! release could hold (`fixtures/compat/README.md` says where each came from). None may fail
//! `validate`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::{Path, PathBuf};

use common::Spawn;
use typdoc_testkit::fixtures;

fn compat_projects() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        if dir.join(".typdoc/config.json").is_file() {
            found.push(dir.to_path_buf());
            return;
        }
        for entry in std::fs::read_dir(dir).expect("a readable folder") {
            let path = entry.expect("a readable entry").path();
            if path.is_dir() {
                walk(&path, found);
            }
        }
    }
    let mut found = Vec::new();
    walk(&compat_root(), &mut found);
    found.sort();
    found
}

fn compat_root() -> PathBuf {
    fixtures::root().join("fixtures/compat")
}

#[test]
fn the_compat_folder_holds_the_projects_its_readme_lists() {
    let root = compat_root();
    let names: Vec<String> = compat_projects()
        .iter()
        .map(|p| {
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    assert_eq!(
        names,
        [
            "0.1.0/examples",
            "0.1.0/stories",
            "0.1.0/tickets-notes",
            "0.1.0/with-import/app",
            "0.1.0/with-import/memory",
            "0.3.0/stories-excluded",
            "0.3.1/chief-example",
            "0.3.1/examples",
        ]
    );
}

#[test]
fn every_project_from_an_earlier_release_still_validates() {
    let mut failed = Vec::new();
    for project in compat_projects() {
        let ran = Spawn::args(["validate"]).cwd(&project).run();
        if ran.code != 0 {
            failed.push(format!(
                "{} exited {}\n{}{}",
                project.display(),
                ran.code,
                ran.stdout,
                ran.stderr
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
