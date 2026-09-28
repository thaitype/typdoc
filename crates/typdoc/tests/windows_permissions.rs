//! Covers SPC-10 and the write seam on Windows: a replaced document keeps its DACL and its
//! read-only flag, a DACL that cannot be carried stops the write, and a document another program
//! holds open is not replaced.
//!
//! Windows only: DACLs and the read-only flag are Windows' permissions; `mv.rs` covers the Unix
//! mode.

#![cfg(windows)]

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::os::windows::fs::OpenOptionsExt;

use common::{Ran, Scratch, Spawn};
use typdoc_testkit::acl::{sddl, set_sddl};

/// One collection of `*.md` whose schema has a `title`.
const TITLED: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
    ),
];

/// Administrators and SYSTEM may do anything, everyone else only read, and nothing is inherited:
/// a DACL no file in a temporary folder gets by itself.
const RESTRICTIVE: &str = "D:PAI(A;;FA;;;BA)(A;;FA;;;SY)(A;;FR;;;WD)";

fn set_title(project: &Scratch, title: &str) -> Ran {
    Spawn::args(["set", "a.md", &format!("title={title}")])
        .cwd(project.path())
        .run()
}

#[test]
fn a_set_keeps_the_dacl_the_document_had() {
    let project = Scratch::project(&TITLED);
    project.file("a.md", "---\ntitle: A\n---\n");
    let document = project.path().join("a.md");
    set_sddl(&document, RESTRICTIVE).unwrap();

    let ran = set_title(&project, "B");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(project.read("a.md"), "---\ntitle: B\n---\n");
    assert_eq!(sddl(&document).unwrap(), RESTRICTIVE);
}

#[test]
fn a_set_keeps_a_read_only_document_read_only_and_writes_it() {
    let project = Scratch::project(&TITLED);
    project.file("a.md", "---\ntitle: A\n---\n");
    let document = project.path().join("a.md");
    set_sddl(&document, RESTRICTIVE).unwrap();
    let mut permissions = std::fs::metadata(&document).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&document, permissions).unwrap();

    let ran = set_title(&project, "B");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(project.read("a.md"), "---\ntitle: B\n---\n");
    assert!(
        std::fs::metadata(&document)
            .unwrap()
            .permissions()
            .readonly()
    );
    assert_eq!(sddl(&document).unwrap(), RESTRICTIVE);
}

/// The owner is denied reading the DACL, so it cannot be carried: nothing is replaced. The bytes
/// are read through a handle that asks only to read data, which that DACL still allows; setting
/// the DACL back would need the right it denies.
#[test]
fn a_dacl_that_cannot_be_read_stops_the_write_and_the_document_is_not_replaced() {
    let project = Scratch::project(&TITLED);
    project.file("a.md", "---\ntitle: A\n---\n");
    let document = project.path().join("a.md");
    set_sddl(&document, "D:P(D;;RC;;;OW)(A;;FA;;;WD)").unwrap();

    let ran = set_title(&project, "B");

    assert_eq!(ran.code, 6, "{}", ran.stderr);
    assert!(
        ran.stderr.contains(
            "its permissions could not be carried to the replacement, so it was not replaced"
        ),
        "{}",
        ran.stderr
    );
    let mut bytes = String::new();
    std::fs::OpenOptions::new()
        .access_mode(0x1) // FILE_READ_DATA, without READ_CONTROL
        .open(&document)
        .and_then(|mut file| std::io::Read::read_to_string(&mut file, &mut bytes))
        .unwrap();
    assert_eq!(bytes, "---\ntitle: A\n---\n");
}

#[test]
fn a_document_another_program_holds_open_without_share_delete_is_not_replaced() {
    let project = Scratch::project(&TITLED);
    project.file("a.md", "---\ntitle: A\n---\n");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1 | 0x2) // read and write, not delete
        .open(project.path().join("a.md"))
        .unwrap();

    let ran = set_title(&project, "B");
    drop(held);

    assert_eq!(ran.code, 6, "{}", ran.stderr);
    assert!(
        ran.stderr.contains(
            "another program has this file open and does not let it be replaced; close it there \
             and run the command again"
        ),
        "{}",
        ran.stderr
    );
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
}
