# 9: What is the test strategy, given tests must run offline and gates must be able to fail?

Type: wayfinder:grilling
Status: resolved
Blocked by: 1, 6

## Question

Project rules require that `cargo test --workspace` passes offline, that remote schemas are mocked, and that every command has stable `--json` output and the exit codes in the design's table. Prior team experience is that a check which cannot fail is not evidence.

Decide: how fixtures are laid out (namespaces under `tests/` or `examples/`), whether `--json` output is pinned with golden files, how remote schemas are mocked given the HTTP client chosen in ticket 6, how frontmatter round-trip (write, then re-read, then diff the body byte for byte) is tested given the approach chosen in ticket 1, and which deliberately broken fixtures prove each validation rule can go red.

## Inputs

Checks already required by other tickets, grouped by the mechanism each one needs. The strategy decides the mechanism per group, not each check.

1. **Pure logic against fixtures.** Slugs and anchors (ticket 4: fixtures built from GitHub's renderer, empty slugs, duplicates, a percent-decoded fragment, a case-insensitive compare, a `%` not followed by two hex digits); columns (11: a Thai line and an emoji line, each with its expected `col`); link forms (10: inline, image and reference-style links, a definition reported once with its use count, an unused definition, a duplicate label, text that looks like a link including one with a title, `<…>` and `%20` accepted, `version 1.2` not reported, an undefined `[t][ref]` not reported, definitions inside code are not definitions); query grammar (5: one fixture for each error it names); config and namespace rules (13, 7: bad namespace names, entries with `/` or `**`, dot folders, `default`, a nested `.typdoc`, an orphan state file, `name:` against `name::`, `names.shadowed`, a leftover `name` key).
2. **Deliberately broken projects.** Every validation rule needs a fixture that turns it red (project rule and this ticket's question).
3. **Write paths.** Frontmatter round trip (1: real documents, untouched lines byte-identical, `no`, `yes`, `on` and `off` stay strings, a re-read mismatch rejects the write with no fallback, anchors and tags probed first); `new`, `set` and `mv`, including `mv --renumber` (13: the old key is never issued again, refs rewritten in the form that is correct from each document, `moves` recorded, `refs.moved` naming the new key), state files per namespace, and `mv` keeping each link's written form (10).
4. **Process level.** Locks (7): a lock held by another process is not taken however old it is; two processes racing get one winner and the other exits 4; an interrupted process leaves no lock; a lock removed and re-created by another writer is not removed by us; a reader never sees half of `lock.json` while `pull` writes it.
5. **Environment isolation.** The order in which `imports.json` is found (7: five cases); no test reads the real home directory; an unset or empty `${ENV}` in an import path; `imports.absent` at `error` gives exit 2; `TYPDOC_DIR`, `TYPDOC_NAMESPACE` and `--namespace` scope (13).
6. **Network.** An in-memory `Fetch` for most tests; the real adapter and the shipped binary against a plain-http loopback server; the timeout, the response size limit, the redirect limit and the proxy variables; a vendor copy whose hash differs from its name and a missing copy are config errors (6, 7).
7. **Shells.** Every documented example goes through sh and bash and reaches typdoc byte for byte identical; an unquoted case must go red; the tests and the document share one list of examples; a shell the document lists but the machine lacks makes the suite red (7).
8. **Across all of it.** Tests run offline; `--json` is stable for every command; the exit codes in the design's table; macOS is not claimed as supported until a macOS run exists (see macOS below); a fixture copied from a real document into this public repository is reviewed first (project rules, 7, 1).

## Answer

### Fixtures

Decided: one top-level `fixtures/` directory, read by every crate, with `valid/<name>/` (complete projects that validate clean), `broken/<rule>/` (each names the rule it exists to prove and carries the rules it is expected to trip) and `roundtrip/` (documents for the write tests). `examples/` (samples for users) stays separate. Reasons: the tests of `typdoc-core` and of the CLI need the same fixtures, and two copies would drift apart; naming `broken/<rule>/` after the rule shows at once which rule has no fixture; keeping `examples/` apart lets a sample for users change without breaking a test that needs fixed values.

What carries this decision:

1. **Two coverage tests, one for each direction.** Every folder in `broken/` names a rule that exists, so an orphan folder cannot look like coverage. And every rule that exists has at least one fixture in `broken/`: a rule with no fixture has never been shown to fire, which cannot be told apart from a rule that is silently broken. The second test goes red the moment a rule is added without a fixture.
2. **A fixture must go red for the reason it claims.** Red alone is not enough: if the fixture for one rule happens to trip another rule as well, the test passes on the wrong evidence. Each broken fixture carries the exact set of rules expected to fire, and the test compares that set exactly, not only a non-zero exit. When a later change makes a second rule fire, that is loud, not silent.
3. **The examples are validated as a user would receive them.** A test runs `validate` on every project in `examples/`, with the default configuration that reaches a user (the example's own config, default rule levels, no test-only flags or overrides). Otherwise it proves the examples work under conditions no user has.
4. **A fixture copied from a real document passes the public-text gate** as well as the review decided in ticket 1. They are two separate requirements and both apply.

Default: the expected set of rules is written by hand and never generated by running the tool, since a generated expectation certifies whatever the tool does. The coverage tests iterate the same registry of rules that the validator uses, so the two lists cannot drift apart.

### Fixtures and packaging

Decided: `fixtures/` and `examples/` stay at the repository root and are not shipped in any package. The test suite runs from a checkout of the repository.

Verified with `cargo package --list` on a throwaway workspace whose crate sits under `crates/` and whose fixtures sit at the root: only files inside the crate's own directory are packaged (`Cargo.toml`, `src/`, `tests/`), the root directories are not, so a packaged `tests/` reads a `../../fixtures` that does not exist in the package. The mechanism: in a repository with a single crate the package root is the repository root, so root directories are packaged; in a workspace whose crates live under `crates/` the package root is the crate's directory and the repository root lies outside it.

When fixtures cannot be found, the loader fails loudly with a message saying the suite has to run from a checkout of the repository. It is not a bare file-not-found and it is not a skip, so someone running the tests from a tarball is told the reason at once instead of guessing what they configured wrongly.

Not decided here: whether tests must run from a published package, and so whether crate manifests exclude `tests`. That belongs to the distribution decision (see the map). Excluding `tests` now would settle distribution by side effect, and a published crate would look as if it had no tests at all, which is untrue and misleading. What is decided now is where fixtures live, not the shape of a package.

### What needs a broken fixture

Decided: every rule and every config error needs a fixture in `broken/`; the outcomes of a command (exit 1, 3, 4, 5 and 6) do not, and are tested by ordinary CLI tests that check the exit code and the `--json` output.

The criterion, meant to be applied without a list: does the thing read a project and report a defect in it? If it does, it needs a fixture in `broken/`, because a check that reads a project can go silent without anyone noticing, and a check with no fixture has never been seen to fire. If it is what a command does when it meets a certain state (a false `--if`, a lock that is held), it is a CLI test. Whoever adds a new kind of report can then say on which side it falls without asking.

The line must not dissolve. Some rules need more than files (`imports.absent` needs an environment variable that is unset or empty; `collections.overlap` needs two files that collide, which are only files). So a fixture may carry declared values and nothing else: environment variables, and the name and arguments of the command to run. Never a setup script. A case that needs a script is not a fixture of a project; it belongs to the CLI tests. Otherwise fixtures that carry commands and setup would turn into command tests and the broader option (every outcome needs a fixture) would return through the side door.

Config errors get ids. The design listed them as plain sentences, so a caller that met one had to read the message; ids let it branch on them, as the exit codes do for a rule, and let each one have a fixture. About twenty ids are added to the design, in a table under Config errors, and `--json` carries them in `details[].rule`. This is what the design wanted from the start, not a cost of the test strategy.

Defaults: the only substitution a declared value may use is a placeholder for the fixture's own directory (an import path has to point at a neighbouring fixture); a config id is carried in `details[].rule` alongside rule ids (a single registry), and `path` names the configuration file it is about (ticket 15 replaced `doc` and `file` by `path`).

Exit 1 covered three unrelated things (not found, bad arguments, I/O), which says little to a program and made a test that asserted exit 1 prove little. Ticket 14 splits it.

### `--json` output

Decided: the `--json` output of every command is pinned with golden files. A golden is compared as parsed JSON (whitespace and key order do not matter; array order does, see below), and each command also has hand-written assertions on its load-bearing fields. The safeguards are structural, not a matter of discipline: anything that relies on people being careful breaks on the day someone is in a hurry, which is also the day the most output changes.

1. **Hand-written assertions live in files the golden-generating command cannot write.** This matters most. If a whole set is regenerated, the hand-written assertions are what would catch it, but only if they are not regenerated along with it; a net that is updated by the thing it watches is not a net. The generating command touches golden files only, and no path lets it write an assertion file.
2. **The generating command has no mode that regenerates everything.** It must be told which golden to regenerate. Regenerating one is easy; regenerating all of them is a visible chore, not one key. The day the output of the whole system changes is the most dangerous day, and the day one key would be pressed.
3. **The clock is injected as a constant, not scrubbed out before comparing.** If time values were removed, they would never be pinned, and a wrong format (an epoch instead of ISO, a lost time zone) would leave the golden green. Injecting a fixed value pins the format too.
4. **Which arrays guarantee an order is declared first.** If the order of an array is an accident of the implementation, pinning it gives either a flaky test or an accident frozen into a contract. The design states, per array, whether order has a meaning, and the golden compares accordingly: an array with no guaranteed order is sorted before comparing.

A golden regenerated from the tool's own output certifies whatever the tool does, including what it does wrong. The same principle stands behind the hand-written expectations of the broken fixtures; point 1 is what makes it effective rather than intended.

Defaults: the function that regenerates goldens refuses to write any path outside a `golden/` directory, and a test proves it by trying to make it write an assertion file, so the guard is shown to work. The clock is injected at the library boundary (a `Clock` passed into `typdoc-core`), and the shipped binary has no environment variable that fakes the time: such a knob could write a false `created_at` into a real document. So the outputs of commands that stamp the time (`new`, `set`, `pull`) are pinned through `typdoc-core` with the injected clock, and the binary-level goldens cover output that does not depend on the current time.

For the contract: the per-array statement of ordering (point 4) is part of the exact `--json` shapes, which the design fixes under JSON output (ticket 15).

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

Decided: `deps` is the only way a module reaches the outside world. If any module can pick up a real client, the real clock or a global on its own, that route escapes testing with no signal and nobody finds out until it breaks. It follows the principle stated for the golden files: the net has to sit outside what it watches. The list of what `deps` holds below is completed under Failing a write in the middle: the filesystem is the channel typdoc reaches the outside world through most, and it was missing.

Defaults, so that this is enforced by structure and not by care: `ureq` is a dependency of the binary crate only, so `typdoc-core` cannot reach a real client at all. The clock, the environment and the home directory are reached in `typdoc-core` only through `deps` (a `Clock` and an `Env`; the location of `imports.json` in ticket 7 is tested with a fake environment for that reason), and `clippy.toml` lists `chrono::Utc::now`, `std::time::SystemTime::now`, `std::env::var`, `std::env::var_os` and `std::env::home_dir` as disallowed methods there, which the `cargo clippy --workspace --all-targets -- -D warnings` rule enforces (`--all-targets` is what makes clippy read test code; see Environment of spawned processes). The guard is shown to work the way the text gate was: plant a violation, see clippy go red, remove it.

### Frontmatter round trip

Decided: two layers, and the second is cut by a criterion instead of by difficulty ("hard cases" is a line nobody derived from anything).

**Layer one: invariants, over every document and every operation, with no expected file.** It carries almost all the weight.

- Frontmatter lines that the operation does not touch are byte-identical, and so is the body.
- Reading the result back gives exactly the intended change, no more and no less.
- A no-op write (read, then write with no operation at all) returns the file byte-identical. It is the cheapest check, it covers every fixture for free, and it catches the worst failure mode: a writer that reformats on load.
- Type preservation is not only `no`, `yes`, `on` and `off`. The research found `1e3` becoming 1000.0 and `1.10` becoming 1.1 as well, all one class and a real concern of typdoc because typing comes from the schema. `0755`, dates and datetimes belong to this layer too.

**Layer two: whole expected files, only for a case that meets one criterion.** Whoever writes the expected file must be able to write it from the design without running the tool. If they cannot, it is not a promise, and there are only two ways out: make it a promise (write it into the design and the trait, after which the file can be written) or drop the case. There is no third way. Running the tool and copying its output into the expected file is the golden-from-the-tool option dressed as this one, and it breaks the rule already stated in this ticket, that the tool's own output certifies whatever the tool does. The criterion has no number and no line drawn by anyone, and it says at once on which side a new case falls, the same shape as the criterion that cuts `broken/`: does it read a project and report a defect.

**Where keys go: decided.** An existing key keeps its position and is never re-sorted; the design already promised that. A key that is added goes at the end of the block, always. The design had said nothing about this and now says it. The reason is that it is one fixed rule, and whoever writes the expected file can apply it without opening the tool; "sorted by schema" would mean computing a position in a file that is probably not in schema order already, with a result nobody could predict. The quote style of a new value is not a promise, so it has no expected file and no test. The design sentence that said writes preserve key order and existing style "where possible" covered both with one qualifier; it is split. Key order has no case where it cannot be kept, so it is promised in full and can be tested. Existing YAML style stays a best effort, and because it is a best effort it has no test.

Observation: under this criterion, whole-file expectations exist only where the resulting text is fully determined without a choice of style (numbers, booleans and similar). For an added key, position is a hand-written assertion, not a whole file. Layer one therefore does most of the work, which is the point of it.

**The guard test comes first in this group, not last.** An editor double returns damaged text of the kind the research found (`note: newb: 2`, from replacing a block scalar), through the trait of three operations, and the test confirms that the re-read check rejects the write. By the rule this ticket set for `broken/`, it must go red for the reason claimed: the test asserts that exact error, not just a failure. The reason for the order is that `yaml-edit` 0.3.2 was two days old when the research was done and has a single maintainer; this guard is the one thing between that crate and a user's real documents. The re-read uses the reader (`yaml_serde`) and not the writer's own parser, so the net sits outside what it watches. That is the same rule as the golden files and as `deps` in this ticket: one rule, not three coincidences.

**The list of documents must not be trusted to be complete.** A list that someone thinks of finds only what someone thought of. The corpus must contain real files, as ticket 1 decided, because real files carry formats nobody listed. This repository's own files carry no frontmatter (its tickets use inline lines), so the real documents come from the public `chief` repository (26 files with frontmatter) and the public `typmem` repository (23), counted on 2026-09-20, copied after review and after the public-text gate. Hand-made cases are added as well: several quote styles, flow lists, block scalars, comments, CRLF, a BOM, Thai text, a file with no frontmatter, `---` inside the body, no final newline. Layer one catches formats nobody thought of; layer two catches only what someone listed.

**Anchors and tags:** `yaml-edit` has never been tested on them (ticket 1). That is not left to disappear: it is not known, and it is to be proven in the prototype for the contract, under the same rule as the proxy variables.

Rejected: expected files that the tool produced and that were then kept as goldens.

For the contract: the error for a write rejected by the re-read check has its own id, so that a test can assert it exactly.


### Process level: locks and signals

Decided: two layers, and nothing in the shipped binary exists for tests. A forced kill and a power loss leave a stale lock by design (ticket 7), so the signal handler is best effort and no test claims more than that.

1. **The library, with no process.** A test holds a lock guard in code, removes the lock file and creates another in its place (a different inode), then drops the guard. It asserts that nothing is removed, that the report says the lock was removed by someone else, and that the new file is intact. The case of a link count of zero is tested the same way. It runs on a real filesystem, so the device and inode comparison is the one that ships.
2. **The shipped binary, in a state the test makes.** `mv --renumber` holds the locks of two namespaces and takes them in name order (tickets 7 and 13). The test creates the lock of the later namespace, `b.lock`, itself and runs `mv --renumber` from namespace `a` to namespace `b`. The binary takes `a.lock` and then waits for `b.lock` until `--lock-timeout`: a state that lasts long enough, is fully determined, and needs no hook. The test waits until `a.lock` exists (it watches the file, it does not sleep for a guessed time), sends SIGINT, and asserts three things: `a.lock` is gone, `b.lock` is still there with the same device and inode, and the process ended by the signal. A second run does the same with SIGTERM. This observes what the handler does, not a shadow of it: the process really holds a lock when the signal arrives, the build under test is the one users receive, and the binary carries no code for tests.

Each property has one observation. A handler is installed in the shipped build (without one, `a.lock` stays behind). Cleanup removes a lock the process holds. A lock the process never took is not removed (`b.lock` stays).

What the scene needs from the design, stated as a requirement and not as a result: no program exists yet, so this scene has not been run and is not a measured fact. It is a requirement placed on the design so that it can be tested this way. Two things follow. First, every command must take its locks through one acquisition path, a single lock type whose guard owns the cleanup. `new` and `set` hold one lock and never wait after taking it, so no state a test can make lets a signal arrive while they hold it; they are covered only because they use the code `mv --renumber` uses, and layer 1 exercises that guard. Second, the fallback is decided now, so that whoever reaches it in the contract does not have to think again, and it comes in two pieces because the scene serves two needs at once: seeing the cleanup happen, and proving that the handler is in the shipped binary. If the contract finds that the scene cannot be made, or that commands take locks in more than one way, a Cargo feature that pauses the process just after it takes a lock brings back the first need only, and it is a build that differs from the shipped one. The second need then has to be read from an exit status that tells a handler from none, and a process that raises the signal again cannot be told from one with no handler (see the probe below). So the interrupted process exits by its own exit with 128 plus the signal number, and the design's table of exit codes gains a row saying that an interrupt exits 128 plus the signal number. Project rule 5 needs no change, because it points at the design's table (ticket 14). The two pieces go back together.

Rejected: a Cargo feature that pauses the process after it takes a lock. It puts code for tests in the shipped binary, or tests a build that differs from the shipped one, and the scene above reaches the same property without either. Rejected: sending signals at random moments and counting how many rounds prove enough; a proof that depends on a number nobody derived is the mistake this ticket avoids elsewhere, and a signal test that goes red now and then ends up switched off, not investigated. Rejected: reading the exit code to tell a handler from none (see the probe below: a handler that cleans up and raises the signal again looks exactly like no handler); the scene observes the lock file instead.

The tests observe results only: whether a lock file exists, its device and inode, whether the process exited or was killed. None looks at how the handler is built, because the crate that catches signals is chosen in the contract (ticket 7) and any choice must leave these tests valid.

### What an interrupted process ends with

Decided: on SIGINT or SIGTERM typdoc removes its own locks, restores the default action for the signal and raises it again, so it ends by the signal. A caller sees a process killed by a signal, not a code from the exit table. Reasons:

1. A shell puts 128 plus the signal number into `$?` when a child ends by a signal, so a shell user sees 130 or 143 with nothing added to typdoc.
2. A caller that uses a process API gets more from a death by signal than from a code: it can tell which signal ended the process and that it was not typdoc's own decision.
3. Project rule 5 stands. A process ended by a signal has no exit code; the table of exit codes describes the outcomes of a command, not every way a process can end.
4. Code 1 then covered three unrelated things (split later, ticket 14), and a fourth would have made a test that asserts exit 1 prove less.

`docs/design.md` gets one sentence in Exit codes and errors that says this.

Probe of 2026-09-20 (Linux, Python 3.14.4 `subprocess`, GNU bash 5.3.9): a child with no handler that is killed by SIGINT reports return code -2 (killed by signal) and by SIGTERM -15; a child whose handler exits by itself with 7 reports 7; a child whose handler restores the default and raises the signal again reports -2 for SIGINT and -15 for SIGTERM, exactly as if it had no handler; bash reports 130 for a foreground child killed by SIGINT and 143 for one killed by SIGTERM; `sleep 5 &` in a non-interactive bash ignored SIGINT and finished normally, so a test process must be started directly and never as a background job of a shell script. Not verified: macOS.

Observation: an interrupted process ends by the signal, so it writes no `--json` error object; the object described under Exit codes and errors is for a command that ended with an outcome.

For the contract: what an interrupted write leaves behind (the design promises the target file is whole, and says nothing about a temp file); the instant between creating a lock file and registering it for cleanup, which no test can reach; and an interrupt that arrives while the process is working and not waiting.

### Environment of spawned processes

Decided: every process a test starts begins with an empty environment and receives only the variables the test declares, through one helper. A list of variables to override finds only what someone thought of, and the environment differs from one machine to the next: a developer with `HTTPS_PROXY` set changes the result of a fetch, and the probe in this ticket showed that the variable applies even to an `http://` URL.

Defaults:

1. `HOME` is a new empty directory for each test, so no test reads the real home directory. `PATH` is declared in the helper as a short constant written out there and never copied from the machine that runs the suite: `PATH` decides which binary is run, so copying it would open a hole at the place this rule closes. Every other variable is declared by the test that needs it (`TYPDOC_DIR`, `TYPDOC_NAMESPACE`, `TYPDOC_CONFIG_DIR`, `XDG_CONFIG_HOME`, a proxy variable).
2. The helper is the only place that starts a process. `clippy.toml` lists `std::process::Command::new` as a disallowed method, and the helper carries one narrow `#[allow(clippy::disallowed_methods, reason = "...")]` on the function that calls it, with the reason written out; never on the file or the crate.
3. This guard reaches test code only if clippy reads test code, and the command in project rule 1 did not: `cargo clippy --workspace -- -D warnings` lints library and binary targets and skips `tests/` and `#[cfg(test)]` modules. The command is now `cargo clippy --workspace --all-targets -- -D warnings`, changed in both places in `.chief/project.md` (the command block and rule 1). It is changed now, while the repository has no code: turned on later, it shows every warning hidden in test code at once, and a check that turns everything red ends up switched off, not fixed. The `disallowed-methods` list for `typdoc-core` (clock, environment, home directory) now also fires in that crate's tests, which is intended.
4. The guard is shown able to fail by planting a violation in test code only. A violation planted in `typdoc-core` would go red, and everyone would be reassured by the old guard, which already reads that code, not by the new one.

Probe of 2026-09-20 (cargo and clippy 1.96.0, a throwaway workspace with a library crate that has a `tests/` directory and a `#[cfg(test)]` module, and `disallowed-methods` naming `std::process::Command::new`): a spawn in `tests/` and a spawn in a `#[cfg(test)]` module both gave exit 0 under `cargo clippy --workspace -- -D warnings` and exit 101 under `--all-targets`; a spawn in library code gave 101 under either; a helper with the narrow allow gave 0; a second spawn without the allow next to it gave 101.

Locale: a program that uses only Rust's standard library, started under `env -i`, `LC_ALL=C` and `LC_ALL=C.UTF-8`, gave the same result each time for a Thai argument (18 bytes, 6 scalar values), for writing and reading a Thai file, and for a Thai file name being valid UTF-8. So an empty environment needs no locale variable for code that uses the standard library only. `th_TH.UTF-8` is not installed on the machine where this was run, so a run under it says nothing more than an unset locale and is not counted. Not verified: the shell harness (a shell may use the locale for multi-byte text), typdoc's own dependencies (nothing is built yet), macOS.

### Shells: a listed shell that is missing

Decided: a shell that the document lists and the machine lacks makes the suite red. It is not skipped. What is red is not "this machine has no such shell" but "the document claims a shell and nobody has shown it", and there are two ways to clear it: install the shell, or remove its row from the document, which is what the rule in ticket 7 already says. The message the suite prints names both ways, so that someone who cannot install a shell does not read it as being stuck when a correct way out exists.

The reason beyond the visible one: a skip that is normal on developer machines is seen every day until it reads as normal, and on the day a CI runner changes its image and skips by itself, the signal looks like the one seen every day. A warning people have got used to is not a warning.

Rejected: a variable that turns the skip on for a machine. It is a switch that works only while whoever sets it is careful.

Current state: zsh is not installed on the machine where this was decided, and installing it was not attempted, so the zsh row is removed everywhere the shell list appears (the design, tickets 5, 7 and 9, and the map). The v1 list is sh and bash, both present on that machine. The consequence is written into the design in one sentence: zsh is the default shell on macOS, so the most common way to run typdoc on macOS is not a covered shell. This is not a reduction of scope. The document already claimed zsh with no run behind it, and it now says what has been run. It is a known gap on the map, and the row returns together with a test that runs zsh, not on its own.

### Shells: which examples run, and what they must give typdoc

Decided: the examples are taken from the design document itself, as one list, with no marker in the document. An example that contains a character a shell can change carries a value declared by hand in test code: the argument list typdoc must receive. The test runs each example through every listed shell, with a stand-in `typdoc` that writes the arguments it received to a file, and compares them with the declared value.

Why a declared value is needed, and why nothing computes it. Ticket 7 requires that an example with its quotes left out on purpose turns the test red. A test that only compares the shells with each other cannot do that: every shell changes an unquoted `\,` or `*` in the same way, so they agree and the test stays green (run 2026-09-20 below). Some statement of what the example should give typdoc is needed either way, and once it exists a program that splits words the way a shell does has nothing left to do. Writing one means reimplementing POSIX word splitting to judge the real shells: code the project would not own and could not check except with a test of its own. The statement is already in the document as intent (Quoting in the shell: give the values to typdoc exactly as written); the declared value is that sentence applied to one example.

How it works:

1. **The list is taken from the document.** Every fenced line and every inline code span that begins with `typdoc`, with `TYPDOC_` followed by a name and `=`, or with `--` (an argument fragment) is an example. The last form is needed: the three examples the Quoting in the shell paragraph is built on (`--where 'title=Cosmos\, or SQL'`, `--namespace '*'`, `--namespace 'chief::*'`) begin with `--`, so a rule that looks only for `typdoc` and `TYPDOC_` would not see them and would still report that everything is covered.
2. **Declared values live in test code, keyed by the text of the example**, never in the document: the repository is public and scaffolding for tests does not belong in its prose. The criterion for a declared value is the one used for whole expected files in the frontmatter round trip: whoever writes it must be able to write it from the design without running anything. That holds here because the design states the intent in words.
3. **Which examples need a declared value** is decided by complement, not by a list of special characters. An example needs one if any word contains a character outside a short safe set: letters, digits, `_ - . / : = , @ % +` and the space. The alternative is a note that says it is a template, with the reason (for example `typdoc new <CODE> "<title>" [--set k=v ...]`, where `<CODE>` is a redirection in a shell). A list of special characters finds only what someone thought of, and the first list considered missed `!`, `?`, `#` and braces, all present in or allowed by the document; `!` is on the design's own list of characters that single quotes must pass. A complement fails on the safe side: a character nobody thought about demands a declaration. This is the mistake the public-text gate's list of known phrasing has already made once: a list of what is dangerous finds only what someone thought of, while a list of what is safe sends everything else to a decision. It is one rule, not two events. Braces are not hypothetical: `kind={a,b}` gave two words in bash and one in sh (run below).
4. **Run plus skipped equals total, in both directions.** Every example is either run or named as a template with a reason, and the two counts must add up to the total found in the document, so an example added later that nobody declared turns the test red and its text is printed. A declared value whose example is no longer in the document is also red and named, so an orphan declaration cannot look like coverage; this is the same pair of directions as the coverage tests for `broken/`.
5. **The example with its quotes removed must go red against the declared value**, not against the other shells. For each declared example the test strips the quotes, runs the result through every shell, and requires that the arguments differ from the declared value. Because this is generated from the text, a new example gets it without anyone writing it.
6. **The unquoted case runs in a directory that contains files.** An unquoted `*` in an empty directory reaches typdoc unchanged, so a variant run there cannot go red; the directory must hold files that the star matches.
7. **Every code span in the Quoting in the shell paragraph is classified.** Point 1 finds examples by their form, so the counts can only add up over what the form shows, and a form that is missed is missed by the total as well. This paragraph is where the promise about quoting lives and where the gap was found, so a narrow rule closes it: each code span in it is either run with a declared value or named with a reason (a template, the name of a flag, a single character). It has 13 spans today (four flag names, four examples, five single characters). The rule is not widened to the whole document.

Run of 2026-09-20 (Linux, GNU bash 5.3.9 and dash as `sh`, a stand-in `typdoc` printing each argument in brackets): with the quotes, both shells gave `[--namespace][*][--where][title=Cosmos\, or SQL]`. Without them, in an empty directory, both gave `[--namespace][*][--where][title=Cosmos,][or][SQL]`, identical to each other; in a directory holding `a.md` and `b.md`, both gave `[--namespace][a.md][b.md][--where][title=Cosmos,][or][SQL]`. So the shells agree with each other on the unquoted form, and the star changes only where a file matches. `kind={a,b}` gave `[kind=a][kind=b]` in bash and `[kind={a,b}]` in `sh`. A `PATH` set to the stand-in's directory alone made the shells themselves unfindable, so the helper's constant `PATH` names both the stand-in's directory and the directory of the shells. Not verified: macOS, and the stand-in as a shipped test helper (no program exists yet).

The cost, stated plainly: the values are written by hand, on the order of ten to fifteen by a rough count (the count is an estimate from a pattern search over the document: 58 examples begin with `typdoc` or `TYPDOC_`, and 60 fragments begin with `--`, most of them plain flags), and the number grows with the document. That is the price of not maintaining a word splitter that has to be right forever and that nobody could check. The estimate does not carry weight: point 4 makes the counts agree by themselves, whatever the true number is.

Rejected: a word splitter written for the test, for the reasons above. Rejected: a marker in the document that says which examples run, because it puts scaffolding for tests into a public document and an example that is added without the marker is silently never tested. Rejected: a list of examples kept in test code, because it becomes a second list that has to be kept equal to the document, against the requirement that the tests and the document share one list.

### macOS: what the document claims

Decided: the design says what v1 supports, and macOS is not yet on that list. It says that v1 supports Linux, that the code is written for macOS as well, and that macOS is not yet a supported platform. Support is claimed for what has been run, the rule already applied to shells. The design states the promise and nothing about tests; why there is no evidence yet lives here and on the map.

What does not change: the code still targets Unix on both platforms, the Windows build still fails with a message, and the platform default for `imports.json` is still named for both. What changes is the claim, not the target. Nothing waits on any tool or service for this: the sentence is true today, so the design can say it today.

Rejected: keeping "supports Linux and macOS" and making a macOS run a condition of release, because it moves the discovery of a problem to the day before release, when it costs most. Rejected: dropping macOS from v1.

Open, as its own item on the map: the default filesystem on macOS does not distinguish upper and lower case (general knowledge, not run on this machine, so not verified). Two keys that differ only in case would then collide on macOS and not on Linux, and a `mv` that changes only the case is a case of its own. This is not a gap in testing; it is a mechanism that makes the program behave differently on two platforms, and the contract decides what typdoc does about it.

Parked: a macOS runner, and running the public-text gate in CI. Neither is waiting on an answer and neither blocks a ticket; the decision is not raised again until someone takes it up.

### The suite runs offline, and shown able to fail

Decided: a tripwire. Every process the spawn helper starts gets `HTTPS_PROXY` and `HTTP_PROXY` pointing at a listener on loopback that turns the test red, with the target it was asked to reach, when anything connects to it. `NO_PROXY` names the address the harness gives to test servers, so the loopback server of a test goes straight through. It needs no special rights and runs on every platform. A gate that runs only on some machines is a gate that depends on who has the right to run it, which is not a gate.

1. **The harness gives every test its server's address.** No test writes an address of its own. `NO_PROXY` was verified only for a host named exactly (probe below), so a test that wrote `localhost` or `::1` where the harness wrote `127.0.0.1` is not expected to match and would run into the tripwire (an inference, not run). That has two outcomes: time lost looking for the cause, and, worse, someone widening `NO_PROXY` to get past it and leaving a hole with no signal. A list of loopback spellings would find only the ones someone thought of; handing out the address needs no list.
2. **The test that proves the tripwire asserts that it recorded the connection.** The binary is asked to fetch `http://schemas.invalid/x`, which has no pin, and the assertion is that the tripwire recorded a connection naming `schemas.invalid:80`. The assertion is never "the fetch failed": that name cannot be resolved anyway, so a failure would be green with no tripwire at all. Red alone is not enough; it must be red for the reason claimed, the rule used for `broken/`. The tripwire sees the host and port of the request for a tunnel, not the path of the URL (probe below). The same test shows that the proxy variables still work with the version of `ureq` in use, so a change in a later version turns it red.
3. **A test that needs a proxy of its own** (the proxy tests under Remote schemas) declares it, replacing the tripwire, like any other variable.

What this does not cover, stated as it is:

- It covers processes the helper starts. A test that runs the real adapter inside the test process (Remote schemas, layer two) does not pass through the helper, and nothing checks that it connects only to the listeners it creates. Open, for the contract.
- It covers a client that follows the proxy variables, which is `ureq`, probed on 3.4.2 for `HTTPS_PROXY`, `HTTP_PROXY` and an exact `NO_PROXY` host only. A socket opened directly does not go through it.
- The guards built into the structure cover the library only: `typdoc-core` has no `ureq`, and a `clippy.toml` entry for `std::net::TcpStream::connect` in it is possible (not tried). The binary relies on network access being confined to the `Fetch` adapter, and the tripwire is what watches that adapter. Nothing checks a socket opened elsewhere in the binary crate, and this ticket does not claim it.
- Cargo's own registry access while building the suite is a separate matter and is not decided here.

Rejected as a gate, kept as an extra for machines that allow it: a network namespace that holds only loopback, which the kernel enforces and which covers every kind of client. It cannot be the gate on the machine where this was decided. Run on 2026-09-20 as an ordinary user: `unshare -rn python3 …` printed `unshare: write failed /proc/self/uid_map: Operation not permitted`. On the same machine `kernel.unprivileged_userns_clone` is 1, `kernel.apparmor_restrict_unprivileged_userns` is 1 and `user.max_user_namespaces` is 65176; seccomp is not active and `NoNewPrivs` is 0 for the shell. The AppArmor setting is consistent with the refusal, but the cause was not proven, and whether a profile or a change by an administrator would allow it was not tried. The reason for rejecting it is written down so that the next person to propose it does not have to find it out again.

Rejected: structure alone, with no dynamic check. A check that cannot fail is not evidence.

Probe of 2026-09-20 behind points 1 and 2 (the proxy probe recorded under Remote schemas): with `ureq` 3.4.2 the proxy named by `HTTPS_PROXY` or `HTTP_PROXY` received `CONNECT schemas.invalid:80` for an `http://` URL, a tunnel is requested even for plain http, and `NO_PROXY` naming the host exactly bypassed the proxy. Not verified: `NO_PROXY` with `localhost`, an IPv6 literal or a pattern, and precedence when several variables are set.

### Failing a write in the middle

Decided: a narrow seam in `deps` around the one function that writes a file by temp file and rename. Every file typdoc writes goes through it (documents, `state/<namespace>.json`, `lock.json`, the copies in `vendor/`), so there is a single place to make a write fail.

The reason is not that tests find it convenient. `deps` is the only way a module reaches the outside world, and the list it was given (`Fetch`, `Clock`, `Env`) left out the channel typdoc touches most, the filesystem. A principle that names the outside world and omits its main channel contradicts itself, so the seam closes a gap the principle left open. That it also makes a failing write testable follows from that; it is not why the seam exists.

What is behind the seam and what is not, as a line drawn on purpose:

- Writes are behind it. Failure injection needs only the write side.
- Reads are not. They are controlled by fixtures, and putting them behind a seam would collide with the lock tests, which need the real inode of a real file.
- Lock files are not. They are created with `O_EXCL` and removed, never renamed, so they are outside the seam. Injecting a failure into the seam therefore never touches a lock left behind, and no test built on it claims to.

The double can fail the Nth write, or fail after the temp file is written and before the rename. Each test asserts that the count was actually reached before it draws any conclusion: if a fixture ever holds fewer than N files, the injection never happens and the test would pass with nothing tested, so it must be red instead. This is the same family as the assertion on the tripwire.

Rejected: making a write fail with directory permissions. As root, permissions stop nothing and a test meant to fail is green with no signal (general knowledge; not run, since the shell here is an ordinary user). And it fails at another layer: a file that cannot be opened, not a write that got part of the way and broke. Rejected: leaving failure in the middle untested until the contract decides whether a command is all or nothing across namespaces. The tests below are written as invariants that refer to what the contract will decide, so they do not wait for it.

The guard is a `clippy.toml` in the directory of `typdoc-core` that lists `std::fs::rename`, `std::fs::write`, `std::fs::copy`, `std::fs::File::create`, `std::fs::File::create_new` and `std::fs::OpenOptions::open` as disallowed methods. One narrow `#[allow(clippy::disallowed_methods, reason = "...")]` sits on the function that writes through the seam and one on the function that creates a lock file, each with its reason written out; never on a module or the crate. The same command, `cargo clippy --workspace --all-targets -- -D warnings`, enforces it.

Probe of 2026-09-20 (cargo and clippy 1.96.0, a throwaway workspace): reads alone gave exit 0; `rename`, `write` and `copy` each gave 101, also when reached through `use std::fs`; `File::create` and `File::create_new` gave 101; `OpenOptions::new().write(true).create_new(true).open(..)`, the way a lock file is made, gave 101 and so needs its own allow; an allow on one function gave 0 and a second function next to it without the allow gave 101; a `write` in `tests/` alone gave 101. With the ban in the `clippy.toml` of the library crate, a `write` in the binary crate gave 0 and one in the library crate gave 101, so the ban covers that crate only, and a test inside `typdoc-core` that writes a fixture with `std::fs::write` gave 101: such tests go through one test helper with its own narrow allow, or write through the seam. Not tried: `std::fs::remove_file` and `create_dir_all`, which the lock module and `new` may need.

Limit, stated as it is: like the other structural guards, it covers the library. The binary crate is outside it, and nothing checks a write opened there.

### Write commands, state files, and what every write must keep

Decided: the write commands (`new`, `set`, `mv`, `mv --renumber`) are tested in four ways. Each one has a reason to exist that another does not supply.

1. **Cases whose result the design determines, with expected values written by hand.** The criterion is the one used for whole expected files in the frontmatter round trip: whoever writes the expectation must be able to write it from the design without running anything. For `new`: the number is the larger of the highest existing number in the collection within the namespace and the collection's `last`, plus one, and `last` is updated (a case for each side being the larger); a number is never issued again after its document is deleted; a document created by hand with a higher number is respected; a gap is harmless; the file is named from `match`; defaults and `auto` fields are filled from the injected clock; a scope that holds more than one namespace exits 1 with the choices. For `set`: `--if` false gives 3 and writes nothing; `k=` removes a field; writing an `auto` field directly is a validation error; `auto: update` fields change only when a value changes; `auto: create` never changes; a value is split on `,` only for array fields. For `mv`: refs are rewritten in the form that is correct from each referencing document (bare key, `name:` prefix, `name::` prefix, relative path); body links keep their written form (`<…>` stays, `%20` stays, a definition is rewritten once, an ignored duplicate is left alone); `moves` and `refs.moved` are recorded; a coded document cannot leave its namespace except by `--renumber`, and the failure suggests it. For `--renumber`: the number comes from the destination's `last`, the source's `last` never goes down, and the old key is never issued again. Whole expected files are used where the resulting text is fully determined by the design (a rewritten link, a rewritten ref); for an added key, position is a hand-written assertion, as decided for the round trip.
2. **A held lock is what shows that a command takes its lock.** For each writing command the test creates the lock file itself, runs the command, and asserts exit 4, that nothing under the tree changed, and that the message carries the path, pid, host and age. `mv --renumber` holds the second of the two namespaces, as in the signal test. A command that skips its lock turns this red every time, so it can be shown able to fail by planting exactly that. Two processes racing for the same collection is kept as a smoke check only and is not offered as evidence: whether a lost update shows in one run depends on timing, and the number of rounds needed cannot be derived, which is the reason random signals were rejected. The double behind the seam adds one check that is not a race: at each write it asserts that the lock file of every namespace being written exists. Limit: nothing shows that the read of `last` happens inside the lock, since reads are not behind the seam; the smoke check may catch a violation and proves nothing.
3. **Failure in the middle, through the seam.** For `mv --renumber` across two namespaces and for `set`, fail the Nth write and the write between temp file and rename, for every N up to the number of writes the command makes, and assert the invariants below. The count of writes is asserted, as above.
4. **Invariants after every write command, whether it succeeded or a write was failed.** They carry most of the weight, as in the round trip, and need no expected file: (a) every file is byte-identical to its old contents or is a complete new version, never a mixture; (b) `validate` reports no defect other than the ones the command reported; (c) the `last` of a namespace never goes down, and a key that was issued is never issued again; (d) a namespace the command did not write has state and files byte-identical. What a failed command leaves behind, whether it is all or nothing across namespaces and whether a temp file may remain, is decided by the contract (map, Not yet specified); until then the tests assert (a) to (d) and not more, and a decision by the contract adds an assertion.

State files: `state/<namespace>.json` is written by `new` and, for the destination namespace, by `mv --renumber`; `set` and a plain `mv` never write it. It is written through the same seam and by the same rule as any file, and invariant (d) is what shows that a namespace a command did not issue a number in keeps its state file byte-identical. How a missing record is treated is decided in the next section.

### Across all commands

Decided: two coverage tests in both directions, the same shape as for `broken/`. Every one of the nine commands in the Commands section has a `--json` golden, and every golden names a command that exists. Both are read from the registry of commands the CLI itself uses, so those two lists cannot drift apart, and the registry is checked against the headings of the Commands section, read from the document as the shell examples are, so a command added to the CLI with no entry in the design is red. Every exit code in the design's table is produced by at least one CLI test that asserts the code and, for a failure, the error object in `--json`; a code with no such test has never been seen to occur. Ticket 14 split exit 1 by what the caller has to do, so a test that asserts a code now proves more. The interrupt is outside this table by decision (Exit codes and errors).

### Where each input landed

1. Pure logic against fixtures: Fixtures (layout, hand-written expectations, the criterion for what needs a broken fixture); the slug fixtures built from GitHub's renderer are a matter for the contract, as in ticket 4.
2. Deliberately broken projects: What needs a broken fixture; the two coverage tests.
3. Write paths: Frontmatter round trip, Write commands.
4. Process level: Process level (locks and signals); an interrupted write: Failing a write in the middle.
5. Environment isolation: Environment of spawned processes, and `Env` in `deps` for the library.
6. Network: Remote schemas in tests; the suite runs offline.
7. Shells: the three Shells sections.
8. Across all of it: the offline section, `--json` goldens, Across all commands, macOS, and the review of a copied fixture.

### A missing state record

Decided: "no state file" is two cases, told apart by a criterion that uses no number anyone chose. The unit is a collection within a namespace.

1. No record for the collection, and no coded documents of it in the namespace: it is new. `new` starts numbering as usual, creates the file or the entry with the first number it issues, and reports nothing.
2. No record for the collection, and coded documents of it exist in the namespace: the record was lost, or the documents were made before typdoc was used. This must be loud. `typdoc new` and `mv --renumber` refuse to issue a number for that collection in that namespace and exit 2, and `validate` reports the always-on rule `state.missing`.

The reason is in the design. `typdoc new` promises without condition that a number is never reused after its document is deleted, and the design says the state file is what makes that true. Reading a missing file as "start from the highest number that exists" does not mean no number has been issued; it means the record is gone. A collection numbered 1 to 40 from which ten documents were deleted would get numbers reused, and a ref left behind would point at a different document, which is what ticket 7's rule that the source namespace's `last` never goes down exists to prevent. It happens easily: someone puts `state/` in `.gitignore`, or clones a repository where the file was never committed. The layout in the design already says `state/` is committed; nothing enforces it, and no second rule is written for it. The design's sentence that the two sources can only disagree by leaving a gap was true only while the record existed, and it is corrected together with the promise.

Where it goes: `state.missing` is an always-on rule, in the table with `keys.unique` and `collections.overlap`, and not a row in the table of config errors next to `config.state-orphan`, although it is that error's mirror image (a namespace with documents and no file, against a file with no namespace). Config errors are reported when the config loads, without reading any document, and they stop the namespace. Recognising a lost record needs the index of documents, so it cannot be a config error, and as one it would stop every command in the situation of someone adopting typdoc on documents that already have codes, including `validate --audit`, the command the design gives for adoption. As an always-on rule it is reported by `validate` as an error, listed by `--audit` without stopping it, and cannot be switched off, which keeps the safe behaviour as the default: `new` never issues a number silently in that state.

The cost of that, stated: adopting typdoc on documents that already have codes takes one deliberate act before `new` works, creating the state file with `last` set to the highest existing number (or higher, if a higher number was ever used and deleted). The message says so. The act is deliberate because only the person adopting knows whether any document was ever deleted, and typdoc cannot tell a lost record from a repository that never had one.

The boundary is drawn per collection: a state file that exists but has no entry for a collection is the same two cases, decided by whether that collection has coded documents in the namespace.

A limit, stated as it is: a record can be lost without a trace when every coded document of the collection in that namespace has also been deleted, since nothing is left to compare it with. Nothing detects that, and the guarantee does not claim to.

How it is tested: a fixture in `broken/state.missing` (documents with codes and no state file, and the command `new` as the declared argument), one for an existing file without the entry, and one for `mv --renumber` into a namespace in that state; each asserts exit 2, that the rule id is `state.missing` and that nothing under the tree changed. Cases 1 and 2 above are the pairs of hand-written expectations for `new`: a new collection gets its first number and the entry, a collection with documents and no record gets the refusal. The fix leaves the design's promise in one form: unconditional wording is replaced by a condition the design can keep.

### A name that a command prints is accepted by the next

Decided: a test walks every document of every project in the fixtures (a project with one namespace and one with several, and one with an import) and builds from the identity a command printed (`path`, `namespace`, `key`, `project`) each string that the table under Arguments that name a document in the design gives, in the path form and in the key form where the document has a code. It passes each to `get --json` and asserts that the file it returns is the one it started from. If the rule for assembling a string is wrong, or a printed shape changes one day, this goes red at once: the promise has something that watches it instead of living in the document only. It is built in story 1.

### Where each part is built

The test strategy applies to all three stories that deliver v1 (see the map): reading in story 1, writing in story 2, remote schemas in story 3. Each story builds the part its commands need, and its contract lists that part under Testing Decisions.

| Part | Story 1 (the read core) | Later |
| --- | --- | --- |
| Fixtures and packaging | `fixtures/valid` and `fixtures/broken`, both coverage tests, the loader that fails loudly outside a checkout, `examples/` validated as a user receives it | `fixtures/roundtrip`: story 2 |
| What needs a broken fixture | a fixture for every rule and config error that story 1 builds, including `state.missing` (it only reads) | anything that needs a fetch: story 3 |
| `--json` goldens | goldens for `get`, `list`, `toc`, `refs`, `validate` and the error object; the generator, with its guard (it writes no assertion file and has no regenerate-all) | the `Clock` and the goldens of commands that stamp time: story 2 |
| `run(args, deps)` | `run` and `Env` in `deps`; the ban on `std::env::var` and on the home directory in `typdoc-core` | `Clock`: story 2; `Fetch` and `ureq`: story 3; the write seam: story 2 |
| Environment of spawned processes | the spawn helper (empty environment, constant `PATH`, a fresh `HOME`), the ban on `Command::new` outside it, the `--all-targets` lint | the tripwire proxy: story 3, since story 1 has no network |
| Shells | the examples harness for sh and bash, and a listed shell that is missing turns the suite red, because the quoting paragraph belongs to Query | none |
| Imports | the five cases of finding `imports.json`, tested with a fake `Env` | none |
| Round trip, the write seam, locks and signals, the write commands, the write-side of the state file | none | story 2 |
| Remote schemas in tests, the offline tripwire | none | story 3 |
| Pinned copies of remote schemas | hand-made pinned copies and the config errors about pins (settled, below) | fetching them: story 3 |

The rows about pinned copies follow ticket 20: a read command that meets a remote schema with no pin reports `config.schema-unpinned` until story 3 adds fetching. Every row stands.

### Differences between the design and the binary that are acknowledged

Two coverage tests of Across all commands would be red from the first day of story 1, which builds only the read commands: nine commands need a golden, and exit codes 3 and 4 are produced by writes. A gate that starts red is switched off, not repaired, so a difference between the design and the binary is recorded openly in code, in two lists: `unimplemented_commands` and `unproduced_exit_codes`.

What a list is: the design describes something the binary does not yet have, and each entry is a difference that is known and accepted. It is not a queue of things that are yet to come. Two checks give a list its meaning, for commands and in the same way for exit codes:

1. Every command heading in the Commands section is in the registry or in the list, and every exit code in the design's table is produced by a test or is in the list.
2. Something in the list is not in the registry, and a code in the list is not produced by any test. An entry that outlives the work is red at once, and so the list cannot be used as a way out: a command that was built and then put in the list to avoid writing its golden is caught by this check.

An entry carries the story expected to deliver it as a comment for readers. No test looks at it, so that putting the stories in a different order, or moving a command from one to another, cannot turn the suite red for a reason that has nothing to do with the program. The lists only shrink. Empty is the correct state of a finished v1: not a list that happens to have no members, but one that is expected to disappear.

Not decided: how exit code 6 is produced by a read command in a test. A failure caused by permissions does not work when the tests run as root, and there is no seam behind reads. If a deterministic way is not found, code 6 goes in `unproduced_exit_codes` until one is.

### Closing

Every group of inputs has an answer above (Where each input landed). What stays with the contract, all on the map: exact `--json` shapes with the order of each array; whether a command that fails midway is all or nothing across two namespaces and whether a temp file may remain; the crate that catches signals, and that every command takes its locks through one path; the shape of the `mv --renumber` argument; the error id of a write rejected by the re-read check; proving `yaml-edit` on anchors and tags; the default filesystem of macOS, case, and `mv` that changes only case; what an interrupted write leaves behind; and tests that run the real HTTP adapter inside the test process. Parked, blocking nothing: a macOS runner and the public-text gate in CI.
