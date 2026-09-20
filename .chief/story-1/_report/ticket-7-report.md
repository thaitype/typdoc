# Ticket 7 Report

## Ticket
Keys per namespace, the forms an argument may take to name a document, and `get` by key in a project with one namespace and in one with several.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A namespace prefix is read the same way on a path argument and on a key.** Choosing a namespace names "a prefix on a key or path argument" in as many words, and Arguments that name a document does not mention a prefix at all, so it narrows nothing here. `story-2:story-2/tickets/WF-9.md` names the same document as the bare path, because a path stays relative to the project folder and scope does not narrow a path argument, which is ticket 4's reading and is kept. A prefix that names no namespace exits 1 and lists the namespaces the project has. Refusing the form outright was the first reading, and it was wrong twice over: it gave exit 1 to an argument that ends in `.md`, with a message saying that it does not, and it left the words "or path argument" with no effect.
- **A prefix therefore has priority over a path that really begins with `word:`.** A document whose project-relative path starts with a name and a colon before the first `/` cannot be named by its bare form: `weird:name.md` reports that `weird` is not a namespace, even with that file in the index. The design gives the way out and it was checked by running: `./weird:name.md` reads the file, as Refs says for a path that really contains a colon. The form is never decided by what happens to exist on disk, so the outcome is the same whether or not the prefix names a real namespace. Doubt: the way out is written under Refs and not under Arguments that name a document, so a reader of that section alone does not find it.
- **`project::` is refused as not read yet (exit 1)**, for a path as for a key, which follows the same reading `scope` already uses. Imports are ticket 17.
- **`code` is always in `get`'s document object and may be null; `key` is there only for a coded document.** The design lists `code` beside `collection` and `schema` without condition, and makes `key` part of the name only when the document has a code. The name in `toc` takes `path`, `namespace` and `key`, and no `code`, since the design calls it the name and not the whole document object. Doubt: a consumer that reads `code` on a `toc` document finds nothing there.
- **A key that is not found carries no `./name` hint;** only a path does, since a key names no place on disk for a hint to point at.
- **A path on disk is normalized by its parts, with no call to the file system:** `.` and `..` are folded, and a `..` that would climb past the start simply stops. Not verified past the cases the tests cover: an argument with more `..` than there is depth is an unexercised corner.
- **A key exists per namespace, and one that exists in more than one in scope exits 1** with every choice listed and a `candidates` array, as the design asks; typdoc picks none.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (333 passed and 1 ignored, from 303) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone, red, removed: `std::env::var` in `argument.rs` (library code) and `std::fs::write` in its `#[cfg(test)]` module (test code), both refused by the bans in `crates/typdoc-core/clippy.toml`. Planted alone on purpose: a violation in library code stops the crate compiling before any test is reached, so planting both together proves nothing about the test-code ban.
- Checked by running, not by reading: the prefix on a path in all three of its outcomes, the bare path unchanged, `project::` still refused, a path differing from the file only in case not found, the `./name` hint, the `candidates` shape, and the `./` way out for a colon in a path.
- `DocumentArg::Path` now carries the prefix beside the path, in the shape `Key` already had.
- New fixture documents under `valid/several-namespaces`: a coded collection with `WF-1` in two namespaces, so an ambiguous key has something to be ambiguous about, and `WF-9` in one.
- The four `get` goldens gained `"code": null`, written by hand, since `code` is now always present.
- Not shown: the cross-command name test has no leg for an imported project, since imports are ticket 17.
