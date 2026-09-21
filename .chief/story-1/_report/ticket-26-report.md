# Ticket 26 Report

## Ticket
A `namespaces` entry that reaches a symbolic link, and a namespace named after a URL scheme: two
places where a rule reached one mechanism and not the one beside it. Plus the sweep of every place
a name enters the two name spaces and every place a directory entry is read.

## Outcome
done. Both cases are decided in `docs/design.md` and built as written there; the sweep found one
more place with the same symptom, which is fixed, and two places that need a decision the design
does not hold, which are left and named under Notes.

- A `namespaces` glob that reaches a symbolic link skips it, and `validate` reports it under
  `files.unreadable`; every other namespace is answered. A `namespaces` entry that names a link in
  plain text is `config.namespaces-entry` and says to name the folder the link points to.
- A namespace named `http`, `https`, `mailto` or `file` is `config.namespace-name`. `[namespace-scheme]`
  is out of `KNOWN_GAPS`.
- The scenario, run before the change on the unmodified tree: a project with the namespaces
  `story-1` and `story-2`, `namespaces: "*"` and `ln -s story-2 current` stopped `validate` and
  `list` with exit 6 (`current: a symbolic link is not read: whether a run follows one is not
  decided`). After the change `list` gives both documents once each under their own paths
  (`story-1/a.md`, `story-2/a.md`), and `validate` exits 2 with one finding, `files.unreadable` at
  `current`, and `checked.documents` 2.

## Decision if any
Decided where the design is silent, with the doubt that remains:

- **The `files.unreadable` finding for a skipped link carries `path` only.** It is in no
  namespace, so it has none to carry, the same shape `schema.valid` uses for a finding about the
  project's own files. It is reported by the whole-project scan of `validate`, whatever `--namespace`
  or `TYPDOC_NAMESPACE` says, for the reason a `schema.valid` finding is: it is about the project,
  not about a namespace. Doubt: `validate --namespace story-1` still lists a finding about a link
  that points at `story-2`.
- **A glob skips and reports only a symbolic link to a folder.** A link to a file and a link to
  nothing (dangling) are left alone, like any other entry that is not a folder: a file cannot be a
  namespace, and a glob already leaves every regular file alone, so reporting a link to a file would
  give an error finding for `CLAUDE.md -> README.md` at the project root while `README.md` is silent.
  The design's paragraph says "a folder that is a symbolic link", and its table row says a glob
  "leaves a file that is a symbolic link alone". Whether a link points at a folder is asked with
  `fs::metadata` on the link's path. Doubt: a dangling link that was meant as a namespace is not
  reported.
- **A folder whose name is not valid UTF-8 that a glob reaches is skipped and reported under
  `files.unreadable`**, the way a `match` treats one. Before, it was `config.namespace-name`
  (`the folder `\u{fffd}` cannot be a namespace`). The design's table row now names a `namespaces`
  glob for a name that is not valid UTF-8, and the sentence "a matched folder with any other name is
  a config error" is left to every name that is valid UTF-8, which gives both words an effect.
- **An alias given in the machine file `imports.json` is held to the URL-scheme rule.** The
  design's `schema.valid` row says "import names colliding with URL schemes" and does not tell the
  two sources apart; before, only an alias in `config.json` was refused, so `{"http": ...}` in the
  machine file validated clean (run before the change, exit 0). The finding is `schema.valid` at the
  machine file's own path, which is where the alias is written. Doubt: that path is outside the
  project and is printed as the path found, absolute; no other finding has one.
- **A link is skipped before its name is looked at.** A link named `my link` or `http` that a
  glob reaches is a `files.unreadable` finding and not `config.namespace-name`, since a link is not
  a namespace whatever it is called. The design's "a matched folder with any other name is a
  config error ... never skipped" is about folders; a link is not followed, so it is not one.
- **`Error::symbolic_link` and `Error::Unreadable` are removed**, since nothing constructs them
  once a link is a finding; exit 6 is still made by a read that fails (`Error::Io`).

## Notes

### Consumers walked
- `namespaces.rs` and everything that reads what it returns: `Config::load` (the one caller) now
  receives `Resolved { namespaces, skipped }`; `Config.namespaces` is read by `Project::load`,
  `Index::build`, `stray_files`, `all_markdown_files`, `state::orphans` and `read_state`, `scope`
  (`choose`, `inside`), `ref_name_in` and `refs.rs`. A skipped link is not in that list, so none of
  them sees it.
- `Report` and config findings against config errors: a skipped link is NOT added to `Report`,
  which stops the command; it is held on `Config.skipped` and turned into a finding by
  `Project::validate`'s whole-project scan. A plain-text entry that names a link and a
  reserved-scheme name are `Report` errors, as their ids say (they stop the command with exit 2, as
  every `config.*` error does today).
- Behaviour of each command on the project with the skipped link (run, and tested): `validate`
  exits 2 with the one finding and every other namespace answered; `validate --audit` counts 2
  documents, `uncollected` empty; `validate current/a.md` and `get current/a.md` and `toc
  current/a.md` are exit 5 (no document there: the link is not followed, so the path names nothing);
  `list` and `refs story-2/a.md` and `get story-2/a.md` answer normally and report no findings
  (only `validate` has findings).
- The accounting invariant: `all_markdown_files` walks only namespace folders, so a skipped link is
  in neither side of the count. The invariant test passes on every fixture project.
- `Error::symbolic_link` and its message: removed with its only caller. The words "whether a run
  follows one is not decided" are in no file of the tree except the ticket's own text, which
  quotes them, and the reports of tickets 1 and 4, which are records of what was true then and are
  not edited here.
- Registry lists: `KNOWN_GAPS` loses `[namespace-scheme]`, and nothing pins the other two entries
  through this one. `UNPRODUCED_EXIT_CODES` is unchanged (3 and 4): exit 6 is produced by
  `produced_exit_codes` in `coverage.rs`, by a collection file that is a folder, which does not
  depend on a link. Ticket 23's report says a `namespaces` entry that matches a link produces it as
  well; that is no longer true and is not edited there.
- Rule table coverage: the rule ids in the design's tables are the same; the fixtures of
  `files.unreadable`, `config.namespaces-entry` and `config.namespace-name` are unchanged and still
  trip exactly their rule. No fixture is added: a link to a folder is not committed under
  `fixtures/`, and each new case is built in a temporary folder.

### The sweep

Every place a name enters the two name spaces (the left side of `name:` and `name::`):

| Place | Applies the rule? |
| --- | --- |
| Namespace names, `namespaces::resolve` (`name_problem`), reached by a plain-text entry and by a glob | yes, now: characters, `default`, and the four URL schemes |
| Import aliases in `config.json` (`Project::load_inner`, `reserved_alias_finding`) | yes: the four URL schemes under `schema.valid`; the character set of an alias is not checked, and the design does not give one |
| Import aliases from the machine file `imports.json` | no before, yes now: same finding, at the machine file's path |
| `--namespace` and `TYPDOC_NAMESPACE` (`scope::select`) | not applicable: a value selects names that already exist, so it cannot introduce one; a scheme-named namespace can no longer exist to be selected |
| A `namespace:` or `alias::` prefix on an argument (`scope`, `Project::imported`) | not applicable, same reason: matched against names that exist |
| A collection name (`read_collections`, `config.collection-name`) | not applicable: it is never the left side of `name:` or `name::`; it is checked for its characters |
| A schema name, a qualified `target` (`alias::schema`) | the alias part is an import alias, checked above; a schema name is not in either name space |

Every place a directory entry is read:

| Place | Applies the rule? |
| --- | --- |
| `namespaces::folders` (the folders directly inside the project folder) | yes, now: a link to a folder that a glob reaches is skipped and reported, a link to a file or to nothing is left alone, one a plain-text entry names is refused, a name that is not UTF-8 is skipped and reported |
| `index::Index::build` / `walk` (per-collection walk) | yes (ticket 23) |
| `index::all_markdown_files` (the audit's own walk) | skips a link and a non-UTF-8 name silently, on purpose: it is not a `match`, so it reaches nothing, and the `match` walk reports the entries it reaches |
| `index::stray_files` (scan of a coded collection's folder) | same as the audit walk: reads only regular files, passes a link over |
| `config::read_collections` (`.typdoc/collections/*.json`) | not applied, and the design has no rule for `.typdoc/`: a link there is read through with `fs::read`. Left as it is; no decision needed for this ticket |
| `state::orphans` (`.typdoc/state/`) | names only, nothing is opened; not applied, same reason |
| `namespaces.rs` nested check (`symlink_metadata` on `<folder>/.typdoc`) | does not follow: a link named `.typdoc` counts as nested |
| `scope::inside` (`canonicalize` of the current directory) | follows links in the directory the caller stands in, which is not an entry a run reads; a shell inside `current/` is told it is in `story-2` |
| project discovery (`config_file(..).is_file()`, `discover`, `discover_for`) and an import's path | follow links: a path the caller gave, not an entry read |
| `refs::resolve_path` (`root.join(path).is_file()`) | **not applied, and left: needs a decision, see below** |

### Places that need a decision the design does not hold (left, not fixed)
- **A ref whose path passes through a symbolic link resolves.** `refs::resolve_path` checks a path
  that no collection matched with `is_file()`, which follows links, so a field `up: ../current/a.md`
  (with `current -> story-2`) resolves as a document outside every collection under a second path
  for `story-2/a.md`. The design says a link is not followed "so a run cannot ... read one file
  twice under two names" but the existence check of a ref is not a walk, and it does not say
  whether a ref through a link is `not-found`. Run: `validate` on such a project reports nothing.
- **`refs <document>` panics when a ref names a file that exists and is in no namespace folder.**
  Reproduction: a project whose `.typdoc/config.json` is `{"version":1,"namespaces":["story-1",
  "story-2"]}`, one collection `{"match":"*.md","schema":"schemas/note.json"}` and a schema with a
  `title` string and an `up` ref, folders `story-1/` and `story-2/` each with one document, a
  `README.md` at the project root, and `story-1/a.md` holding `up: ../README.md`. Then
  `typdoc refs story-1/a.md --json` exits 101 with `thread 'main' panicked at
  crates/typdoc-core/src/project.rs:3250:10: every project has a namespace with an empty folder when
  none matches by prefix` (`ref_name_in`). `validate --json` on the same project reports nothing.
  Independent of links and not changed by this ticket. What name a document outside every namespace
  has in a `refs` answer is not in the design, so it is left and needs a decision.

### Gates
Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`scripts/test.sh` (713 passed, 0 failed, 1 ignored, from 698) and the public-text gate, all green,
every cargo command under the memory ceiling.

- Tests removed: `a_symbolic_link_that_an_entry_matches_is_refused_and_not_followed` (it asserted
  exit 6 for a link a glob reaches, which is now a finding) and
  `a_namespace_named_after_a_url_scheme_validates_clean_unlike_an_import_alias_of_the_same_name`
  (it pinned the gap that is closed). No other test is lost.
- Tests added (17): in `crates/typdoc/tests/namespaces.rs`, the link a glob reaches (three
  spellings of the glob, with the finding, the two documents once each and the count), the
  commands on the link's path, the audit, the scope, a link to a file and a link to nothing (left
  alone, with a regular `README.md` beside `CLAUDE.md -> README.md`), a link named with a leading
  dot, a link named in plain text (two names), plain text against a glob, a folder that is not
  UTF-8, one test per scheme (`http`, `https`, `mailto`, `file`, each reached by a plain-text entry
  and by `*`), the names that only look like a scheme, and the control that an import alias of the
  same name is still `schema.valid`; in `crates/typdoc/tests/imports.rs`, an alias from the machine
  file and an alias in both files (refused once, at the project).
- Which case tells which pair of readings apart: `*` against `cur*` against a plain name (a link a
  glob reaches is skipped, a link a plain entry names is refused: the tests build both from one
  project); the two documents once each against a followed link (`current/a.md` would be a second
  path, and `list` would show four); a link `.current` against `*` (a glob reaches no dot name,
  so nothing is reported, against a wildcard that would); `HTTPS`, `Http`, `httpx`, `ftp`, `files`
  against the four (the rule is the four spellings and not their shape); a machine alias also named
  in `config.json` (one finding, at the project, not two).
- Mutations, each shown red, each restored from a backup copy of the one file:
  - the glob's skip no longer records the link: 2 tests red (the finding, and the scope);
  - every link counts as a link to a folder (the folder check made `true`): the link-to-a-file test
    red; the check made `is_ok()` (a link to a file counts): red; the check made `map_or(true, ..)`
    (a link to nothing counts): red; the check made `false`: the two folder-link tests red;
  - a plain-text entry no longer refused (`if !glob` made `if false`): the plain-text test red (the
    later "names a folder that does not exist" refusal still stops the run, so the exit code alone
    could not tell it; the message is what catches it);
  - `glob` always true: 6 red; `matches_folder` replaced by `matches`: 18 red, the dot-link test
    among them; the link branch never taken: 4 red; the non-UTF-8 branch never taken: 1 red;
  - the scheme check for `http` only: `https`, `mailto` and `file` red, each by its own test; the
    scheme check removed: all four red; `default` no longer refused: 2 red;
  - the report of skipped entries removed in `Project::validate`: 4 red;
  - the machine file's alias check removed: 1 red; its "not also in `config.json`" guard removed:
    1 red.
- The ban on writes was shown red: `std::fs::write` planted inside `name_problem` in
  `namespaces.rs` gives `use of a disallowed method std::fs::write` with the note `the read core
  changes no file`; removed, and no file was written.
