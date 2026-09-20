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
6. **Network.** An in-memory `Fetch` for most tests; the real adapter and the shipped binary against a plain-http loopback server; the timeout, the response size limit, the redirect limit and the proxy variables; a vendor copy whose hash differs from its name and a missing copy are config errors (6, 7).
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

### What needs a broken fixture

Decided: every rule and every config error needs a fixture in `broken/`; the outcomes of a command (exit 1, 3 and 4) do not, and are tested by ordinary CLI tests that check the exit code and the `--json` output.

The criterion, meant to be applied without a list: does the thing read a project and report a defect in it? If it does, it needs a fixture in `broken/`, because a check that reads a project can go silent without anyone noticing, and a check with no fixture has never been seen to fire. If it is what a command does when it meets a certain state (a false `--if`, a lock that is held), it is a CLI test. Whoever adds a new kind of report can then say on which side it falls without asking.

The line must not dissolve. Some rules need more than files (`imports.absent` needs an environment variable that is unset or empty; `collections.overlap` needs two files that collide, which are only files). So a fixture may carry declared values and nothing else: environment variables, and the name and arguments of the command to run. Never a setup script. A case that needs a script is not a fixture of a project; it belongs to the CLI tests. Otherwise fixtures that carry commands and setup would turn into command tests and the broader option (every outcome needs a fixture) would return through the side door.

Config errors get ids. The design listed them as plain sentences, so a caller that met one had to read the message; ids let it branch on them, as the exit codes do for a rule, and let each one have a fixture. About twenty ids are added to the design, in a table under Config errors, and `--json` carries them in `details[].rule`. This is what the design wanted from the start, not a cost of the test strategy.

Defaults: the only substitution a declared value may use is a placeholder for the fixture's own directory (an import path has to point at a neighbouring fixture); a config id is carried in `details[].rule` alongside rule ids (a single registry) and `file` replaces `doc` when the error is about a file.

For the contract: exit 1 currently covers three unrelated things (not found, bad arguments, I/O). For a tool whose main users are programs, a code that can mean three things says little, and a test that asserts exit 1 proves little. Not decided here.

### `--json` output

Decided: the `--json` output of every command is pinned with golden files. A golden is compared as parsed JSON (whitespace and key order do not matter; array order does, see below), and each command also has hand-written assertions on its load-bearing fields. The safeguards are structural, not a matter of discipline: anything that relies on people being careful breaks on the day someone is in a hurry, which is also the day the most output changes.

1. **Hand-written assertions live in files the golden-generating command cannot write.** This matters most. If a whole set is regenerated, the hand-written assertions are what would catch it, but only if they are not regenerated along with it; a net that is updated by the thing it watches is not a net. The generating command touches golden files only, and no path lets it write an assertion file.
2. **The generating command has no mode that regenerates everything.** It must be told which golden to regenerate. Regenerating one is easy; regenerating all of them is a visible chore, not one key. The day the output of the whole system changes is the most dangerous day, and the day one key would be pressed.
3. **The clock is injected as a constant, not scrubbed out before comparing.** If time values were removed, they would never be pinned, and a wrong format (an epoch instead of ISO, a lost time zone) would leave the golden green. Injecting a fixed value pins the format too.
4. **Which arrays guarantee an order is declared first.** If the order of an array is an accident of the implementation, pinning it gives either a flaky test or an accident frozen into a contract. The design states, per array, whether order has a meaning, and the golden compares accordingly: an array with no guaranteed order is sorted before comparing.

A golden regenerated from the tool's own output certifies whatever the tool does, including what it does wrong. The same principle stands behind the hand-written expectations of the broken fixtures; point 1 is what makes it effective rather than intended.

Defaults: the function that regenerates goldens refuses to write any path outside a `golden/` directory, and a test proves it by trying to make it write an assertion file, so the guard is shown to work. The clock is injected at the library boundary (a `Clock` passed into `typdoc-core`), and the shipped binary has no environment variable that fakes the time: such a knob could write a false `created_at` into a real document. So the outputs of commands that stamp the time (`new`, `set`, `pull`) are pinned through `typdoc-core` with the injected clock, and the binary-level goldens cover output that does not depend on the current time.

For the contract: the per-array statement of ordering (point 4) is part of the exact `--json` shapes, which the contract fixes.

### Remote schemas in tests

Decided: three layers. typdoc does not restrict the type of connection, so the tests need no TLS server, no test certificate and no way for the shipped binary to trust one.

1. **Logic, with an in-memory `Fetch` and no network at all**, through a `run(args, deps)` function at the library boundary (the shipped binary calls `run` with the real dependencies). No pin: the schema is fetched. A pin exists: it must not be fetched, which is what guarantees that typdoc works offline. The bytes stored equal the bytes received. The pin equals the SHA-256 of those bytes. A failed fetch gives the right error and exit code. A second fetch whose bytes differ from the existing pin is reported and never overwritten silently (ticket 7).
2. **The real `ureq` adapter through a real socket**, against a plain-http server on loopback. It fetches a real schema from a local server, which shows that the real adapter works and not only a fake. The timeout: a listener that never answers must really time out at the configured time. The size limit: a body larger than the limit must be refused. These two used to be checks of a setting and are now tests of behaviour; the adapter's constructor takes the timeout and the limit so that a test can use small values. The redirect limit: a server that redirects forever must fail after ten. The proxy variables: a loopback listener acting as a proxy records `CONNECT` when `HTTPS_PROXY` or `HTTP_PROXY` is set, and nothing when `NO_PROXY` names the host.
3. **The real binary, end to end**, which was not possible before. The binary that ships is run against a plain-http server on loopback. First, a schema with no pin is downloaded, stored under `vendor/` and its pin recorded. Second, the server is shut down and the binary is run again: it must still work, because it reads the pinned copy. The second step is the heart of it. One test shows two things at once, that the binary really downloads and that it really stops downloading once a pin exists, and shutting the server down is what lets the test fail: if the binary fetched again it would fail, so a green result cannot be an accident of nothing having run.

Gone from the earlier plan: a TLS server with a generated test certificate, a test that plain http is refused, a downgrade-redirect test, a certificate-error test on the shipped binary, and the whole question of a way to trust a test certificate in the shipped binary, since there is nothing to trust.

Why it changed, beyond being cheaper: under the earlier plan the real adapter never fetched anything successfully in any test. Every successful fetch happened against a fake, which is against the lesson that a path nothing walks is a path nothing checks. Now the real path is walked from start to end.

Not proven, stated plainly: a successful https request through the real adapter is exercised by no test, because the loopback server is plain http. It runs through the same adapter and rustls with the bundled roots, but nothing shows a genuine https fetch succeeding, and the first fetch over http is not authenticated. Closing the gap in https needs a loopback TLS server and a constructor that lets the adapter trust its certificate at library level only, so the shipped binary is untouched.

Proxy variables: a behaviour is not claimed in the documentation until it has been run, as for a shell in the quoting list. Probe of 2026-09-20 (`ureq` 3.4.2, `rustls` 0.23.45, rustc 1.96.0), with a loopback listener acting as the proxy. For an `https://` URL, an agent with default settings and an agent built with `https_only(true)` and `timeout_global` both sent `CONNECT schemas.invalid:443` to the proxy named by `HTTPS_PROXY`, and also to the one named by lowercase `https_proxy`. For an `http://` URL, the proxy named by `HTTP_PROXY` and by `http_proxy` received `CONNECT schemas.invalid:80`; `HTTPS_PROXY` alone was used for the `http://` URL too, so the variables are not applied per scheme, and a tunnel is requested even for plain http. With `NO_PROXY` naming the host exactly the proxy was not used, and with no variable the request went direct (a DNS failure). Not verified: precedence when several variables are set, `ALL_PROXY`, `NO_PROXY` patterns other than an exact host, proxy authentication, an `https://` proxy, and a completed tunnel followed by TLS. The documentation says only what was run, and it does (Remote schemas, Proxies).

### `run(args, deps)` is the only path

Decided: `deps` is the only way a module reaches the outside world. If any module can pick up a real client, the real clock or a global on its own, that route escapes testing with no signal and nobody finds out until it breaks. It follows the principle stated for the golden files: the net has to sit outside what it watches.

Defaults, so that this is enforced by structure and not by care: `ureq` is a dependency of the binary crate only, so `typdoc-core` cannot reach a real client at all. The clock, the environment and the home directory are reached in `typdoc-core` only through `deps` (a `Clock` and an `Env`; the location of `imports.json` in ticket 7 is tested with a fake environment for that reason), and `clippy.toml` lists `chrono::Utc::now`, `std::time::SystemTime::now`, `std::env::var`, `std::env::var_os` and `std::env::home_dir` as disallowed methods there, which the existing `cargo clippy -D warnings` rule enforces. The guard is shown to work the way the text gate was: plant a violation, see clippy go red, remove it.

