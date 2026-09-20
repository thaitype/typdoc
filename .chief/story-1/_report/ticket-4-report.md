# Ticket 4 Report

## Ticket
`config.json` and collection files in full for a read, namespaces, match templates, the eleven config errors that need no schema, import, state file or pin (each with its id, `details` and `complete`), and the scope of a command.

## Outcome
done

## Decision
Nothing blocked the build. The design and the ticket left these open. Each is a placeholder or a reading that a later ticket may change, with the doubt that remains.

- **Which files a run reads.** This is the part most likely to be revisited.
  - `*` and `**` never match a name that starts with `.`, in files and folders. The design says this for namespace folders only; it is carried over to collection matches. A literal segment that starts with `.` does match. This replaces ticket 1's placeholder, where `*` matched a leading dot.
  - A folder that holds `.typdoc/config.json` is not entered: the design says a match never crosses into a nested project.
  - `**` must be a whole segment and matches zero or more folders; a trailing `**` matches every file below.
  - A symbolic link that a template reaches, including one to a folder under `**`, and a non-UTF-8 name that it reaches, stop the run with exit 6. Unchanged from ticket 1. Exit 6 is probably the wrong code, since a retry never fixes it, and the policy is still not decided.
  - Doubt: `.md` files inside dot folders are reachable only by naming the folder literally in `match`. Whether a run should see `.agents/` and `.claude/` by default is the open question, and the audit and the acceptance run depend on it.
- **Coded templates.** `{key}` matches the schema's `code`, a dash and one or more digits. Template violations (`{key}` in an uncoded schema, a wildcard in a coded one, a missing `{key}`, empty or relative segments, stray braces) are exit 2 with no id and `details: []`, because `config.match-template` is built with the schemas in ticket 5.
- **Scope.** Resolved and validated for `get`, but a path argument is not narrowed by it, since a path already names the file. It becomes observable with keys and `list` (tickets 7 and later). A name that is no namespace exits 1, a glob may match none, an empty `TYPDOC_NAMESPACE` counts as unset, and `name::x` (an import) exits 1 as not read yet. The current-directory rule applies only to namespaces that have a folder. The binary always passes no prefix; the parameter is unit-tested.
- **`config.legacy-file`** is reported only when `.typdoc/config.json` also exists, because discovery finds a project by that file. Doubt: a project with only a `.typdoc.json` exits 5 (no project), not with this id. The design says a legacy file beside the folder is a config error.
- **Shape errors in `config.json`.** A wrong-typed `validation` or `lock`, or a `lock` other than `local` or `git-common`, is `config.parse` with `complete: false`. The design ties `complete: false` to "cannot be parsed" and "unknown version", so this widens it; no other id fits. The same fault in a collection file is `config.collection-parse` with `complete: true`. `imports` is accepted with any value and left to ticket 17.
- **`config.rule-unknown`** is also used for a bad `level` and for an option value of the wrong type. A rule object may omit `level`, since a collection states only what differs.
- **`namespaces`.** A non-text entry is `config.namespaces-entry`. A folder that is named explicitly is checked like a matched one (`default`, a bad name and `.hidden` give `config.namespace-name`). "Holds its own `.typdoc`" means any `.typdoc` entry, not only one with `config.json`. A matched symbolic link is exit 6. `[]`, or a glob that matches nothing, gives no namespace and no error.
- **Files outside every namespace folder** are in no collection, so `get` gives 5. The design says a relative path can still point at them; that is refs work.
- **Order of `details`:** by path, then id, then message. `path` is the config file; for namespace errors it is `.typdoc/config.json` with the folder named in the message. The `error` string is the first detail, prefixed with a count when there are several.
- **Errors still without an id** (`collections.overlap`, a missing schema, a remote schema URL) keep ticket 1's behaviour: exit 2 with `details: []`.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (208 passed and 1 ignored, from 112) and `scripts/check-public-text.sh` with the names list, all green.
- The eleven ids moved from `UNIMPLEMENTED_RULES` to `RULES`, each with a fixture and an exact expected set. The checks over `fixtures/broken/` now carry weight and were shown red by planted faults: a second rule in a fixture, a fixture that trips nothing, a folder with a wrong name, a folder removed. Re-run by hand, each planted, red, removed: `complete` always true, the dot rule off in the template matcher, a second rule tripped by `config.rule-unknown`, `config.parse` renamed to a name no rule has.
- New valid fixtures: `valid/several-namespaces` and `valid/templates`. The default namespace with no folder of its own is `valid/minimal`, with an explicit test.
- Not shown: the text form of a multi-error failure is unreachable, since `get` without `--json` exits 1 first. Config errors as findings in the `validate` report wait on ticket 8; today each one stops the command.
- Left alone: near-duplicate directory listings in `index.rs` and `namespaces.rs`, `config.rs` at about 470 lines, and `Project::scope` as a thin forward to `scope::choose`.
