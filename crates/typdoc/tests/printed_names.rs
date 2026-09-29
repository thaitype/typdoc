//! Covers SPC-2, SPC-12 and SPC-18: every document of every project in the fixtures reads back as
//! itself from both names it is printed with, its `path` and its portable name, `ref`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::{Path, PathBuf};

use common::Spawn;
use serde_json::Value;

/// Projects in `fixtures/valid` that only load as an import of another.
const NOT_LOADED_ALONE: [&str; 2] = ["imports/memory", "imports/memory/nowhere-nested"];

fn projects() -> Vec<PathBuf> {
    let valid = typdoc_testkit::fixtures::path("valid");
    let mut found = Vec::new();
    let mut stack = vec![valid.clone()];
    while let Some(dir) = stack.pop() {
        if dir.join(".typdoc/config.json").is_file() {
            let relative = dir.strip_prefix(&valid).expect("under fixtures/valid");
            if !NOT_LOADED_ALONE
                .iter()
                .any(|known| relative == Path::new(known))
            {
                found.push(dir.clone());
            }
        }
        for entry in std::fs::read_dir(&dir).expect("a readable fixture folder") {
            let path = entry.expect("a readable entry").path();
            if path.is_dir() && path.file_name().is_some_and(|name| name != ".typdoc") {
                stack.push(path);
            }
        }
    }
    found.sort();
    found
}

fn path_of(project: &Path, name: &str) -> Result<String, String> {
    let ran = Spawn::args(["get", name, "--json"]).cwd(project).run();
    if ran.code != 0 {
        return Err(format!("exit {}: {}", ran.code, ran.stderr));
    }
    Ok(ran.stdout_json()["document"]["path"]
        .as_str()
        .expect("a path")
        .to_owned())
}

#[test]
fn every_document_reads_back_from_its_path_and_from_its_ref() {
    let mut problems = Vec::new();
    let mut checked = 0;
    for project in projects() {
        let listed = Spawn::args(["list", "--json"]).cwd(&project).run();
        if listed.code != 0 {
            problems.push(format!(
                "{}: list failed: {}",
                project.display(),
                listed.stderr
            ));
            continue;
        }
        let documents = listed.stdout_json()["documents"].clone();
        for document in documents.as_array().expect("an array") {
            let path = document["path"].as_str().expect("a path");
            let mut names = vec![path];
            if let Some(Value::String(portable)) = document.get("ref") {
                names.push(portable);
            }
            for name in names {
                match path_of(&project, name) {
                    Ok(found) if found == path => checked += 1,
                    Ok(found) => problems.push(format!(
                        "{}: `{name}` read as {found}, not {path}",
                        project.display()
                    )),
                    Err(error) => {
                        problems.push(format!("{}: `{name}` ({path}): {error}", project.display()))
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(checked > 40, "only {checked} names were read back");
}
