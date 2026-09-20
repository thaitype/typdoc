# 9: What is the test strategy, given tests must run offline and gates must be able to fail?

Type: wayfinder:grilling
Status: claimed
Blocked by: 1, 6

## Question

Project rules require that `cargo test --workspace` passes offline, that remote schemas are mocked, and that every command has stable `--json` output and exit codes 0 to 4. Prior team experience is that a check which cannot fail is not evidence.

Decide: how fixtures are laid out (namespaces under `tests/` or `examples/`), whether `--json` output is pinned with golden files, how remote schemas are mocked given the HTTP client chosen in ticket 6, how frontmatter round-trip (write, then re-read, then diff the body byte for byte) is tested given the approach chosen in ticket 1, and which deliberately broken fixtures prove each validation rule can go red.

## Inputs

Checks already required by other tickets, grouped by the mechanism each one needs. The strategy decides the mechanism per group, not each check.

1. **Pure logic against fixtures.** Slugs and anchors (ticket 4: fixtures built from GitHub's renderer, empty slugs, duplicates, a percent-decoded fragment, a case-insensitive compare, a `%` not followed by two hex digits); columns (11: a Thai line and an emoji line, each with its expected `col`); link forms (10: inline, image and reference-style links, a definition reported once with its use count, an unused definition, a duplicate label, text that looks like a link including one with a title, `<…>` and `%20` accepted, `version 1.2` not reported, an undefined `[t][ref]` not reported, definitions inside code are not definitions); query grammar (5: one fixture for each error it names); config and namespace rules (13, 7: bad namespace names, entries with `/` or `**`, dot folders, `default`, a nested `.typdoc`, an orphan state file, `name:` against `name::`, `names.shadowed`, a leftover `name` key).
2. **Deliberately broken projects.** Every validation rule needs a fixture that turns it red (project rule and this ticket's question).
3. **Write paths.** Frontmatter round trip (1: real documents, untouched lines byte-identical, `no`, `yes`, `on` and `off` stay strings, a re-read mismatch rejects the write with no fallback, anchors and tags probed first); `new`, `set` and `mv`, including `mv --renumber` (13: the old key is never issued again, refs rewritten in the form that is correct from each document, `moves` recorded, `refs.moved` naming the new key), state files per namespace, and `mv` keeping each link's written form (10).
4. **Process level.** Locks (7): a lock held for more than thirty seconds is not taken; two processes racing get one winner and the other exits 4; an interrupted process leaves no lock; a lock removed and re-created by another writer is not removed by us; a reader never sees half of `lock.json` while `pull` writes it.
5. **Environment isolation.** The order in which `imports.json` is found (7: five cases); no test reads the real home directory; an unset or empty `${ENV}` in an import path; `imports.absent` at `error` gives exit 2; `TYPDOC_DIR`, `TYPDOC_NAMESPACE` and `--namespace` scope (13).
6. **Network.** An in-memory `Fetch` for most tests; a loopback TLS server only for the https-only and redirect rules; timeout, response size limit and `HTTPS_PROXY` (probed first); a vendor copy whose hash differs from its name and a missing copy are config errors (6, 7).
7. **Shells.** Every documented example goes through sh, bash and zsh and reaches typdoc byte for byte identical; an unquoted case must go red; the tests and the document share one list of examples; zsh has to be run for its row (7).
8. **Across all of it.** Tests run offline; `--json` is stable for every command; exit codes 0 to 4; a claim that macOS is supported needs a macOS run; a fixture copied from a real document into this public repository is reviewed first (project rules, 7, 1).

## Answer

### Fixtures

Decided: one top-level `fixtures/` directory, read by every crate, with `valid/<name>/` (complete projects that validate clean), `broken/<rule>/` (each names the rule it exists to prove and carries the rules it is expected to trip) and `roundtrip/` (documents for the write tests). `examples/` (samples for users) stays separate. Reasons: the tests of `typdoc-core` and of the CLI need the same fixtures, and two copies would drift apart; naming `broken/<rule>/` after the rule shows at once which rule has no fixture; keeping `examples/` apart lets a sample for users change without breaking a test that needs fixed values.

What carries this decision:

1. **Two coverage tests, one for each direction.** Every folder in `broken/` names a rule that exists, so an orphan folder cannot look like coverage. And every rule that exists has at least one fixture in `broken/`: a rule with no fixture has never been shown to fire, which cannot be told apart from a rule that is silently broken. The second test goes red the moment a rule is added without a fixture.
2. **A fixture must go red for the reason it claims.** Red alone is not enough: if the fixture for one rule happens to trip another rule as well, the test passes on the wrong evidence. Each broken fixture carries the exact set of rules expected to fire, and the test compares that set exactly, not only a non-zero exit. When a later change makes a second rule fire, that is loud, not silent.
3. **The examples are validated as a user would receive them.** A test runs `validate` on every project in `examples/`, with the default configuration that reaches a user (the example's own config, default rule levels, no test-only flags or overrides). Otherwise it proves the examples work under conditions no user has.
4. **A fixture copied from a real document passes the public-text gate** (`scripts/check-public-text.sh`) as well as the review decided in ticket 1. They are two separate requirements and both apply.

Default: the expected set of rules is written by hand and never generated by running the tool, since a generated expectation certifies whatever the tool does. The coverage tests iterate the same registry of rules that the validator uses, so the two lists cannot drift apart.

### Fixtures and packaging

Decided: `fixtures/` and `examples/` stay at the repository root and are not shipped in any package. The test suite runs from a checkout of the repository.

Verified with `cargo package --list` on a throwaway workspace whose crate sits under `crates/` and whose fixtures sit at the root: only files inside the crate's own directory are packaged (`Cargo.toml`, `src/`, `tests/`), the root directories are not, so a packaged `tests/` reads a `../../fixtures` that does not exist in the package. The mechanism: in a repository with a single crate the package root is the repository root, so root directories are packaged; in a workspace whose crates live under `crates/` the package root is the crate's directory and the repository root lies outside it.

When fixtures cannot be found, the loader fails loudly with a message saying the suite has to run from a checkout of the repository. It is not a bare file-not-found and it is not a skip, so someone running the tests from a tarball is told the reason at once instead of guessing what they configured wrongly.

Not decided here: whether tests must run from a published package, and so whether crate manifests exclude `tests`. That belongs to the distribution decision (see the map). Excluding `tests` now would settle distribution by side effect, and a published crate would look as if it had no tests at all, which is untrue and misleading. What is decided now is where fixtures live, not the shape of a package.

