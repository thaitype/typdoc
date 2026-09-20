# 6: Which HTTP client for remote schemas, and does the CLI go async?

Type: wayfinder:research
Status: resolved
Blocked by: None (can start immediately)

## Question

Remote schemas are fetched only for an unpinned URL and by `typdoc pull`, over `https://` only, then pinned by SHA-256 under `.typdoc/vendor/`. Everything else in the CLI is local file work.

Find, from primary sources, the trade-off between a blocking client (for example `ureq`) and `reqwest`, which the sibling crates use but which pulls in `tokio`: compile time and binary size, TLS backend (rustls versus native), redirect handling, timeouts, and offline behaviour. Establish whether keeping `typdoc-core` free of an async runtime is practical. Recommend one, with the cost of the alternative stated.

## Answer

Full findings, measurements and sources: [research 6](../_research/6-remote-schema-fetching.md).

**Approach: `ureq` 3.x (blocking)**, behind a `Fetch` trait and a typdoc-owned `FetchError` in `typdoc-core`, so neither client's error type appears in the public API and a later swap costs one adapter.

- **`typdoc-core` stays async-free.** ureq's dependency tree contains no tokio. `reqwest::blocking` still compiles tokio and hyper and spawns an internal runtime thread, so a runtime-free lib is not possible with reqwest at all. Checked independently in the registry source: `src/blocking/client.rs` names that thread `reqwest-internal-sync-runtime` in both 0.12.28 and 0.13.5.
- **Measured cost of the client** (clean release build, one run each, 4 cores, so times are +/- 10-20%; sizes and crate counts are deterministic):

  | Client | Build added | Binary added | Crates added |
  |---|---|---|---|
  | ureq 3.4.2 | 23 s | 2.2 MB | 29 |
  | reqwest 0.12.28 | 33-42 s | 3.1-3.2 MB | 84-85 |
  | reqwest 0.13.5 | 68-72 s | 5.3 MB | 88-89 |

- **Both** can refuse plain http and an https-to-http redirect, and both can be tested against a loopback TLS mock. Most tests should use an in-memory `Fetch` instead.
- **Configure deliberately:** `https_only(true)`; an explicit `timeout_global` (ureq's timeouts all default to none); redirects at 10 or fewer; body limit stated on purpose (10 MB default is ample for a schema); treat any error as "cannot fetch", because DNS failure came back as `Io`, not `HostNotFound`.
- **Costs accepted:** no async, so later concurrency means one thread per in-flight fetch with cloned `Agent`s (fine for the handful of schemas a namespace has); error classification by hand.
- **Cost of the runner-up:** reqwest would add 1.4-2.8x the compile time, 1.4-2.2x the binary size and about 2.6x the crate count for two call sites, and would force tokio on every `typdoc-core` consumer if used async. Its gains are real async and consistency with the sibling workspace.

## Not verified

Timings are single runs; ureq's `platform-verifier` and `native-tls` variants were not built (root-store choice for users behind a corporate CA is deferred behind a cargo feature); concurrent fetching, proxy behaviour and an `SSL_CERT_FILE` end-to-end test were not run.

**Decided 2026-09-19:** the approach above stands, with these additions: TLS is rustls; an explicit timeout and a maximum response size; redirects only https to https; `HTTPS_PROXY` is supported. Notes: `ureq`'s proxy behaviour was not run in research, so it is to be probed in the prototype for the contract (`NO_PROXY` should be honoured too); the root store is the bundled one, and users behind a corporate CA stay deferred behind a cargo feature. Most tests use an in-memory `Fetch`; a loopback TLS mock is only for the https-only and redirect rules (ticket 9).
