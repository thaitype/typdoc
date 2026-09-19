# 9: What is the test strategy, given tests must run offline and gates must be able to fail?

Type: wayfinder:grilling
Status: open
Blocked by: 1, 6

## Question

Project rules require that `cargo test --workspace` passes offline, that remote schemas are mocked, and that every command has stable `--json` output and exit codes 0 to 4. Prior team experience is that a check which cannot fail is not evidence.

Decide: how fixtures are laid out (namespaces under `tests/` or `examples/`), whether `--json` output is pinned with golden files, how remote schemas are mocked given the HTTP client chosen in ticket 6, how frontmatter round-trip (write, then re-read, then diff the body byte for byte) is tested given the approach chosen in ticket 1, and which deliberately broken fixtures prove each validation rule can go red.

## Answer

