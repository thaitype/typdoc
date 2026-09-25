#!/usr/bin/env python3
"""Self-test for scripts/installer_fixture_server.py.

Exercises the fixture server's routing logic — the "latest" redirect, direct tag downloads,
missing files, and its request log — against a real bound HTTP server on 127.0.0.1, independent
of pages/install. The same "prove the mechanism before trusting it" shape as
scripts/test_windows_test_report.py's own self-test for scripts/windows_test_report.py.

This is the seam .chief/story-5/_contract/contract.md's Testing Decisions names for the
installer script's PR-time test: a local HTTP server serving fixture archives/checksums, pointed
at via TYPDOC_INSTALL_BASE_URL. This file proves the server itself is correct before
scripts/test_installer_posix.sh trusts it to drive pages/install end to end.

Run standalone:
    python scripts/test_installer_fixture_server.py -v
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from installer_fixture_server import FixtureServer  # noqa: E402


class FixtureServerTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.fixtures_dir = self._tmp.name
        self._write("v1.0.0", "typdoc-x.tar.gz", b"archive-bytes-v1")
        self._write("v1.0.0", "typdoc-x.tar.gz.sha256", b"deadbeef *typdoc-x.tar.gz\n")
        self._write("v2.0.0-second", "typdoc-x.tar.gz", b"archive-bytes-v2")

        self.log_path = os.path.join(self.fixtures_dir, "requests.log")
        self.server = FixtureServer(
            fixtures_dir=self.fixtures_dir,
            latest_tag="v1.0.0",
            log_path=self.log_path,
            host="127.0.0.1",
            port=0,
        )
        self.server.start()
        self.addCleanup(self.server.stop)
        self.base_url = f"http://127.0.0.1:{self.server.port}"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _write(self, tag: str, name: str, content: bytes) -> None:
        directory = os.path.join(self.fixtures_dir, tag)
        os.makedirs(directory, exist_ok=True)
        with open(os.path.join(directory, name), "wb") as f:
            f.write(content)

    def _get(self, path: str, follow_redirects: bool = True):
        url = f"{self.base_url}{path}"
        if follow_redirects:
            with urllib.request.urlopen(url) as resp:  # noqa: S310
                return resp.status, resp.read()
        opener = urllib.request.build_opener(_NoRedirect())
        try:
            resp = opener.open(url)
            return resp.status, resp.headers.get("Location")
        except urllib.error.HTTPError as exc:
            return exc.code, exc.headers.get("Location")

    def test_direct_download_serves_fixture_bytes(self) -> None:
        status, body = self._get("/releases/download/v1.0.0/typdoc-x.tar.gz")
        self.assertEqual(status, 200)
        self.assertEqual(body, b"archive-bytes-v1")

    def test_direct_download_of_second_fixture_version(self) -> None:
        status, body = self._get("/releases/download/v2.0.0-second/typdoc-x.tar.gz")
        self.assertEqual(status, 200)
        self.assertEqual(body, b"archive-bytes-v2")

    def test_latest_redirects_to_the_configured_tag(self) -> None:
        status, location = self._get(
            "/releases/latest/download/typdoc-x.tar.gz", follow_redirects=False
        )
        self.assertEqual(status, 302)
        self.assertEqual(location, "/releases/download/v1.0.0/typdoc-x.tar.gz")

    def test_latest_redirect_is_followed_to_the_right_bytes(self) -> None:
        status, body = self._get("/releases/latest/download/typdoc-x.tar.gz")
        self.assertEqual(status, 200)
        self.assertEqual(body, b"archive-bytes-v1")

    def test_missing_tag_is_404(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self._get("/releases/download/v9.9.9-does-not-exist/typdoc-x.tar.gz")
        self.assertEqual(ctx.exception.code, 404)

    def test_missing_file_within_existing_tag_is_404(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self._get("/releases/download/v1.0.0/does-not-exist.tar.gz")
        self.assertEqual(ctx.exception.code, 404)

    def test_unknown_path_is_404(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self._get("/nonsense")
        self.assertEqual(ctx.exception.code, 404)

    def test_server_bind_skips_the_reverse_dns_lookup(self) -> None:
        # http.server.HTTPServer's own server_bind() calls socket.getfqdn(host), which has been
        # observed to hang indefinitely on a sandboxed/restricted-network CI runner (see
        # _FastBindHTTPServer's own docstring). If that call ever creeps back in, server_name
        # stops being the literal bind host and becomes whatever the resolver returns instead
        # (typically not "127.0.0.1" -- often "localhost" or a fully-qualified name) -- this
        # would not by itself prove a hang can't happen again, but it is the one part of that
        # regression a fast, always-run unit test can actually catch.
        self.assertEqual(self.server._httpd.server_name, "127.0.0.1")  # noqa: SLF001

    def test_requests_are_logged_for_the_calling_test_to_assert_on(self) -> None:
        self._get("/releases/download/v1.0.0/typdoc-x.tar.gz")
        self._get("/releases/latest/download/typdoc-x.tar.gz")
        self.server.flush_log()
        with open(self.log_path, "r", encoding="utf-8") as f:
            lines = f.read().splitlines()
        self.assertIn("GET /releases/download/v1.0.0/typdoc-x.tar.gz", lines)
        self.assertIn("GET /releases/latest/download/typdoc-x.tar.gz", lines)


class _NoRedirect(urllib.request.HTTPErrorProcessor):
    def http_response(self, request, response):
        return response

    https_response = http_response


if __name__ == "__main__":
    unittest.main()
