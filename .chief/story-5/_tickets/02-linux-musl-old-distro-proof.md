Type: implementation
Status: resolved
Blocked by: 01

# Ticket 02 — Linux minimum-supported-distro proof

After the musl binaries build (ticket 01's reusable workflow, consumed via its PR-triggered
caller), add a CI step per Linux arch that copies the built binary into an intentionally old,
unrelated container image (same choice on both arches) and runs `typdoc --version` there,
proving no dynamic dependency on that system's libc or missing kernel feature. Both
`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` get this proof.

Docs: state the minimum supported Linux (kernel floor, no glibc version requirement) next to the
install instructions this story is adding.

Demoable on its own: the CI job log shows the container run succeeding on both arches.
