#!/usr/bin/env python3
"""Self-test for scripts/check_live_install.py.

Exercises both layers independently of the real https://typdoc.thaitype.dev:
  - the comparison/redirect-detection/retry logic against an in-memory fake fetcher (no
    network at all), and
  - urllib_fetch itself -- the one function that actually talks HTTP -- against a local
    http.server fixture, so a real 200/404 response is proven to be handled correctly, not
    just assumed.

Same "prove the mechanism before trusting it" shape as this repo's other self-tested scripts
(windows_test_report.py, extract_release_notes.py, installer_fixture_server.py). Run in CI as
its own step, before the real live check, in pages.yml's post-deploy job, in publish.yml's
post-release job is not relevant here (that job runs the real installers, not this script), and
in the scheduled live-check workflow.

Run standalone:
    python scripts/test_check_live_install.py -v
"""

from __future__ import annotations

import http.server
import os
import socket
import sys
import tempfile
import threading
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from check_live_install import (  # noqa: E402
    LiveCheckError,
    check_byte_identical,
    check_root_redirects,
    main,
    retrying,
    run_checks,
    urllib_fetch,
)


def _fake_fetch(pages: dict) -> "callable":
    """A fetch function backed by an in-memory dict, standing in for urllib_fetch: an unknown
    URL behaves like a real 404 (raises LiveCheckError), a known URL returns its bytes."""

    def fetch(url: str) -> bytes:
        if url not in pages:
            raise LiveCheckError(f"GET {url} returned HTTP 404, expected 200")
        return pages[url]

    return fetch


class CheckByteIdenticalTests(unittest.TestCase):
    def test_passes_on_match(self) -> None:
        fetch = _fake_fetch({"https://example/install": b"same-bytes"})
        result = check_byte_identical(
            fetch, "https://example/install", b"same-bytes", "pages/install"
        )
        self.assertIn("ok:", result)

    def test_raises_on_mismatch(self) -> None:
        fetch = _fake_fetch({"https://example/install": b"live-bytes"})
        with self.assertRaises(LiveCheckError) as ctx:
            check_byte_identical(
                fetch, "https://example/install", b"local-bytes", "pages/install"
            )
        self.assertIn("does not match", str(ctx.exception))
        self.assertIn("pages/install", str(ctx.exception))

    def test_raises_on_unreachable_url(self) -> None:
        fetch = _fake_fetch({})
        with self.assertRaises(LiveCheckError):
            check_byte_identical(fetch, "https://example/install", b"x", "pages/install")


class CheckRootRedirectsTests(unittest.TestCase):
    def test_passes_when_target_present(self) -> None:
        body = (
            b'<meta http-equiv="refresh" content="0; url=https://github.com/thaitype/typdoc" />'
        )
        fetch = _fake_fetch({"https://example/": body})
        result = check_root_redirects(
            fetch, "https://example/", "https://github.com/thaitype/typdoc"
        )
        self.assertIn("ok:", result)

    def test_raises_when_target_missing(self) -> None:
        fetch = _fake_fetch({"https://example/": b"<html>nothing here</html>"})
        with self.assertRaises(LiveCheckError) as ctx:
            check_root_redirects(
                fetch, "https://example/", "https://github.com/thaitype/typdoc"
            )
        self.assertIn("does not redirect", str(ctx.exception))


class RunChecksTests(unittest.TestCase):
    def test_all_pass_returns_two_ok_lines(self) -> None:
        fetch = _fake_fetch(
            {
                "https://example/install": b"sh-bytes",
                "https://example/": b"redirect to https://github.com/thaitype/typdoc",
            }
        )
        results = run_checks(fetch, "https://example", b"sh-bytes")
        self.assertEqual(len(results), 2)

    def test_stops_at_first_failure_and_names_it(self) -> None:
        fetch = _fake_fetch(
            {
                "https://example/install": b"WRONG-BYTES",
                "https://example/": b"redirect to https://github.com/thaitype/typdoc",
            }
        )
        with self.assertRaises(LiveCheckError) as ctx:
            run_checks(fetch, "https://example", b"sh-bytes")
        self.assertIn("pages/install", str(ctx.exception))

    def test_trailing_slash_on_base_url_does_not_double_up(self) -> None:
        fetch = _fake_fetch(
            {
                "https://example/install": b"sh-bytes",
                "https://example/": b"redirect to https://github.com/thaitype/typdoc",
            }
        )
        # Must not require "https://example//install" -- a trailing slash on --base-url is an
        # easy real-world mistake (e.g. typed by hand from a browser address bar) that must not
        # silently break every check.
        results = run_checks(fetch, "https://example/", b"sh-bytes")
        self.assertEqual(len(results), 2)


class RetryingTests(unittest.TestCase):
    def test_returns_immediately_on_first_success(self) -> None:
        calls = []

        def fetch(url: str) -> bytes:
            calls.append(url)
            return b"ok"

        wrapped = retrying(fetch, attempts=5, delay_seconds=0.0)
        self.assertEqual(wrapped("https://example/x"), b"ok")
        self.assertEqual(len(calls), 1)

    def test_retries_on_failure_then_succeeds(self) -> None:
        calls = {"n": 0}

        def flaky_fetch(url: str) -> bytes:
            calls["n"] += 1
            if calls["n"] < 3:
                raise LiveCheckError("transient")
            return b"ok"

        with mock.patch("check_live_install.time.sleep") as sleep_mock:
            wrapped = retrying(flaky_fetch, attempts=5, delay_seconds=1.0)
            self.assertEqual(wrapped("https://example/x"), b"ok")
        self.assertEqual(calls["n"], 3)
        self.assertEqual(sleep_mock.call_count, 2)

    def test_gives_up_after_exhausting_attempts(self) -> None:
        def always_fails(url: str) -> bytes:
            raise LiveCheckError("still down")

        with mock.patch("check_live_install.time.sleep"):
            wrapped = retrying(always_fails, attempts=3, delay_seconds=0.0)
            with self.assertRaises(LiveCheckError) as ctx:
                wrapped("https://example/x")
        self.assertIn("still down", str(ctx.exception))

    def test_rejects_zero_attempts(self) -> None:
        with self.assertRaises(ValueError):
            retrying(lambda url: b"", attempts=0, delay_seconds=0.0)


class _FixtureHandler(http.server.BaseHTTPRequestHandler):
    routes: dict = {}

    def log_message(self, fmt: str, *args) -> None:  # silence per-request logging in test output
        pass

    def do_GET(self) -> None:  # noqa: N802
        body = self.routes.get(self.path)
        if body is None:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class UrllibFetchTests(unittest.TestCase):
    """Proves urllib_fetch's real HTTP behavior against a local server -- not mocked -- since
    this is the one function in this module that actually talks HTTP."""

    def setUp(self) -> None:
        handler = type("_Bound", (_FixtureHandler,), {"routes": {"/ok": b"hello-world"}})
        self.httpd = http.server.HTTPServer(("127.0.0.1", 0), handler)
        self.httpd.socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        self.base = f"http://127.0.0.1:{self.httpd.server_address[1]}"

    def tearDown(self) -> None:
        self.httpd.shutdown()
        self.httpd.server_close()
        self.thread.join(timeout=5)

    def test_fetches_real_200_response_body(self) -> None:
        self.assertEqual(urllib_fetch(f"{self.base}/ok"), b"hello-world")

    def test_raises_live_check_error_on_404_not_an_uncaught_exception(self) -> None:
        with self.assertRaises(LiveCheckError):
            urllib_fetch(f"{self.base}/missing")

    def test_raises_live_check_error_on_connection_refused(self) -> None:
        # Port 1 on loopback: nothing listens there, so this proves the connection-failure path
        # (as distinct from the 404 path above) also raises LiveCheckError, not a raw
        # urllib.error.URLError leaking out of this module.
        with self.assertRaises(LiveCheckError):
            urllib_fetch("http://127.0.0.1:1/unreachable")


class MainTests(unittest.TestCase):
    def _write_local_pages(self, tmp_dir: str, install_sh: bytes) -> None:
        pages_dir = os.path.join(tmp_dir, "pages")
        os.makedirs(pages_dir, exist_ok=True)
        with open(os.path.join(pages_dir, "install"), "wb") as f:
            f.write(install_sh)

    def test_reports_success_when_all_checks_pass(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            self._write_local_pages(tmp, b"sh-bytes")
            fake = _fake_fetch(
                {
                    "https://example/install": b"sh-bytes",
                    "https://example/": b"redirect to https://github.com/thaitype/typdoc",
                }
            )
            with mock.patch("check_live_install.urllib_fetch", fake):
                code = main(
                    [
                        "check_live_install.py",
                        "--base-url",
                        "https://example",
                        "--repo-root",
                        tmp,
                        "--attempts",
                        "1",
                    ]
                )
            self.assertEqual(code, 0)

    def test_content_mismatch_exits_nonzero_and_names_the_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            self._write_local_pages(tmp, b"sh-bytes")
            fake = _fake_fetch(
                {
                    "https://example/install": b"DIFFERENT",
                    "https://example/": b"redirect to https://github.com/thaitype/typdoc",
                }
            )
            stderr = _CapturedStderr()
            with mock.patch("check_live_install.urllib_fetch", fake), stderr:
                code = main(
                    [
                        "check_live_install.py",
                        "--base-url",
                        "https://example",
                        "--repo-root",
                        tmp,
                        "--attempts",
                        "1",
                    ]
                )
            self.assertEqual(code, 1)
            self.assertIn("pages/install", stderr.value)

    def test_missing_local_files_errors_loudly(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            code = main(
                [
                    "check_live_install.py",
                    "--base-url",
                    "https://example",
                    "--repo-root",
                    tmp,
                ]
            )
            self.assertEqual(code, 2)


class _CapturedStderr:
    """Minimal context manager capturing sys.stderr text, so a test can assert on the printed
    error message without pulling in a heavier capture framework."""

    def __enter__(self) -> "_CapturedStderr":
        import io

        self._orig = sys.stderr
        self._buf = io.StringIO()
        sys.stderr = self._buf
        return self

    def __exit__(self, *exc_info) -> None:
        sys.stderr = self._orig
        self.value = self._buf.getvalue()


if __name__ == "__main__":
    unittest.main()
