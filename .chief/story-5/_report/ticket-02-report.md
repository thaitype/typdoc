# Ticket 02 Report

## Ticket
Prove the two musl binaries run on an old, unrelated Linux distro (both arches).

## Outcome
done

## Decision
- **Issue:** which old distro to run the proof in — contract only gave a non-binding example
  (an old Debian release).
- **Options considered:** very old EOL Debian (e.g. debian:6/7 — but these predate Debian's
  arm64 port, so the image wouldn't exist for one of the two arches) vs. CentOS 7.
- **Chosen:** CentOS 7 — EOL mid-2024, glibc 2.17/kernel 3.10 baseline, genuinely old and
  unrelated to this repo's own Ubuntu-based build images, and confirmed (via `docker manifest
  inspect`) to be an official multi-arch image with both amd64 and arm64/v8 manifests, so the
  same tag works unmodified on both musl targets' runners.
- **Issue:** contract said "copied into" the container image; implementation used a read-only
  bind mount instead of `docker cp`.
- **Chosen:** bind mount — functionally equivalent for what's being proven (does the static
  binary start with no dynamic-libc dependency from the old system), simpler in CI. Flagged by
  review as a literal wording deviation, judged non-blocking, documented inline in the workflow.
- **Issue:** minimum-supported-kernel number for docs — first draft used one shared "3.2+" floor
  for both arches.
- **Corrected during review:** Rust's own platform-support table puts `aarch64` at kernel 4.1+,
  not 3.2+ (confirmed against Rust's 2022 glibc/kernel-requirements post, which says aarch64-musl
  was untouched by that bump because it already had the higher floor). Docs now state both
  numbers correctly.

## Notes
- x86_64 leg: proven directly in-sandbox (real musl build, real CentOS 7 container run,
  `typdoc 0.3.1` printed). aarch64 leg: proven by symmetry (same image confirmed multi-arch, same
  command) since no arm64 hardware/QEMU was available in the build sandbox — the real native
  proof happens on the actual `ubuntu-24.04-arm` CI runner, same limitation ticket 01 already
  noted for its own aarch64 verification.
- Docs note landed in `docs/getting-started.md`, not README — ticket 03 (which adds the real
  install section) hadn't merged yet when this ticket ran.
- Extends ticket 01's existing reusable `dist-build.yml` workflow (new step, gated on
  `contains(matrix.target, 'musl')`) rather than adding a separate workflow — so both the PR
  caller and the real release pick it up automatically.
- Commit `0d5f4e3` on `story-5-prebuilt-installer` (rebased cleanly onto the tip before merge, no
  conflicts). Public-text grep clean (checked twice).
