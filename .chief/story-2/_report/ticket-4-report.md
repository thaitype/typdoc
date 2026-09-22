# Ticket 4: signals

Resolved. Commit `f664927`, merged `1ba25bc` with no conflicts. `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean, all re-run
on the merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 948 passed / 0 failed /
1 ignored (938 before this ticket). The four signal tests were also run directly and watched pass on
their own, since this ticket sends real OS signals to a real spawned process.

## Outcome

done

## What it does

`signal-hook` catches `SIGINT` and `SIGTERM`, registered first thing in `main`, before the first
lock file can exist. The handler only wakes a thread; the thread runs the cleanup, then re-raises
the signal through `low_level::emulate_default_handler`, so the process ends by the signal and not
with an exit code of its own. A second signal is queued by `signal_hook` rather than reaching a
second, concurrent iteration, so it cannot cut the cleanup short.

The cleanup needs `NamespaceLock`'s own identity check, but `NamespaceLock` is not `Send` — it
holds `&dyn Fs` and a boxed handle with no bound — so it cannot cross the thread boundary itself.
Resolved by ticket 3's option (b): a small process-wide registry (`REGISTRY`), holding each open
lock's path and the `FileId` its handle had at creation, populated right after the file is created
and deregistered by `NamespaceLock`'s own `Drop` however its life ends. The cleanup thread walks the
registry and applies the same identity check `release` already does, extracted into one shared
function so both paths run the identical logic.

## The hard part, solved with no test-only code

Nothing shipping gained a flag, a sleep, or a hidden mode. `new`'s own real validation
(`Project::prescan_refs`, part of checking `refs.acyclic`) already reads every document in the
namespace from disk, unconditionally, under the lock, before the document is written — cost that
scales with document count and was there before this ticket. A scratch project with a large number
of filler documents makes that scan take long enough (measured: ~2.7s at 30,000 documents) for a
test to reliably poll for the lock file, catch the run a few dozen milliseconds into the scan, and
signal it — well before the scan would finish on its own. This is the mechanism tickets 14 and 15
are expected to reuse: a `Scratch` project with coded filler documents, a write command run through
the spawn helper's new `Spawn::spawn()`, polling for `.typdoc/locks/<namespace>.lock`.

## Checked by running

- A real `SIGINT` and a real `SIGTERM`, each sent to the shipped binary while it holds a lock: the
  lock file is gone afterward, and the caller sees `ended.code == None`, `ended.signal ==
  Some(SIGINT|SIGTERM)` — a process ended by the signal, not an exit code from the table.
- Two signals sent back-to-back do not cut the cleanup short: the lock is still gone and the process
  still ends by a signal.
- A run meeting a lock file nobody owns stops at exit 4 with the path named — the window inside the
  creating call is not tested as closed, because it is not.
- Eight things were shown able to fail, one at a time, each restored: the write ban reaching the new
  code (library and test); `Drop` deregistering even when `release` is never called; the identity
  check in the signal path; the handler's own wiring in `main` (three of four signal tests failed
  exactly as expected, the exit-4 test correctly staying green since it does not depend on the
  handler); the exit-4 mapping itself; and `UNPRODUCED_EXIT_CODES`'s own consumer test, by putting 4
  back on the list while it is now produced.

## Decision

- **Issue:** ticket 3 left `NamespaceLock`'s missing `Send` bound as an open choice between adding
  `Send`/`Sync` to `Fs`/`WriteHandle`, or a small registry the cleanup thread reads instead.
- **Chosen:** the registry. Adding the bounds would ripple through every implementor of `Fs` for a
  benefit only the signal path needs; the registry caches what the identity check actually needs
  (device and inode are immutable for the life of an open handle) and reads the live `stat` through
  `Fs`, which the cleanup thread can already reach on its own.

## Notes

- `Exit 6` for a startup failure of `signals::install()` itself is marked `Not verified` in the
  code's own comment: nothing in this environment can force `Signals::new` to fail, so the mapping
  is the closest table entry rather than a checked one.
- A narrow, pre-existing leak, not introduced here and not this ticket's scope: if `acquire`'s
  `handle.write_all` fails right after the file is created and the registry entry added, the
  function returns before a `NamespaceLock` value exists, so neither `Drop` nor `release` ever runs
  and the registry entry leaks alongside the file for the rest of the process.
