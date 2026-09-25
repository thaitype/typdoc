Type: implementation
Status: open
Blocked by: 01

# Ticket 07 — GitHub Release creation + asset upload (found gap, added mid-loop)

Not in the original ticket frontier — surfaced by ticket 01's own build, and confirmed live: every
existing release on this repo (`v0.1.0`, `v0.2.0`, `v0.3.0`) has `assets: []`. Release creation has
been a manual, out-of-band step until now, and no ticket in the original frontier added the "make
this a real GitHub Release with the five binaries attached" step the goal actually requires
("release 0.3.1 ... carrying all five binaries" via `publish.yml`).

Add a `release` job to `publish.yml`, `needs: attest`, gated the same way `cargo publish` already
is (`if: ${{ inputs.dry_run == false }}` — a dry run builds and attests but creates nothing
public):

- Create the git tag `v<version>` and a GitHub Release for it if one doesn't already exist
  (`gh release create` or an equivalent action), using the CHANGELOG entry for that version as
  the release body — same shape as the existing `v0.3.0` release's body, generated instead of
  hand-written.
- Upload all five archives and their `.sha256` checksum files as release assets.
- Confirm ordering: this job runs after `attest` (ticket 01), so the uploaded archives are the
  same ones already attested — never re-build or re-download from a different source.

Out of scope for this ticket: the `v0.3.1-rc.1` pre-release from the story's "Full-flow proof"
section is a separate, differently-triggered workflow (push/tag, not `workflow_dispatch`,
explicitly not this ticket's job) — but it's reasonable and encouraged to factor the "create
release + upload these assets" logic so both can call it, rather than duplicating it outright.
That factoring is this ticket's call to make, not a requirement.

Demoable on its own: a `workflow_dispatch` dry run of `publish.yml` shows the `release` job
skipped (dry run); a description of what a real (non-dry-run) run would do is reviewable in the
diff even though this loop never dispatches one for real.
