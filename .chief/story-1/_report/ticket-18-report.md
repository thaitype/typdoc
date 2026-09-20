# Ticket 18 Report

## Ticket
The state file read for its three rules, pinned copies of remote schemas read from disk and checked against their names, and the drift check on qualified `target` names.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A qualified `target` reaches an imported project's own schemas, and so do the two rules that were skipping it.** A ref crossing into an import is now allowed or reported by the same `target` it would be inside the project, and a qualified name that the imported project no longer has is reported as drift. A bare name in `target` never matches across an import and a qualified one never matches inside it. This is what the previous ticket parked here: assembling the imported project's qualified names is what those two rules were missing.
- **An import alias may be any name but the four URL schemes the design lists.** The check that was here refused any name shaped like a scheme, which is nearly every name and includes `memory` and `chief`, the two the design's own examples use. It now matches `http`, `https`, `mailto` and `file` exactly. The shape test still does the job it was written for, telling a schema reference's URL from a path, and nothing else uses it. Checked by running, both ways: the two example names validate clean, and an alias named `https` is still refused with a message that names the four.
- **`config.state-uncoded` also covers a state entry naming a collection that is not there at all,** since such a collection has no schema and therefore no code, and no other id fits. Doubt: an id of its own is a defensible alternative that the design does not offer.
- **The state file and the pinned copies are under the project's `.typdoc` folder,** following the design's folder tree over the shorthand used elsewhere, which reads as elision.
- **A state file or a lock file that exists and cannot be read is a stop with no id,** as for the other files the tool writes and re-reads rather than a person editing by hand.
- **An unconfigured alias in a qualified `target` is drift under `schema.valid`,** the same as a schema the imported project has renamed away: in both the target names nothing.
- **A relative reference inside a remote schema's own `extends` is resolved against the project folder, not the schema's URL.** The design says it should follow the URL. Nothing in this story needs a chain of remote schemas, and the case ends in an honest "does not exist" rather than a wrong fetch, but it is a gap and it is marked in the code.

## Open against the design, carried to the end of the story
- **A namespace named after a URL scheme is not checked, while an import alias is.** The design's sentence covers both sibling names and import aliases, and the rule table names only import names, so the namespace half is enforced nowhere. Checked by running: a project whose namespace is literally `http` validates clean, while the same name as an import alias is refused.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (646 passed and 1 ignored, from 627) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone inside a function body, red with the error naming the banned call, removed: `std::fs::write` in the reading of the lock file, and the same in a test beside it.
- **The read-only promise was checked rather than assumed:** every file under `fixtures/` was hashed, `validate`, `list` and `validate --schemas` were run across four projects including the one with a pinned schema, and every file hashed again. Nothing changed, nothing was added, nothing was removed. This is the ticket that reads the state file and a vendored copy, so it is the one where writing something by accident would be easiest.
- Checked by running, not by reading: a ref crossing an import allowed by a qualified `target` and another refused by it, drift reported for a qualified name the import does not have, both example alias names accepted, a reserved scheme still refused, and a namespace named for a scheme going unreported.
- The pinned copies are made by hand: the bytes were written first and hashed with the system's own tool, never produced by running this project.
- A rule that fires on every project without a state file made several existing fixtures report a finding that was not theirs; each was given the state file it should always have had, and the coverage test still holds each fixture to its own exact set.
