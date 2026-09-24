//! The goal's second criterion, real-process half (contract, testing decision 2, "Across real
//! processes"): a shipped binary holds a namespace's lock — ticket 4's own mechanism, reused
//! rather than reinvented (`crates/typdoc/tests/signals.rs`'s `large_project`, now shared through
//! `crates/typdoc/tests/common/mod.rs`) — while several independent `new` processes are started
//! against the very same lock, with a `--lock-timeout` long enough that none of them gives up.
//! Running them one after another and finding no duplicate key would prove nothing (it is the
//! same result a build with no lock at all gives whenever the runs happen not to overlap), so
//! this test forces the overlap: the holder is not let go until every contender has been
//! observed still alive and still turned away, not merely started.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::time::{Duration, Instant};

use common::{LOCK_APPEARS_WITHIN, RunningChild, Spawn, large_project, lock_path, wait_for_file};

/// Large enough that the holder's own `Project::prescan_refs` scan (see `common::large_project`'s
/// own doc comment) reliably takes a stretch of real wall-clock time next to the setup and
/// polling this test does around it — the same size ticket 4 measured in the low seconds.
const DOCUMENT_COUNT: u32 = 20_000;

/// How many independent `new` processes race for the one lock the holder is standing on.
const CONTENDER_COUNT: u32 = 3;

/// Long enough that no contender's own `acquire` gives up before the holder's scan does, on any
/// machine this suite runs on; `new`'s own default is five seconds, and the holder's scan alone
/// can take a few of those.
const CONTENDER_LOCK_TIMEOUT_SECS: u64 = 120;

/// How long this test waits to observe every contender turned away and still running before
/// giving up and failing outright — generous next to how quickly that state is normally reached
/// (well before the holder's own scan, which takes seconds, finishes), so a failure to observe
/// it within this bound is treated as a real fault, not retried or silently accepted.
const WAITING_OBSERVED_WITHIN: Duration = Duration::from_secs(60);

/// The lock file's own recorded owner, read the same way `crates/typdoc-core/src/
/// namespace_lock.rs`'s `Owner` does, without depending on that private type.
fn lock_owner_pid(lock: &std::path::Path) -> Option<u32> {
    let bytes = std::fs::read(lock).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value["pid"].as_u64().map(|pid| pid as u32)
}

/// Polls until the lock file at `lock` names `pid` as its owner, or panics if `LOCK_APPEARS_WITHIN`
/// (already generous next to how quickly a real write finishes) passes without that happening.
/// `wait_for_file` alone only proves the file exists, not that the write of its contents — the
/// `pid` field this reads — has finished; reading it back immediately after existence is confirmed
/// can observe an empty or partially written file, which is a race in the observation, not a real
/// fault, so this polls for the field itself rather than accepting `None` as the answer.
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

/// Polls until every contender is confirmed still running (`RunningChild::is_alive`) *and* the
/// lock file still names the holder's own pid (nobody else has taken it over, and the holder has
/// not yet released it) — the combination the ticket's own reasoning gives for "waiting" rather
/// than "started" or "finished": a contender that had succeeded would have taken the lock away
/// from the holder (impossible while the holder still holds it, but checked anyway, since it is
/// exactly the evidence that would show a fault if the reasoning above were wrong), and a
/// contender that had errored out (for instance by giving up on the timeout) would no longer be
/// running. Panics with a clear message if this is never observed within a generous bound,
/// rather than silently proceeding on an unproven assumption.
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

/// **Contract testing decision 2, "Across real processes."** The holder is an *uncoded* `new`
/// (a path target, matching a second collection of the same namespace, so it shares the same
/// lock) rather than one of the `n`: it allocates no key and touches no state file, so it cannot
/// be mistaken for one of the `n` distinct keys this test counts, and its own pid, read back out
/// of the lock file it creates, is never one a contender could legitimately have. It is not
/// released deliberately (there is no signal, and no test-only mode to hold it open): its own
/// scan is what holds the lock, exactly ticket 4's mechanism, and it is simply given enough
/// filler documents that, by the time all `n` contenders have been confirmed turned away and
/// still waiting, it has not finished on its own yet either.
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

    // Nothing releases the holder: its own scan finishes in its own time, exactly as it would
    // for any other run of `new` against a project this size.
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

    // (b) `n` distinct keys.
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

    // `last` equal to the highest of them.
    let state_text = std::fs::read_to_string(project.path().join(".typdoc/state/default.json"))
        .expect("the state file");
    let state: serde_json::Value = serde_json::from_str(&state_text).expect("valid JSON");
    assert_eq!(
        state["tickets"]["last"],
        serde_json::json!(highest),
        "`last` must equal the highest key any contender was actually given"
    );

    // `n` documents on disk, and no document written over another: each contender's own key
    // names a file that exists, carries that contender's own title, and no other contender's.
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
