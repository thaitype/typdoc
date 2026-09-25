#!/usr/bin/env python3
"""Confirms the live typdoc.thaitype.dev Pages site matches this repo.

Used by three callers, all against the real https://typdoc.thaitype.dev (see
.chief/story-5/_contract/contract.md's "CI: live-URL install check"):
  - pages.yml's post-deploy step, run for real on every push that deploys Pages;
  - a scheduled workflow, run independently of any deploy or release to catch drift (cert
    expiry, DNS, GitHub Pages outage) between deploys;
  - ad hoc, by hand, e.g. right after a DNS or Pages-settings change.

Three checks, in this order (the first failure stops the run -- see run_checks below):
  1. GET <base-url>/install is byte-identical to this repo's pages/install.
  2. GET <base-url>/install.ps1 is byte-identical to this repo's pages/install.ps1.
  3. GET <base-url>/ contains the expected redirect target. GitHub Pages serves static files
     only (no server-side HTTP redirect), so pages/index.html redirects client-side via a
     meta-refresh + a JS fallback (see its own header comment) -- there is no HTTP 3xx to follow
     here, so this checks that the served page actually names the redirect target, not that the
     response is a redirect status.

Split into two layers so each can be proven independently (see scripts/test_check_live_install.py):
  - run_checks/check_byte_identical/check_root_redirects take a `fetch` function as a parameter,
    so the comparison/redirect-detection logic itself is tested against an in-memory fake
    fetcher, with no network involved at all;
  - urllib_fetch is the one function that actually talks HTTP, and is proven separately against
    a local http.server fixture -- the same "prove the mechanism before trusting it" shape as
    this repo's other self-tested scripts (windows_test_report.py, extract_release_notes.py,
    installer_fixture_server.py).

Run standalone:
    python check_live_install.py --base-url https://typdoc.thaitype.dev --repo-root .

See scripts/test_check_live_install.py for the self-test, run as its own CI step before every
real invocation of this script.
"""

from __future__ import annotations

import argparse
import os
import sys
import time
import urllib.error
import urllib.request
from typing import Callable

DEFAULT_REDIRECT_TARGET = "https://github.com/thaitype/typdoc"

FetchFn = Callable[[str], bytes]


class LiveCheckError(RuntimeError):
    """Raised when a live-URL check fails: unreachable URL, content mismatch, or a missing
    redirect. The message always names which URL and which check failed."""


def urllib_fetch(url: str) -> bytes:
    """Fetches url and returns its body, raising LiveCheckError (never an uncaught exception)
    on anything but a 200 response."""
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:  # noqa: S310 (fixed https:// URLs)
            if resp.status != 200:
                raise LiveCheckError(f"GET {url} returned HTTP {resp.status}, expected 200")
            return resp.read()
    except urllib.error.HTTPError as exc:
        # HTTPError is itself an open, file-like response object (e.g. a 404's body) -- close
        # it explicitly so it isn't left for the garbage collector to warn about.
        exc.close()
        raise LiveCheckError(f"GET {url} returned HTTP {exc.code}, expected 200") from exc
    except urllib.error.URLError as exc:
        raise LiveCheckError(f"GET {url} failed: {exc}") from exc


def retrying(fetch: FetchFn, attempts: int, delay_seconds: float) -> FetchFn:
    """Wraps fetch so a transient failure (e.g. brief CDN propagation lag right after a Pages
    deploy) doesn't fail the whole check on its first attempt. Only retries on LiveCheckError
    (unreachable / non-200) -- a content mismatch is a real bug, never transient, and is never
    retried here; run_checks raises it straight through on the first try."""
    if attempts < 1:
        raise ValueError("attempts must be >= 1")

    def wrapped(url: str) -> bytes:
        last_error: LiveCheckError | None = None
        for attempt in range(1, attempts + 1):
            try:
                return fetch(url)
            except LiveCheckError as exc:
                last_error = exc
                if attempt < attempts:
                    time.sleep(delay_seconds)
        assert last_error is not None
        raise last_error

    return wrapped


def check_byte_identical(fetch: FetchFn, url: str, expected: bytes, label: str) -> str:
    live = fetch(url)
    if live != expected:
        raise LiveCheckError(
            f"{label} at {url} does not match the repo copy byte-for-byte "
            f"({len(live)} bytes live vs {len(expected)} bytes local)"
        )
    return f"ok: {url} matches {label} ({len(live)} bytes)"


def check_root_redirects(fetch: FetchFn, url: str, target: str) -> str:
    body = fetch(url).decode("utf-8", errors="replace")
    if target not in body:
        raise LiveCheckError(
            f"{url} does not redirect to {target} (that URL does not appear anywhere in the "
            "response body -- no meta-refresh, canonical link, or JS redirect naming it)"
        )
    return f"ok: {url} redirects to {target}"


def run_checks(
    fetch: FetchFn,
    base_url: str,
    install_sh: bytes,
    install_ps1: bytes,
    redirect_target: str = DEFAULT_REDIRECT_TARGET,
) -> list[str]:
    """Runs every live-URL check against base_url, returning one "ok: ..." line per check on
    success. Raises LiveCheckError on the first failure, naming exactly which check failed, so a
    caller never has to guess which of the three broke."""
    base = base_url.rstrip("/")
    return [
        check_byte_identical(fetch, f"{base}/install", install_sh, "pages/install"),
        check_byte_identical(fetch, f"{base}/install.ps1", install_ps1, "pages/install.ps1"),
        check_root_redirects(fetch, f"{base}/", redirect_target),
    ]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", required=True, help="e.g. https://typdoc.thaitype.dev")
    parser.add_argument(
        "--repo-root", required=True, help="repo root containing pages/install and pages/install.ps1"
    )
    parser.add_argument("--redirect-target", default=DEFAULT_REDIRECT_TARGET)
    parser.add_argument(
        "--attempts",
        type=int,
        default=5,
        help="retry each fetch this many times total before giving up (default: 5)",
    )
    parser.add_argument(
        "--delay-seconds",
        type=float,
        default=3.0,
        help="seconds to wait between retry attempts (default: 3.0)",
    )
    args = parser.parse_args(argv[1:])

    install_path = os.path.join(args.repo_root, "pages", "install")
    install_ps1_path = os.path.join(args.repo_root, "pages", "install.ps1")
    try:
        with open(install_path, "rb") as f:
            install_sh = f.read()
        with open(install_ps1_path, "rb") as f:
            install_ps1 = f.read()
    except OSError as exc:
        print(f"check_live_install: could not read local pages/ files: {exc}", file=sys.stderr)
        return 2

    fetch = retrying(urllib_fetch, attempts=args.attempts, delay_seconds=args.delay_seconds)

    try:
        results = run_checks(fetch, args.base_url, install_sh, install_ps1, args.redirect_target)
    except LiveCheckError as exc:
        print(f"check_live_install: {exc}", file=sys.stderr)
        return 1

    for line in results:
        print(line)
    print("all live-install checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
