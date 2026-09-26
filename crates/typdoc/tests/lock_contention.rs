//! Covers SPC-10.
//!
//! A running `new` holds a namespace's lock (`common::large_project`) while several `new`
//! processes contend for it, with a `--lock-timeout` long enough that none gives up. Runs that
//! happen one after another would find no duplicate key even with no lock at all, so the overlap
//! is forced: the holder is not let go until every contender has been seen still running and
//! turned away.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::time::{Duration, Instant};

use common::{LOCK_APPEARS_WITHIN, RunningChild, Spawn, large_project, lock_path, wait_for_file};

/// Large enough that the holder's scan (see `common::large_project`) outlasts this test's setup
/// and polling.
const DOCUMENT_COUNT: u32 = 20_000;

const CONTENDER_COUNT: u32 = 3;

/// Longer than the holder's scan on any machine the suite runs on, which the default of five
/// seconds may not be.
const CONTENDER_LOCK_TIMEOUT_SECS: u64 = 120;

/// Generous next to how soon every contender is normally seen waiting, so not seeing it is a
/// fault, not a reason to retry.
const WAITING_OBSERVED_WITHIN: Duration = Duration::from_secs(60);

/// The lock file's recorded owner, read without the private `Owner` type of
/// `typdoc-core`'s `namespace_lock`.
fn lock_owner_pid(lock: &std::path::Path) -> Option<u32> {
    let bytes = std::fs::read(lock).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value["pid"].as_u64().map(|pid| pid as u32)
}

/// `wait_for_file` shows only that the file exists, not that its contents are written, so this
/// polls for the `pid` itself: an empty or half-written file is a race in the observation, not a
/// fault.
fn wait_until_lock_names_pid(lock: &std::path::Path, pid: u32) {
    let start = Instant::now();
    loop {
        let owner = lock_owner_pid(lock);
        if owner == Some(pid) {
            return;
        }
        assert!(
            start.elapsed() < LOCK_APPEARS_WITHIN,
            "the lock file at {lock:?} never named its own holder's pid ({pid}) within \
             {LOCK_APPEARS_WITHIN:?} (last read: {owner:?})"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waiting means every contender still running while the lock still names the holder. A
/// contender that had taken the lock would have taken it from the holder, which cannot happen and
/// is checked anyway; one that gave up would no longer be running.
fn wait_until_all_contenders_are_observed_waiting(
    lock: &std::path::Path,
    holder_pid: u32,
    contenders: &mut [RunningChild],
) {
    let start = Instant::now();
    loop {
        let lock_still_holders = lock_owner_pid(lock) == Some(holder_pid);
        let all_alive = contenders.iter_mut().all(RunningChild::is_alive);
        if lock_still_holders && all_alive {
            return;
        }
        assert!(
            start.elapsed() < WAITING_OBSERVED_WITHIN,
            "never observed all {} contenders turned away and still waiting on the holder's \
             lock within {WAITING_OBSERVED_WITHIN:?} (lock still names the holder: \
             {lock_still_holders}, all contenders still alive: {all_alive})",
            contenders.len()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The holder is an uncoded `new`, a path in a second collection of the same namespace, so it
/// takes the same lock but allocates no key the test could count. Nothing releases it: its own
/// scan holds the lock until every contender has been seen waiting.
#[test]
fn n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten() {
    let project = large_project(DOCUMENT_COUNT);
    project.file(
        ".typdoc/collections/holder.json",
        r#"{ "match": "holder.md", "schema": "holder.json" }"#,
    );
    project.file("holder.json", r#"{ "name": "holder", "fields": {} }"#);
    let lock = lock_path(project.path());

    let holder = Spawn::args(["new", "holder.md", "--json"])
        .cwd(project.path())
        .spawn();
    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the holder's own lock file never appeared"
    );
    let holder_pid = holder.pid();
    wait_until_lock_names_pid(&lock, holder_pid);

    let titles: Vec<String> = (1..=CONTENDER_COUNT)
        .map(|i| format!("Contender {i}"))
        .collect();
    let mut contenders: Vec<RunningChild> = titles
        .iter()
        .map(|title| {
            Spawn::args([
                "new".to_owned(),
                "WF".to_owned(),
                title.clone(),
                "--set".to_owned(),
                "kind=task".to_owned(),
                "--json".to_owned(),
                "--lock-timeout".to_owned(),
                CONTENDER_LOCK_TIMEOUT_SECS.to_string(),
            ])
            .cwd(project.path())
            .spawn()
        })
        .collect();

    wait_until_all_contenders_are_observed_waiting(&lock, holder_pid, &mut contenders);

    let holder_ended = holder.wait();
    assert_eq!(
        (holder_ended.code, holder_ended.signal),
        (Some(0), None),
        "the holder's own run must succeed (stdout: {:?}, stderr: {:?})",
        holder_ended.stdout,
        holder_ended.stderr
    );

    // Each contender's own key, paired with the title it asked for, so a later check can tell
    // whether the document at that key is genuinely its own.
    let mut allocations: Vec<(String, String)> = Vec::with_capacity(contenders.len());
    for (title, contender) in titles.iter().zip(contenders) {
        let ended = contender.wait();
        assert_eq!(
            (ended.code, ended.signal),
            (Some(0), None),
            "every contender must succeed, none may give up or error (stdout: {:?}, stderr: {:?})",
            ended.stdout,
            ended.stderr
        );
        let value: serde_json::Value =
            serde_json::from_str(&ended.stdout).expect("--json prints one JSON object");
        let key = value["document"]["key"]
            .as_str()
            .expect("a coded allocation always prints a key")
            .to_owned();
        allocations.push((key, title.clone()));
    }

    let keys: Vec<&String> = allocations.iter().map(|(key, _)| key).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    sorted_keys.dedup();
    assert_eq!(
        sorted_keys.len(),
        keys.len(),
        "every contender must have been given a distinct key, got {keys:?}"
    );

    let highest = keys
        .iter()
        .filter_map(|key| key.strip_prefix("WF-"))
        .filter_map(|n| n.parse::<u64>().ok())
        .max()
        .expect("at least one numeric key");

    let state_text = std::fs::read_to_string(project.path().join(".typdoc/state/default.json"))
        .expect("the state file");
    let state: serde_json::Value = serde_json::from_str(&state_text).expect("valid JSON");
    assert_eq!(
        state["tickets"]["last"],
        serde_json::json!(highest),
        "`last` must equal the highest key any contender was actually given"
    );

    for (key, title) in &allocations {
        let path = project.path().join(format!("tickets/{key}.md"));
        assert!(
            path.is_file(),
            "{key} was allocated but no document exists at {path:?}"
        );
        let text = std::fs::read_to_string(&path).expect("the document's own text");
        assert!(
            text.contains(title.as_str()),
            "{key}'s own document must carry the title it was created with ({title:?}): {text}"
        );
        for (other_key, other_title) in &allocations {
            if other_key != key {
                assert!(
                    !text.contains(other_title.as_str()),
                    "{key}'s document must not carry {other_title:?}, another contender's own \
                     title: {text}"
                );
            }
        }
    }
}
