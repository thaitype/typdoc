#!/usr/bin/env python3
"""Local fixture HTTP server for pages/install's PR-time test.

No release of typdoc exists until 0.3.1 is published, so the installer script's PR-time test
can't hit real GitHub Releases (see .chief/story-5/_contract/contract.md's Testing Decisions).
Instead this server serves a GitHub-Releases-shaped URL surface from a local fixtures
directory, and the installer script is pointed at it via TYPDOC_INSTALL_BASE_URL:

    /releases/latest/download/<name>   -> 302 redirect to /releases/download/<latest tag>/<name>
    /releases/download/<tag>/<name>    -> serves fixtures_dir/<tag>/<name>, 404 if missing

The fixtures directory holds one subdirectory per release tag, each containing the archive +
`.sha256` files an installer would download for it. scripts/test_installer_posix.sh populates
it from ticket 01's PR-built dist-build archives plus a second fixture tag (reusing the same
bytes under a different tag, per the contract's own "a second fixture 'version'" wording), to
prove TYPDOC_VERSION pins to an explicit tag instead of following the latest redirect.

Every request is appended to a log file, one line per request, so a calling test can assert
which URL shape was actually hit (e.g. that a TYPDOC_VERSION pin used the direct
/releases/download/<tag>/... path and never touched /releases/latest/...), not just that some
file ended up installed.

See scripts/test_installer_fixture_server.py for the self-test that exercises this module's
routing directly, independent of the installer script.

Run standalone:
    installer_fixture_server.py <fixtures_dir> <latest_tag> --log <path> [--port N]
"""

from __future__ import annotations

import argparse
import http.server
import os
import socket
import socketserver
import sys
import threading


class _Handler(http.server.BaseHTTPRequestHandler):
    # Set by FixtureServer before serving: the directory holding one subdirectory per tag, the
    # tag "latest" resolves to, and the log file every request is appended to.
    fixtures_dir: str
    latest_tag: str
    log_path: str

    protocol_version = "HTTP/1.1"

    def log_message(self, fmt: str, *args) -> None:  # noqa: A003
        sys.stderr.write("[installer-fixture-server] " + (fmt % args) + "\n")

    def _log_request_path(self, path: str) -> None:
        with open(self.log_path, "a", encoding="utf-8") as f:
            f.write(f"{self.command} {path}\n")

    def do_GET(self) -> None:  # noqa: N802
        path = self.path.split("?", 1)[0]
        self._log_request_path(path)

        latest_prefix = "/releases/latest/download/"
        download_prefix = "/releases/download/"

        if path.startswith(latest_prefix):
            name = path[len(latest_prefix):]
            if not name:
                self.send_error(404)
                return
            target = f"{download_prefix}{self.latest_tag}/{name}"
            self.send_response(302)
            self.send_header("Location", target)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return

        if path.startswith(download_prefix):
            rest = path[len(download_prefix):]
            if "/" not in rest:
                self.send_error(404)
                return
            tag, name = rest.split("/", 1)
            if not tag or not name:
                self.send_error(404)
                return
            file_path = os.path.join(self.fixtures_dir, tag, name)
            # Guard against a path that escapes fixtures_dir via "..": this server only ever
            # needs to serve files directly under <fixtures_dir>/<tag>/.
            if os.path.commonpath(
                [os.path.abspath(file_path), os.path.abspath(self.fixtures_dir)]
            ) != os.path.abspath(self.fixtures_dir):
                self.send_error(404)
                return
            if not os.path.isfile(file_path):
                self.send_error(404)
                return
            with open(file_path, "rb") as f:
                body = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        self.send_error(404)


class _FastBindHTTPServer(http.server.HTTPServer):
    """`http.server.HTTPServer` minus its own `server_bind()`'s reverse-DNS lookup.

    `HTTPServer.server_bind()` calls `socket.getfqdn(host)` to set `self.server_name` -- useful
    for a real server that reports its own hostname, but a resolver call this fixture server
    has no need for and never uses (nothing here reads `server_name`). Confirmed hanging this
    server indefinitely on a GitHub-hosted macOS runner (diagnosed by instrumenting the calling
    shell script in a prior commit and reading `ps`: the process had bound and was idling
    normally, never reaching this class's own `print(server.port)` caller at all -- consistent
    with being stuck inside `__init__` -> `server_bind()`, before that line is ever reached).
    `ubuntu-latest` has not shown this; sandboxed/restricted-network reverse-DNS hangs are a
    known, environment-specific CPython `http.server` gotcha, not unique to this script.
    """

    def server_bind(self) -> None:
        socketserver.TCPServer.server_bind(self)
        host, port = self.server_address[:2]
        self.server_name = host
        self.server_port = port


class FixtureServer:
    """Runs the fixture HTTP server on a background thread.

    Usage:
        server = FixtureServer(fixtures_dir=..., latest_tag=..., log_path=...)
        server.start()
        ...
        server.stop()
    """

    def __init__(
        self,
        fixtures_dir: str,
        latest_tag: str,
        log_path: str,
        host: str = "127.0.0.1",
        port: int = 0,
    ) -> None:
        self.fixtures_dir = fixtures_dir
        self.latest_tag = latest_tag
        self.log_path = log_path
        self.host = host
        self.requested_port = port
        self._httpd: _FastBindHTTPServer | None = None
        self._thread: threading.Thread | None = None

    @property
    def port(self) -> int:
        assert self._httpd is not None, "FixtureServer.start() was not called"
        return self._httpd.server_address[1]

    def start(self) -> None:
        # Ensure the log file exists (and is empty) before any request can be logged to it.
        os.makedirs(os.path.dirname(os.path.abspath(self.log_path)) or ".", exist_ok=True)
        open(self.log_path, "a", encoding="utf-8").close()

        handler = type(
            "_BoundHandler",
            (_Handler,),
            {
                "fixtures_dir": self.fixtures_dir,
                "latest_tag": self.latest_tag,
                "log_path": self.log_path,
            },
        )
        self._httpd = _FastBindHTTPServer((self.host, self.requested_port), handler)
        self._httpd.socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        if self._httpd is not None:
            self._httpd.shutdown()
            self._httpd.server_close()
        if self._thread is not None:
            self._thread.join(timeout=5)

    def flush_log(self) -> None:
        """No-op: requests are appended synchronously. Kept so callers can express intent
        ("I'm about to read the log now") without depending on that implementation detail."""


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixtures_dir")
    parser.add_argument("latest_tag")
    parser.add_argument("--log", required=True, dest="log_path")
    parser.add_argument("--port", type=int, default=0)
    args = parser.parse_args(argv[1:])

    server = FixtureServer(
        fixtures_dir=args.fixtures_dir,
        latest_tag=args.latest_tag,
        log_path=args.log_path,
        port=args.port,
    )
    server.start()
    # A calling shell/PowerShell script captures this single line to learn the bound port.
    print(server.port)
    sys.stdout.flush()
    sys.stderr.write(
        f"[installer-fixture-server] serving {args.fixtures_dir} on "
        f"http://127.0.0.1:{server.port} (latest={args.latest_tag})\n"
    )
    sys.stderr.flush()
    try:
        server._thread.join()  # noqa: SLF001
    except KeyboardInterrupt:
        server.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
