//! Covers SPC-7: `version: 1` keeps its meaning, so a project made for an earlier release still
//! validates.
//!
//! Every project under `fixtures/compat/<version>/` is a copy taken unchanged from a release
//! (`fixtures/compat/README.md` names the sources). None may fail `validate`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::PathBuf;

use common::Spawn;
use typdoc_testkit::fixtures;

fn compat_projects() -> Vec<PathBuf> {
    let root = fixtures::root().join("fixtures/compat");
    let mut found = Vec::new();
    for version in std::fs::read_dir(&root).expect("fixtures/compat exists") {
        let version = version.expect("a readable entry").path();
        if !version.is_dir() {
            continue;
        }
        for project in std::fs::read_dir(&version).expect("a readable version folder") {
            let project = project.expect("a readable entry").path();
            if project.join(".typdoc/config.json").is_file() {
                found.push(project);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_compat_folder_holds_the_projects_its_readme_lists() {
    let names: Vec<String> = compat_projects()
        .iter()
        .map(|p| {
            let version = p.parent().unwrap().file_name().unwrap().to_string_lossy();
            format!("{version}/{}", p.file_name().unwrap().to_string_lossy())
        })
        .collect();
    assert_eq!(
        names,
        ["0.1.0/examples", "0.3.1/chief-example", "0.3.1/examples",]
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
