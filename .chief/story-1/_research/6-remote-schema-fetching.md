# Research 6: HTTP client for remote schemas (ureq vs reqwest), and does typdoc-core stay async-free?

Date: 2026-09-19. Feeds ticket `6-remote-schema-fetching` (and ticket 9, test strategy). Written by Mina.
Toolchain used: cargo/rustc 1.96.0, Linux x86_64, 4 cores. Crate versions current on crates.io that day:
`ureq 3.4.2` (published 2026-09-13), `reqwest 0.13.5`, latest 0.12 is `0.12.28`, `rustls 0.23.45`.
Sibling workspace `typ-fleet-4` pins `reqwest 0.12` with `default-features = false, features = ["rustls-tls","json"]` and `tokio 1 full` (its `Cargo.toml` lines 12 and 19).

## Recommendation (short)

Use **`ureq` 3.x (blocking)** behind a tiny `Fetch` trait owned by `typdoc-core`. Keep `typdoc-core` runtime-free. Details and reasons are at the end; the cost of the runner-up (`reqwest`) is stated there too.

## 1. Measurements (mine, reproducible)

Method: one tiny binary per variant, each doing one `GET` of an `https://` URL with `https_only(true)` and a timeout, parsing the body with `serde_json`. Each in its own crate and its own `target/` in the scratchpad. Builds ran one at a time. `[profile.release] strip = true`, otherwise default release profile (opt-level 3, no LTO). Clean build of the whole crate including `serde_json`; registry sources were already downloaded (`cargo fetch`), so times exclude download. **One run each, on a 4-core machine, so treat times as +/- 10-20%; sizes and crate counts are deterministic.**

Commands (scratchpad `.../scratchpad/bench/`, script `run.sh`):

```bash
cd $CRATE && cargo fetch -q
S=$(date +%s.%N); cargo build --release -q; E=$(date +%s.%N)      # build_s = E - S
stat -c %s target/release/$CRATE                                   # bin_bytes
cargo tree -e normal,build --prefix none | sort -u | wc -l         # unique crates, includes the root crate and serde_json's 4-5
```

Dependency lines used:

| Variant | Cargo.toml dependency |
|---|---|
| base (serde_json only) | `serde_json = "1"` |
| ureq3 | `ureq = "3.4.2"` (defaults = `rustls` + `gzip`) |
| ureq3_min | `ureq = { version = "3.4.2", default-features = false, features = ["rustls"] }` |
| reqwest012_blocking | `reqwest = { version = "0.12.28", default-features = false, features = ["rustls-tls","blocking"] }` |
| reqwest012_async | same without `blocking`, plus `tokio = { version = "1", features = ["rt","macros"] }`, `#[tokio::main(flavor="current_thread")]` |
| reqwest013_blocking | `reqwest = { version = "0.13.5", default-features = false, features = ["rustls","blocking"] }` |
| reqwest013_async | same without `blocking`, plus the same minimal tokio |

Results (raw output in scratchpad `bench/results.txt`):

| Variant | Clean release build | Stripped binary | Unique crates in tree | Delta vs base (time / size / crates) |
|---|---|---|---|---|
| base | 4.5 s | 436 KB | 6 | - |
| **ureq 3.4.2, defaults** | **27.4 s** | **2.65 MB** | **35** | +23 s / +2.2 MB / +29 |
| ureq 3.4.2, no gzip | 27.2 s | 2.58 MB | 30 | +23 s / +2.1 MB / +24 |
| reqwest 0.12.28 blocking (rustls-tls) | 46.4 s | 3.60 MB | 91 | +42 s / +3.2 MB / +85 |
| reqwest 0.12.28 async + tokio rt | 37.9 s | 3.58 MB | 90 | +33 s / +3.1 MB / +84 |
| reqwest 0.13.5 blocking (rustls, aws-lc-rs) | 76.1 s | 5.76 MB | 95 | +72 s / +5.3 MB / +89 |
| reqwest 0.13.5 async + tokio rt | 72.9 s | 5.73 MB | 94 | +68 s / +5.3 MB / +88 |

Reading it:
- ureq is roughly 1.4-2.8x faster to compile from clean and 1.4-2.2x smaller than any reqwest variant, with about a third of the crate count. `cargo tree -i tokio` on the ureq build finds no tokio (verified: "package ID specification `tokio` did not match any packages"). The ureq tree has no `hyper`, `tower`, `futures-*`, `tokio*`.
- The reqwest 0.12 tree includes `tokio 1.53.1`, `hyper 1.11.1`, `hyper-util`, `hyper-rustls`, `tokio-rustls`, `tower`, `tower-http`, `futures-*` (from `cargo tree` on `reqwest012_blocking`). **The `blocking` feature does not avoid tokio in the dependency tree** (see section 2).
- reqwest 0.13 is much heavier than 0.12 in this configuration because its `rustls` feature now selects `aws-lc-rs` as the crypto provider and `rustls-platform-verifier` (feature table from `cargo info reqwest`: `rustls = [__rustls-aws-lc-rs, dep:rustls-platform-verifier, __rustls]`). The 0.13.0 changelog says so directly (see sources).
- Caveats: I did not measure debug-build time, incremental rebuilds, LTO/opt-level tuning, the `platform-verifier` variant of ureq, or a `native-tls` variant (no cmake/OpenSSL dev headers were checked). The sibling's `tokio = "full"` would add more than the minimal `rt,macros` I used. Whole-crate numbers include serde_json (about 4.5 s), so client-only cost is the "delta" column.

## 2. Facts from primary sources

### TLS backend and root certificates

- ureq (docs.rs/ureq/3.4.2/ureq/): "By default, ureq uses `rustls` crate with the `ring` cryptographic provider." `native-tls` "is never picked up as a default". Roots: "By default, ureq uses Mozilla's root certificates via the webpki-roots crate" (static bundle); the `platform-verifier` feature uses "the OS roots instead". Feature table from `cargo info ureq@3.4.2`: `default = [rustls, gzip]`, `rustls = [rustls-no-provider, _ring, rustls-webpki-roots]`, plus `native-tls`, `platform-verifier`, `native-tls-webpki-roots`.
- reqwest 0.13 CHANGELOG (github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md, v0.13.0): "rustls is now the default TLS backend, instead of native-tls"; "rustls crypto provider defaults to aws-lc instead of ring"; "rustls-platform-verifier is used by default. To use different roots, call tls_certs_only(your_roots)"; "rustls-tls has been renamed to rustls"; `query` and `form` are now opt-in features. `cargo info reqwest@0.13.5` shows `default = [default-tls, charset, http2, system-proxy]` and `default-tls = [rustls]`, and a `rustls-no-provider` feature.
- reqwest 0.12 (what the sibling uses) with `rustls-tls`: I verified only by build that the tree contains `webpki-roots v1.0.9` and `ring v0.17.14` (same as ureq's tree). I did not read the 0.12 docs page for this; treat "0.12 rustls-tls = webpki-roots + ring" as an inference from the dependency tree.
- rustls docs (docs.rs/rustls/latest/rustls/, v0.23.45, 2026-09-14): default provider is `aws-lc-rs`; `ring` is opt-in; "The simplest way is to depend on the `webpki_roots` crate which contains the Mozilla set of root certificates."
- webpki-roots (docs.rs/webpki-roots/latest/webpki_roots/): "a compiled-in copy of the root certificates trusted by Mozilla", suits "applications that can always be recompiled and instantly deployed".
- rustls-platform-verifier README (github.com/rustls/rustls-platform-verifier; also read locally in the 0.7.0 crate source): on Linux it uses the "System CA bundle, or user-provided certs" via `rustls-native-certs` and `openssl-probe`, loaded once at startup; it does not support CRL revocation on Linux. It argues platform verification is the best default for client apps, and static `webpki-roots` is a clear answer for containerised apps deployed often. `rustls-native-certs 0.8.4` source (`src/lib.rs` lines 8, 52-53) honours `SSL_CERT_FILE` and `SSL_CERT_DIR`.
- native-tls (docs.rs/native-tls/latest/native_tls/): SChannel on Windows, Secure Transport on macOS, "OpenSSL (via the `openssl` crate) on all other platforms"; a `vendored` feature statically links OpenSSL on non-Windows/macOS. So `native-tls` on Linux means OpenSSL at build or run time; both clients offer it, neither default. Not built or measured here.

Consequence for typdoc: root store is a real choice, not a client choice. Both clients can do (a) static Mozilla roots (reproducible, works in bare containers, cannot see a corporate MITM CA) or (b) OS roots via platform-verifier (sees corporate CA on Linux only through the system bundle or `SSL_CERT_FILE`, loaded once). ureq's default is (a); reqwest 0.13's default is (b); reqwest 0.12 with `rustls-tls` is (a).

### Redirects

- ureq (`ConfigBuilder`/`Config` docs, docs.rs/ureq/3.4.2/ureq/config/): `max_redirects` default 10; `max_redirects_will_error` default true (errors with `TooManyRedirects`); `redirect_auth_headers` default `Never`; `https_only`: "Whether to limit requests (including redirects) to https only", default false. Source `ureq-3.4.2/src/run.rs` line 149: `if config.https_only() && uri.scheme() != Some(&Scheme::HTTPS) { return Err(Error::RequireHttpsOnly(..)) }` inside `call_run`, which runs once per redirect hop.
- reqwest 0.13.5 (docs.rs/reqwest/0.13.5/reqwest/): "By default, a Client will automatically handle HTTP redirects, having a maximum redirect chain of 10 hops." Source `src/async_impl/client.rs` lines 2626-2628 reject a non-https initial URL under `https_only`, and lines 1022-1025 pass `with_https_only(config.https_only)` to the redirect policy, so redirect hops are covered too.
- Empirically (section 4C) both clients refuse an `https` to `http` downgrade redirect when `https_only(true)` is set, and follow `https` to `https`.

### Timeouts

- ureq: every timeout defaults to `None`, including `timeout_global`. `timeout_global` is "end-to-end, from DNS lookup to finishing reading the response body"; there are also `timeout_connect`, `timeout_resolve`, `timeout_recv_response`, `timeout_recv_body`, `timeout_per_call`. `timeout_resolve` doc: because most platforms have no async DNS syscall, setting it "might force ... to spawn a thread". **A fetch with no timeout set can hang forever; typdoc must set `timeout_global`.**
- reqwest blocking: "Default is 30 seconds" (`src/blocking/client.rs` line 385; `Timeout(Some(Duration::from_secs(30)))` at line 1562). I did not confirm the async client's default; the reqwest docs page I fetched did not state it. Mark unverified.
- Empirically both honour a 2 s timeout against a black-hole listener (section 4D).

### Blocking model, async runtime

- ureq README/docs: "It uses blocking I/O instead of async I/O, because that keeps the API simple and keeps dependencies to a minimum."
- reqwest blocking module (docs.rs/reqwest/0.13.5/reqwest/blocking/): the client "will block the current thread"; it must not be used from inside an async runtime (it panics when it tries to block there); wrap in `tokio::task::spawn_blocking`. Source `src/blocking/client.rs` lines 1424-1428: `ClientHandle::new` spawns a thread named `"reqwest-internal-sync-runtime"` that builds a `tokio::runtime::Builder::new_current_thread().enable_all()` runtime. So `reqwest::blocking` gives a sync API to callers, but tokio, hyper and an extra thread are still compiled in and started at runtime.
- reqwest async needs an executor: the caller must provide tokio (0.12 and 0.13 both use hyper 1.x and tokio).

### Failure behaviour and defaults (docs + observed)

- ureq: HTTP 4xx/5xx returns `Err(StatusCode(n))` by default (observed for 404). Response body helpers such as `read_to_string` have "a default 10MB limit" (docs.rs/ureq/3.4.2/ureq/struct.Body.html), raised with `body.with_config().limit(n)`; exceeded gives `BodyExceedsLimit`. The error enum (src/error.rs) has `RequireHttpsOnly`, `Timeout`, `HostNotFound`, `ConnectionFailed`, `TooManyRedirects`, `Io`, `Rustls`, and so on.
- reqwest: `send()` returns `Ok` on a 404; you must call `error_for_status()` (observed: 404 gave `Ok` with an empty body). Errors are one opaque `reqwest::Error` with `is_*` predicates and a nested source chain.
- Offline (observed, section 4D): both fail fast and cleanly for connection refused and DNS failure. ureq surfaced DNS failure as `Io("failed to lookup address information: Name or service not known")`, not as `HostNotFound`, on this machine, so do not rely on matching `HostNotFound`. reqwest surfaced it as `Request` with a `dns error` source. For typdoc, both map to "remote schema has no pin and cannot be fetched" (design.md line 546 config error); the offline case must produce that message from any fetch error, so classify by "fetch failed", not by variant.
- Proxies: reqwest 0.13 "System proxies are enabled by default" (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`); ureq docs say proxy config is "Picked up from environment when using `Config::default()` or `Agent::new_with_defaults()`". Not tested.
- ureq changelog (github.com/algesten/ureq/blob/main/CHANGELOG.md): 3.3.0 raised MSRV 1.71 to 1.85 and adopted edition 2024; 3.4.1 fixed "timeout budget application across multiple request phases" and TLS handshake completion; 3.4.2 fixed a network-path-reference redirect handling issue. Both crates list rust-version 1.85 (`cargo info`), compatible with typdoc's edition 2024. Both are actively released (ureq 3.4.2 on 2026-09-13).

## 3. Is keeping `typdoc-core` free of an async runtime practical?

Yes, and cheaply, with ureq. Evidence:
- With ureq there is no tokio anywhere in the tree (section 1), so `typdoc-core` needs no runtime and consumers add none. The client API is plain `fn`.
- With reqwest there is no way to get a plain sync dependency without tokio: `blocking` still builds tokio and hyper and spawns a thread (source above). An async reqwest client in `typdoc-core` would force an executor on every consumer of the lib.
- Independent of the client: design.md says every other part is local file work and that only an unpinned URL or `pull` touches the network (lines 174, 443-446). So the network is a small edge. Put it behind a trait in `typdoc-core`, for example `trait Fetch { fn get(&self, url: &Url) -> Result<Vec<u8>, FetchError> }` with `FetchError` owned by typdoc, and put the ureq adapter in `typdoc-core` behind a default-on cargo feature, or in the `typdoc` bin. This keeps ureq's and reqwest's error types out of the public API, which is what makes a later swap cheap.

**Cost of staying blocking if concurrency is wanted later.** From design.md, `pull` re-fetches "all of them ... including remote parents they extend". The set is a handful of files; parents are discovered only after the child is fetched, so only breadth is parallelisable. With ureq: `Agent` is `Send + Sync` and "Cloning an Agent results in an instance that shares the same underlying connection pool" (docs.rs/ureq/3.4.2/ureq/struct.Agent.html), and the doc suggests "dispatching requests from different threads". So concurrency is `std::thread::scope` plus a cloned `Agent`: one OS thread per in-flight fetch, no cancellation of an in-flight request other than its timeout, no cooperative scheduling. That is fine for tens of schemas; it would be a poor fit for hundreds of concurrent fetches. I did not run a concurrent-fetch test; this rests on the docs quoted. If real async ever became necessary (for example an MCP server embedding typdoc-core), the cost is one adapter rewrite behind `Fetch` plus adding tokio to the bin crate only.

## 4. Mocking the remote, with plain http refused in production

### 4-pre. Empirical setup

Crate `mock/` in the scratchpad: `ureq 3.4.2` (`rustls`), `reqwest 0.13.5` (`rustls`, `blocking`), dev-style deps `rcgen 0.14` and `rustls 0.23` (`ring`). The program starts (a) a plain-http listener on 127.0.0.1:0, (b) an in-process TLS server on 127.0.0.1:0 using a self-signed certificate from `rcgen` for `127.0.0.1`/`localhost`, (c) a redirecting TLS server, (d) closed-port and black-hole listeners. Commands:

```bash
cd .../scratchpad/mock && cargo build && ./target/debug/mock      # output saved in .../scratchpad/mock_output.txt
```

Client setup used: ureq `Agent::config_builder().https_only(true).timeout_global(Some(t)).tls_config(TlsConfig::builder().root_certs(RootCerts::new_with_certs(&[Certificate::from_der(der).to_owned()])).build())`. reqwest `blocking::Client::builder().https_only(true).timeout(t).tls_certs_only([Certificate::from_der(der)?])`.

### 4A-4D. Observed results (abridged from `mock_output.txt`)

| Case | ureq 3.4.2 | reqwest 0.13.5 blocking |
|---|---|---|
| `https_only(true)` against `http://127.0.0.1:P` | Err `RequireHttpsOnly(url)`, 0.2 ms, no connection made | Err `Builder ... BadScheme` |
| `https_only(false)` against the same plain-http mock | OK, body returned | OK |
| TLS mock, default roots | Err `InvalidCertificate(UnknownIssuer)` | Err, same cause nested in `hyper_util ... Connect` |
| TLS mock, test root injected | OK | OK |
| https redirect to http mock, `https_only(true)` | Err `RequireHttpsOnly(http://...)` | Err `error following redirect ... BadScheme` |
| https redirect to https, `https_only(true)` | OK | OK |
| Connection refused | Err `Io(ConnectionRefused)`, 0.25 ms | Err `Request ... tcp connect error`, ~41 ms |
| DNS failure `.invalid` | Err `Io("failed to lookup address information")` | Err `Request ... dns error` |
| Black hole, 2 s timeout | Err `Timeout(Global)` at 2.03 s | Err `TimedOut` at 2.06 s |
| 404 | Err `StatusCode(404)` | `Ok` (needs `error_for_status`) |

reqwest's first requests took tens of ms more than ureq's (for example 41-94 ms versus 0.2-4 ms for refused/TLS-failed cases). I did not investigate why; it is not needed for the decision and may include one-off client construction or root loading. It is a single debug-build run.

### Mocking options, ranked

1. **Inject a `Fetch` trait (recommended for most tests).** Unit and CLI-level tests in `typdoc-core` use an in-memory `Fetch` (a map of URL to bytes) so no socket, no TLS and no `https_only` toggle is involved. Production still refuses plain http because the real adapter is built with `https_only(true)` and the trait is the only path. This works identically with either client, and is the only option that makes "tests must run offline" independent of the client choice.
2. **One small loopback TLS test for the real adapter.** In-process TLS server with an `rcgen` self-signed certificate, and the adapter constructed with the test root added (ureq: `RootCerts::new_with_certs`; reqwest: `tls_certs_only`). Shown to work for both clients above. Keeps `https_only(true)` on, so the test also proves plain http and downgrade redirects are refused. Dev-dependencies only (`rcgen`, `rustls` with `ring`); they are not in the release binary. Needs a constructor that accepts extra roots, for example `UreqFetch::with_root(cert_der)`, not `pub` in the CLI.
3. **End-to-end test of the `typdoc` binary against a local server.** The binary must trust the test root without a code path that weakens production. Options: (a) `SSL_CERT_FILE` with the `platform-verifier` feature, since `rustls-native-certs` honours it (source above), but that is not honoured with ureq's default `webpki-roots` roots; (b) a hidden, documented test-only env var to add a root, which is an extra trust knob in the shipped binary; (c) skip end-to-end and rely on option 1 at the CLI layer via a test-only `Fetch` injection point. I recommend (c) and only add (a) if we ship `platform-verifier`. Mark: I did not run an `SSL_CERT_FILE` experiment; this rests on the `rustls-native-certs` source lines quoted above.
4. **`https_only(false)` in tests against a plain-http mock (works, discouraged).** Shown to work for both clients. It means the test config differs from production in exactly the property we most want to prove, and needs a switch that must never be reachable from the CLI. If used at all, gate it by `#[cfg(test)]` or an internal constructor, never a flag or env var.
5. `wiremock`/`httpmock`/`mockito`-style servers: I did not evaluate them. Most serve plain http on loopback, so they map to option 4 and need `https_only(false)`.

Client comparison for mocking: ureq is synchronous, so a loopback server is just `std::thread` plus `TcpListener`, as in my program; no runtime in tests. reqwest blocking works the same way from the test's side. reqwest async tests need `#[tokio::test]`, which is where the runtime cost leaks into `typdoc-core` tests.

## 5. Recommendation

**Use `ureq` 3.x, blocking**, with `typdoc-core` owning `trait Fetch` and `FetchError`.
- Suggested config: `https_only(true)`, explicit `timeout_global` (default is none), `max_redirects` at the default 10 or lower, explicit body limit (default 10 MB is fine for a schema; state it deliberately), and treat any error as "cannot fetch".
- Roots: default static `webpki-roots` (reproducible, works in bare CI containers). If users behind a corporate CA are expected, enable `platform-verifier` (I did not measure its size or build cost); this is a small decision that can be deferred behind a cargo feature.
- Features: `default-features = false, features = ["rustls"]` drops gzip (about 70 KB, 5 crates); either is fine.

Why: measured 23 s / 2.2 MB / 29 crates of overhead against 42-72 s / 3.2-5.3 MB / 85-89 crates for reqwest; no tokio anywhere in the tree, so `typdoc-core` stays runtime-free at no effort; `https_only` is enforced per redirect hop; mocking is plain threads and sockets.

**Cost of the runner-up (`reqwest`):** roughly 1.4-2.8x the compile time, 1.4-2.2x the binary size and about 2.6-2.7x the crate count for a feature used in two places; tokio, hyper and tower enter the tree even with `blocking`, which spawns an internal runtime thread and panics if a caller is inside an async runtime; going async would force tokio on every `typdoc-core` consumer; 0.13's default crypto provider is `aws-lc-rs` (76 s build here; C code compiled by `cc`). In exchange reqwest would give real async concurrency, HTTP/2 by default, and consistency with `typ-fleet-4` (which shares no build cache with typdoc anyway, since they are separate workspaces).

**Cost of choosing ureq:** no async and thread-per-fetch concurrency (fine for the few schemas a namespace has); errors need classification by hand (DNS failure came back as `Io`, not `HostNotFound`); default timeout is none and must be set.

## 6. Not verified

- Timings are single runs; no debug-build, incremental or LTO measurements.
- `native-tls` and ureq `platform-verifier` variants were not built or measured.
- reqwest 0.12 root-store behaviour is inferred from the dependency tree, not docs.
- The async reqwest client's default timeout was not confirmed.
- Whether ureq 3 supports HTTP/2: not checked, no claim made.
- Concurrent fetching with cloned `Agent` across threads: from docs only, not run.
- `SSL_CERT_FILE` end-to-end test through a client: not run; only the `rustls-native-certs` source was read.
- Proxy-environment behaviour of either client: not tested.
- WebFetch summaries of docs.rs pages are lossy; every load-bearing behavioural claim above was also confirmed in crate source or by running the mock program.

## Sources

- https://docs.rs/ureq/3.4.2/ureq/ : blocking I/O rationale, rustls/ring default, webpki-roots default, platform-verifier, native-tls never default.
- https://docs.rs/ureq/3.4.2/ureq/config/struct.Config.html and `.../struct.ConfigBuilder.html` : `https_only` (including redirects), redirect defaults, all timeouts default `None`, proxy from env.
- https://docs.rs/ureq/3.4.2/ureq/struct.Body.html : default 10 MB read limit.
- https://docs.rs/ureq/3.4.2/ureq/struct.Agent.html : clone shares pool, `Send + Sync`.
- https://github.com/algesten/ureq/blob/main/CHANGELOG.md : 3.3.0 MSRV 1.85 and edition 2024; 3.4.x fixes.
- ureq 3.4.2 source (crates.io registry copy): `src/run.rs` line 149, `src/error.rs`, `src/tls/cert.rs`.
- https://docs.rs/reqwest/0.13.5/reqwest/ and `.../blocking/index.html` : redirect default 10 hops, system proxies, blocking client caveats.
- https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md : v0.13.0 TLS changes; `https_only` added in 0.10.10.
- reqwest 0.13.5 source: `src/blocking/client.rs` lines 385, 1424-1428, 1562; `src/async_impl/client.rs` lines 1022-1025, 2626-2628.
- `cargo info ureq@3.4.2`, `cargo info reqwest@0.13.5` : feature tables and rust-version.
- https://docs.rs/rustls/latest/rustls/ : aws-lc-rs default, ring opt-in, webpki-roots suggestion.
- https://docs.rs/webpki-roots/latest/webpki_roots/ : compiled-in Mozilla roots.
- https://github.com/rustls/rustls-platform-verifier (README, also in crate 0.7.0 source) : Linux behaviour, comparison table.
- rustls-native-certs 0.8.4 source `src/lib.rs` : `SSL_CERT_FILE`, `SSL_CERT_DIR`.
- https://docs.rs/native-tls/latest/native_tls/ : OpenSSL on Linux, `vendored`.
- Local: `/home/thw-home/gits/thaitype/typdoc/docs/design.md` (lines 157-177, 443-446, 546), `.chief/project.md`, `/home/thw-home/gits/typ-fleet-home/typ-fleet-4/Cargo.toml` (read-only).
