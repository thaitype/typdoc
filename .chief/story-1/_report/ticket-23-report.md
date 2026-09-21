# Ticket 23 Report

## Ticket
Build what the design's paragraph **Which files a run reads** decided: the leading-dot rule split
between folders and file names, a symbolic link to a folder not followed, `.gitignore` not read,
and an entry a `match` reaches that cannot be read skipped and reported under `files.unreadable`
instead of stopping the run with exit 6.

## Outcome
done. `files.unreadable` left `UNIMPLEMENTED_RULES` and has a fixture with its exact expected set;
the accounting invariant was re-checked on every fixture project against a re-written independent
count, since what "the files the run reads" means changed under it.

## What the walk answers now

One rule, asked in two places:

- **A folder** is entered by a segment of plain text and by no wildcard. `*`, `.*`, `.g*t` and
  `**` all pass a folder named `.agents` by; `.agents` written out enters it. That is
  `Segment::matches_folder`, and `namespaces` entries use it too, so a collection and a namespace
  answer the question the same way, which is what the design says they should.
- **A file name** has no dot rule at all: `*` matches `.e.md`, because a file is named by the
  template that reaches it rather than found by walking into it. That is `Segment::matches`, and
  it is also what `filename.pattern`'s scan of a coded collection's folder now uses, so a dotted
  name that fits no template there is a stray like any other.

A symbolic link and a name that is not valid UTF-8 are no longer errors. Where a template reaches
one, the walk skips it and records it; `validate` turns each into a `files.unreadable` finding at
`error`, carrying `path` and `namespace` and no `collection` or `key`, the shape
`filename.pattern` already uses for a file that is in a namespace and in no collection. `get` on
the name the link goes by is exit 5: there is no document there, because the link was not
followed.

`.gitignore` was already unread and now has a test that says so.

## The audit's own walk, and the accounting invariant

`index::all_markdown_files` is the independent walk that answers `uncollected`, and it is not a
template, so it had to be given the same answer by hand. It now takes the collections' templates,
reads the name of every folder they write out as plain text (`Template::literal_folder_names`),
and enters a folder whose name begins with a dot only where one of those names it. It asks by
name and not by position, because it stands at no position in any template: a name the project
wrote out is a folder the project said it wants, which is the whole reason a plain-text segment
reaches a folder a wildcard does not. Asking by position was the first shape and it was wrong —
it passed over `.agents` in `**/.agents/*.md`, which the per-collection walk enters, so a file
beside the covered ones there was in no list and in no count. It lists a file whose own name
begins with a dot, and it skips a symbolic link and a name that is not UTF-8 silently — the
design's row for `files.unreadable` is about an entry a `match` reaches, and this walk is not a
`match`, so it adds no finding of its own.

Without that, a project that keeps its documents under `.agents/` would have every covered file
counted and nothing said about the files beside them that no `match` reaches — the audit would
look clean by leaving something out, which is the one thing the story's first criterion exists to
catch. `fixtures/valid/dot-folder/` is that shape: `.agents/notes/*.md` is the `match`,
`.agents/other.md` and `.agents/.private.md` are not covered, and both are listed.

The invariant's independent count in the test was re-written to the same rules, written out from
the design and reading the fixture's own collection files as plain JSON, never going through
typdoc. It is checked on every fixture project, two of which changed:

- `valid/templates` reads one file more than before, `notes/.e.md`, which `notes/**/*.md` now
  matches. `notes/.hidden/d.md` is still unread: `**` is a wildcard.
- `broken/files.unreadable` is new; its symbolic link to a file is counted by neither side.

## Fixtures, and how each was made

- `fixtures/broken/files.unreadable/` is committed, with a link to a file (`link.md`, which
  `*.md` matches). It survives `git add` as mode 120000 and comes back as a link in a fresh
  checkout, which was checked by writing the index out to an empty folder. Its expected set is
  exactly `["files.unreadable"]`.
- A link to a folder is **not** committed. A folder link under `fixtures/` made the public-text
  gate's `grep` print `Is a directory` on every run, and a gate whose output is noisy stops being
  read. Changing what the gate scans was the wrong way to make room for a fixture, so the case is
  built in a temporary folder inside the tests, which is how the file-name cases are built too:
  `a_symbolic_link_a_match_reaches_is_files_unreadable_and_every_other_file_is_still_checked` in
  `validate.rs`, and the matching tests in `templates.rs` and `namespaces.rs`. With the fixture's
  folder link gone, the first of these was shown red by making `enter` stop skipping a folder
  link, then restored.
- A name that is not valid UTF-8 is **not** committed. A `\xff` byte in a file name is storable
  but is a poor thing to hand to every checkout of a public repository, so those three cases are
  built in a temporary folder inside the tests instead: a file name and a folder name at
  `validate`'s seam, and a file name at `get`'s.
- `fixtures/valid/dot-folder/` is committed and holds nothing unusual beyond the dot names.

## Decisions taken where the design is silent, with the doubt that remains

- **A template that ends in `**` takes a file whose name begins with a dot.** The design's
  sentence about a leading dot in a file name is about `*`, and `**` stands for folders. The
  reading taken is that the leading-dot rule is about entering folders and about nothing else, so
  `docs/**` takes `docs/.a.md`. Doubt: `**` at the end is the one place where no segment names
  the file, so an argument that it is "whatever is here" and should pass a dot file by is
  available, and nothing in the design settles it.
- **A `files.unreadable` finding about a name that is not valid UTF-8 carries that name written
  with replacement characters**, so the finding sits in `findings` under a path that reads like
  the entry it is about. Doubt: such a path cannot be opened, and could in principle collide with
  a real file whose name holds U+FFFD. The alternative, naming the folder that holds it, loses
  which entry is meant, and the message says what was done either way.
- **A `namespaces` entry that matches a symbolic link still stops the run with exit 6.** The
  design's row for `files.unreadable` names an entry a `match` reaches, and a `namespaces` entry
  is not a `match`; refusing is not following, so the sentence about not following a link keeps
  its effect. Doubt: the two walks now answer this one question differently, and a later ticket
  may want them the same. **Open, and not fixed here:** it is an inconsistency between two
  entries of the same design, it is left as built, and it does not hold this ticket back.
- **A symbolic link under `**` that points at a folder whose name begins with a dot is skipped
  with no finding**, because a wildcard reaches no such folder at all, so there is nothing for it
  to have reached.
- **An entry a folder step matched that is not a folder is left alone rather than reported**,
  unchanged from before: nothing would have been read from it either way.
- **`files.unreadable` is reported by the whole-project scan of `validate` and by nothing else.**
  A `paths` scope reports findings about the documents the caller named, and an unreadable entry
  is not a document and cannot be named — an argument that names no document stops the command
  with exit 5 before any report. `--schemas` checks schemas and no file. `get`, `list`, `toc` and
  `refs` report no findings at all; the rule tables are `validate`'s. So it sits where
  `collections.overlap`'s project scan and `filename.pattern`'s stray list already sit, which is
  the only place a finding about the project as a whole is made. Doubt: someone running
  `validate` on one path in a folder that also holds a link is told nothing about the link.
- **The audit walk asks for a folder's name and not for its place**, so a template that names
  `.agents` reaches a folder of that name wherever the walk meets one, even under a template
  that would only have reached the one at the top. It over-lists rather than under-lists, which
  is the direction this walk already errs in for every folder whose name has no dot: it lists
  every `.md` below a namespace folder, whether a `match` reaches it or not, because that is
  what `uncollected` is for. Doubt: a project with two folders of the same name, one of them
  meant and one not, sees the unmeant one's files listed as uncollected.

## Exit code 6

Exit 6 is still produced by a test and stays out of `UNPRODUCED_EXIT_CODES`. It no longer comes
from a symbolic link a collection matches; the case `produced_exit_codes` uses is unrelated to
the walk — a `.typdoc/collections/folder.json` that is a folder, so the read fails with EISDIR —
and a `namespaces` entry that matches a link produces it as well.

## Notes

- `files.unreadable` lives in `crates/typdoc-core/src/rules.rs`, not in
  `crates/typdoc/src/registry.rs` as the ticket's wording has it; `registry.rs` holds the command
  and exit-code lists. Nothing else about that item changed.
- Consumers walked before the commit, since this ticket changes the denominator of every count:
  `Index`'s five readers in `project.rs` and `refs.rs`; the audit report builder and its one call
  of `all_markdown_files`; `stray_files`; `--json`'s `summary`, its `audit` object and the text
  form in `cli.rs`; every golden case (none runs against a fixture holding a dot name or a link);
  and every fixture project through the invariant sweep.
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (679 passed, 0 failed, 1 ignored, from 667) and the public-text gate, all
  green, every cargo command under the memory ceiling.
- Each new test was shown able to fail by changing the code it pins and watching it go red, then
  restoring: the file-name dot rule put back, the folder rule's plain-text test removed, the
  symbolic-link and non-UTF-8 branches of the two walk steps removed one at a time (four separate
  branches, four separate tests), a walk that follows a folder link, the audit walk refusing a
  dot folder a `match` names, the audit walk skipping a dot file, `filename.pattern` passing over
  a dotted name again, and the report of unreadable entries removed.
- The bans were shown red one at a time: `std::fs::write` inside the audit walk's folder test in
  `index.rs`, red with `use of a disallowed method std::fs::write`; and, separately,
  `std::process::Command::new` inside a new test in `crates/typdoc/tests/validate.rs`, red with
  `use of a disallowed method std::process::Command::new`. Both were removed.
- `scripts/check-public-text.sh` is unchanged by this ticket, byte for byte as it was at
  `c883410`. An earlier commit of this ticket changed its sweep to drop symbolic links; that
  change is taken back, and the folder link that needed it is no longer a committed fixture (see
  Fixtures). Run on the tree as it was with the link committed, the old script printed the noise
  and still exited 0.
- Raised by the review and acted on: the audit walk's folder test was a two-armed search whose
  second arm could never fire, so it is a plain membership test, in the walk and in the test's
  own count alike; recording a skipped entry is one function that `take` and `enter` both call,
  instead of the same insert written three times, the two of them keeping the different order
  they ask their questions in, which is the part that is deliberate; the reason carried for a
  skipped entry is a `&'static str` rather than an owned `String`, since there are two of them
  and both are constants; the three values that stay the same down the audit walk's recursion
  travel as one; and the two message constants are named alike. Left as it is: the count read
  from the collection files in the test is a second, hand-written reading of the same design
  sentence on purpose, so it is not shared with the crate.
- Also raised by the review and acted on: the audit walk passed over a folder a `match` writes
  out in plain text after a wildcard (`**/.agents/*.md`), so a file beside the covered ones there
  was counted nowhere while the per-collection walk read its siblings. The rule is now by name
  rather than by position, in the crate and in the test's own count, and a test pins it: putting
  the old rule back turns that test red.
- One change in the diff the ticket does not name, and why it belongs to it. The `namespaces`
  entries had to move to the folder rule: this ticket takes the leading-dot test out of
  `Segment::matches`, so without the move an entry of `*` would have started matching `.git`,
  the opposite of what the design says; it also settles `.*`, a glob that begins with a dot,
  which used to reach a dot folder and now does not.

## Checked again after the build, by running

- `scripts/test.sh`: 679 passed, 0 failed, 1 ignored. `cargo fmt --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- The five behaviours, in a project written for the check rather than the fixtures: `**` does not
  enter `.hid/`, a collection that writes `.hid/*.md` does; `*` takes `notes/.dotfile.md`; a link to
  a file, a link to a folder and a file name that is not valid UTF-8 are each a `files.unreadable`
  finding while the other files are still answered; `.gitignore` names a file that is still read.
  `get` on the linked file exits 5. The accounting invariant held on both runs (4 and 5 files
  read, 4 and 5 counted).
- `validate` with an unreadable entry exits 2, since the finding is an error.
