# 7: Which platforms does v1 support, and how does the lock behave on each?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The lock is created with `O_EXCL`, records pid, hostname and timestamp, and is taken over if its pid is dead on the same host or it is older than 30 seconds. Checking that a pid is dead is easy on Unix and different on Windows. Config also reads `~/.config/typdoc/imports.json` and expands `${ENV}` in import paths.

Decide the supported platforms for v1 (Linux and macOS only, or Windows too), what that means for pid-liveness, path separators in keys and refs, and the config location, and how an unsupported platform fails. Consider that the 30-second stale threshold can steal a lock from a live process on a slow write. Also from ticket 5: the query language escapes with `\` and the docs recommend single quotes around `--where` expressions; `\` is also a shell escape in double quotes, and PowerShell and cmd quote differently, so the supported shells matter to how that advice is worded. Amend `docs/design.md`.

## Answer

Decided 2026-09-20. `docs/design.md` is amended in Concurrency, the `.typdoc` folder and `lock.json` text, Machine-specific imports, the `imports.absent` row, and a new paragraph, Quoting in the shell.

Ticket 13 changed the lock layout: there is one lock file per namespace at `.typdoc/locks/<namespace>.lock` (`git-common`: `<project-hash>-<namespace>.lock`), and `mv --renumber` holds two namespaces' locks in name order.

### Platforms

Decided: Linux and macOS only in v1. On Windows the build fails with a clear message (`compile_error!`); WSL is Linux and works. Keys and refs always use `/`, so a repository can be shared across operating systems later. macOS is claimed only when a real macOS CI run checks it (ticket 9).

### Stale locks

Decided: v1 never deletes or takes over a lock that another process created.

1. A lock is an `O_EXCL` file. There is no takeover by age or by pid. After `--lock-timeout` (default 5 s) the command exits 4. The 30-second rule is gone and no other number replaces it. v1 supports locking on one machine.
2. Exit 4 shows the path, pid, host, age and the owner's status. Running on this machine: wait, and no way to remove it is shown. Not running on this machine: the lock is stale and the path is shown. Other host: it cannot be checked. The pid check only chooses the wording; a hostname is taken to mean one set of processes, and a container that reuses the host's hostname makes the status unreliable (documented).
3. The document states the removal rule: one remover at a time per namespace, because two removers acting on earlier information can delete the new lock of a writer that has just acquired it. Who removes belongs to the workflow.
4. typdoc removes its own lock on normal completion, on error, and on the system's interrupt signals (on POSIX SIGINT and SIGTERM). It keeps the file open from creation to removal so the inode cannot be reused. Before removing it compares the file identity the system gives (on POSIX `stat` on the path against `fstat` on the descriptor, device and inode), never the contents. On a mismatch, or a link count of zero, it removes nothing and reports that the lock was removed by someone else. The gap between the check and the removal stays open; it matters only when a live lock is removed, which rule 3 forbids.
5. The document says plainly that v1 does not prevent another party from removing a lock we hold; it makes the affected writer notice and report. Not showing the removal method while the owner is running is a way of not inviting it, not a mechanism.
6. Network fetches happen before any lock. After taking the lock, `lock.json` is re-read: a pin that now exists is used, and if its bytes differ from what was fetched the difference is reported, never overwritten silently.
7. The lock covers reading, checking and the rename only: no network and no waiting for input, which gives the five-second timeout its reason. `mv` and `mv --renumber` are the only commands whose hold time grows with the size of the repository; a large repository may need a longer `--lock-timeout`.

Rejected: taking over a lock whose pid is dead on this machine, and after five minutes for another host. It leaves two weaknesses: a reused pid keeps a dead lock looking alive, and a macOS hostname that changes with the network makes this machine's lock look foreign. Never taking over removes both, since pid and host only choose the wording of a message.

Not done, on purpose: pid plus process start time, a random token read back, an unlock command, an unlock file. All of them serve removal or takeover by the program, which v1 does not have.

Observations: two processes that read the same exit-4 message may both remove the lock, and typdoc cannot stop that, so a workflow that removes stale locks needs a convention naming one remover. Catching interrupt signals needs a signal-handling crate, chosen in the contract. A forced kill (SIGKILL on POSIX) and power loss always leave a stale lock.

### Pins

Decided: `lock.json` and `vendor/` belong to the project, so they get a project lock, `locks/.project.lock` (its name starts with `.`, so no namespace can have it), with the same rules as any lock. A command never takes it while holding a namespace lock; when it needs both it takes the project lock first, then the namespace locks in name order, so two commands cannot deadlock. Network fetches happen before any lock; after taking the project lock the command re-reads `lock.json`, uses a pin that now exists, and reports, never overwrites, when the bytes differ.

Also decided:

- `lock.json` and the files in `vendor/` are written with a temp file plus rename, like every file, so a reader sees the old file or the new one whole. This is written into the existing Atomic writes bullet, not as a second rule, because one rule stated in two places drifts apart.
- v1 never deletes anything in `vendor/`. This is a scope cut, not a condition of correctness: deletion could only add a loud error in a rare case, and clearing files nothing refers to is a separate job. The folder only grows, on purpose, and the document says so, so that someone who finds it large does not read it as a bug. A missing pinned copy is a config error that says to run `typdoc pull`, which fetches again and compares with the pinned SHA-256: equal restores the copy, different is reported.
- `path` is removed from `lock.json`. The copy is always at `vendor/<section>/<sha256>` (not hard-coded to schemas, since `lock.json` is split into sections for other kinds of pin). A copy counts as edited by hand when the hash of its contents differs from its file name. A file in `vendor/` then checks itself without opening `lock.json`, and there is one value where two could disagree.

Default: the copy has no extension (the rule is vendor, section, sha256), so editors do not highlight it. When `pull` after a missing copy fetches different bytes, that is an update of the pin, reported as a changed URL, which is what `pull` already does (refusing to move the pin would make repairing a missing file and updating a pin two different commands). The `git-common` project lock path is `<project-hash>.lock`.

### Machine file `imports.json`

Decided: the location is found as a search order, so that a later Windows port fills in a line instead of editing the rule. Windows stays out of v1; nothing already written is redone.

1. The order, stopping at the first step that applies: `TYPDOC_CONFIG_DIR` if set; `XDG_CONFIG_HOME` if set and absolute; the platform default (in v1, `~/.config/typdoc/imports.json` on Linux and macOS). A later platform is one line at step 3, and the document says so for whoever adds it. `TYPDOC_CONFIG_DIR` is not only for the future: tests and containers with an odd `HOME` need a way to move the file, and borrowing `XDG_CONFIG_HOME` mixes two intents, a machine-wide setup and forcing this program. It completes the set with `TYPDOC_DIR` and `TYPDOC_NAMESPACE` from ticket 13.
2. `XDG_CONFIG_HOME` follows its own specification: unset, empty or relative counts as not set and falls to step 3.
3. No test reads the real home directory of whoever runs it, and every step is exercised, with a fake `HOME` for step 3. Otherwise a test can pass on a developer machine because of a file that is really there, which is green because nothing ran, not because nothing is wrong.
4. An error names the path searched and which of the three steps it came from.
5. A missing file is not an error: there are no machine-specific imports. What that leaves absent is reported by the existing `imports.absent` (`warn`); there is no second mechanism.
6. Two places are worded as ideas rather than POSIX names, so Windows can be added without contradicting the text: the same-file check in the lock release (file identity from the system; on POSIX device and inode, on Windows the volume serial and file index), and cleanup on interruption (the system's interrupt signals; on POSIX SIGINT and SIGTERM). The shell-quoting advice is tied to shells, not operating systems.

Default: `TYPDOC_CONFIG_DIR` is the directory that holds `imports.json`, while steps 2 and 3 add `typdoc/` (they are shared system directories). `TYPDOC_CONFIG_DIR` must be an absolute path to a directory that exists; otherwise it is an error, since it was set on purpose.

### Unset `${ENV}` in an import path

Decided: an unset variable is never replaced by an empty string. The import is treated as absent on this machine and reported by `imports.absent` (default `warn`) with a message that names the variable, which is different from "path does not exist". Other imports and other namespaces load normally. It is written into Machine-specific imports (a new paragraph) and the `imports.absent` row.

Why: an empty string would turn `${HOME}/projects` into `/projects`, a path that may exist and be the wrong project, and the result would look like a correct answer. And per-machine imports exist so that no machine-specific path is committed. If one missing import stopped the whole project loading (a config error), the easiest escape would be to commit the path, which is what the feature exists to prevent; a config error cannot be turned off by configuration, so that pressure has nowhere to go. A rule that pushes people back to what it was built to prevent is worse than a warning. What remains is only warn versus error, no longer a mechanism.

Also decided:

1. The typo risk (a mistyped variable name is only a warning) is closed by turning advice into a requirement. The document says that a project that needs its imports to exist sets `imports.absent` to `error` in CI, and says it where `imports.absent` is described, not only in the paragraph about variables: the warning belongs where the hand is about to act.
2. A variable set to an empty value counts as unset, the same as `XDG_CONFIG_HOME` above.
3. One rule serves the `imports` in `config.json` and in `imports.json`, used in both places.

Observation: `imports.absent` fires on refs into an absent import, so an import that is unresolved but that nothing refers to fails nothing, even at `error`. That is taken as intended (unused, so harmless); the contract should confirm it.

### Quoting in the shell

Decided: the advice is written per shell, in the new paragraph Quoting in the shell (Query section), linked from the escaping bullet, and covers `--namespace` as well as `--where`. The examples include `--namespace '*'`, `--namespace 'chief::*'` and `TYPDOC_NAMESPACE='*'`.

1. The list contains only shells that have actually been run. A row is a claim; a row for a shell that was never run describes an intention that a reader takes for a state. It cannot simply be dropped, because zsh is macOS's default shell and macOS is supported. Quoting is behaviour of the shell, not of the operating system, so installing zsh on a Linux machine and running the test is full evidence for zsh on macOS, unlike the claim about macOS itself, which needs a macOS runner. So the v1 rows are sh, bash and zsh, each backed by a test that runs through the real shell; if zsh cannot be installed the row is removed.
2. The behaviour of an untested shell is not described. fish's handling of a single-quoted `\\` comes from its documentation and was never run, so it stays out of the document. The text says only that fish, PowerShell and cmd are not covered in v1.
3. One line of practical advice for a shell outside the list: avoid values containing a backslash, or run a query whose answer is already known first, because the dangerous symptom is not an error but a changed expression that still parses and gives a plausible wrong answer.

Not verified: run on this machine were bash and dash (as `sh`), where single quotes kept `\*`, `\,` and `\\` intact. Not run: zsh (not installed), fish, PowerShell and cmd. The zsh row therefore has no evidence yet; the design states a rule (a shell is listed only while a test runs it), and the contract or ticket 9 must run it.

### Tests for ticket 9

- A lock held for more than thirty seconds is not taken.
- Two processes racing for a lock get one winner; the other exits 4.
- A process interrupted by SIGINT leaves no lock.
- When our lock is removed and another writer creates a new one during our work, we do not remove theirs at the end.
- While `pull` writes `lock.json`, a concurrent reader always gets a file that parses completely, never half of one.
- A file in `vendor/` whose hash differs from its name is reported as a config error.
- A missing pinned copy is a config error that says to run `typdoc pull`.
- `TYPDOC_CONFIG_DIR` beats a simultaneously set `XDG_CONFIG_HOME`; `XDG_CONFIG_HOME` alone gives that path, not step 3; with neither set and a fake `HOME`, step 3 is used; an empty and a relative `XDG_CONFIG_HOME` both fall to step 3; no test reads the real home.
- An unset variable in an import path never produces a path with an empty replacement, and does produce a report that names the variable; a variable set to an empty value behaves as unset; a missing import does not stop other imports or other namespaces from loading; with `imports.absent` set to `error` and the variable unset, `validate` exits with the validation-failed code (2), not a warning.
- Every example printed in the document goes through every shell in the list and typdoc receives the expression byte for byte identical, the `--namespace '*'` examples included. A case with the quotes left out on purpose must turn the test red, so the suite is shown to be able to fail. The examples in the tests and the examples in the document are one list; kept in two places they would drift apart with no signal.

### Summary

Linux and macOS only, and the Windows build fails clearly. No automatic takeover of any lock; exit 4 explains, one remover at a time, lock removal checked by file identity, network before locks, held only for read, check and rename. A project lock for pins, pins written by temp file plus rename, `path` removed from `lock.json`, and `vendor/` only grows. The machine file is found by a three-step order with `TYPDOC_CONFIG_DIR` first, and no test reads the real home. An unset or empty `${ENV}` is never an empty string: the import is absent and reported by `imports.absent`, which CI sets to `error` when imports are required. Quoting advice is per shell, with rows only for shells that were run, `--namespace` included.
