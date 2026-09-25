#!/usr/bin/env python3
"""Self-test for scripts/extract_release_notes.py.

Exercises the CHANGELOG.md section-extraction logic against fixed sample changelog text,
independent of the repo's real CHANGELOG.md -- the same "prove the mechanism before trusting
it" shape as scripts/test.sh's own --self-test and scripts/test_windows_test_report.py.

Run standalone:
    python scripts/test_extract_release_notes.py -v
"""

from __future__ import annotations

import contextlib
import io
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from extract_release_notes import (  # noqa: E402
    VersionNotFoundError,
    extract_entry,
    main,
)

# Three versions, the same shape as the repo's real CHANGELOG.md: a "## [x.y.z] - date"
# heading per release, each followed by "###"-level subsections, newest first. Includes a
# "0.3.10" entry immediately above "0.3.1" specifically to prove exact-heading matching
# doesn't let "0.3.1" match as a prefix of "0.3.10".
SAMPLE_CHANGELOG = """# Changelog

All notable, user-visible changes are documented here.

## [0.3.10] - 2026-10-01

### Added

- A later, unrelated release that must never be picked up when asking for 0.3.1.

## [0.3.1] - 2026-09-25

### Added

- Prebuilt binaries for five platforms, attached to this release.
- Every released binary carries a SHA-256 checksum and a GitHub artifact attestation.

### Internal

- `dist-workspace.toml` configures the five build targets.

## [0.3.0] - 2026-09-24

### Added

- `namespaces` entries can now carry `!`-prefixed exclusions.
"""

# A changelog whose requested version is also its last entry -- extraction must run to EOF,
# not require a following "## [" heading to know where to stop.
SAMPLE_CHANGELOG_LAST_ENTRY = """# Changelog

## [0.2.0] - 2026-08-01

### Added

- Something in a later release.

## [0.1.0] - 2026-07-01

### Added

- The first release.
"""


class ExtractEntryTests(unittest.TestCase):
    def test_extracts_middle_entry_between_two_headings(self) -> None:
        body = extract_entry(SAMPLE_CHANGELOG, "0.3.1")
        self.assertIn("Prebuilt binaries for five platforms", body)
        self.assertIn("dist-workspace.toml", body)
        # Never bleeds into the neighboring entries.
        self.assertNotIn("must never be picked up", body)
        self.assertNotIn("namespaces", body)

    def test_does_not_prefix_match_a_longer_version(self) -> None:
        body = extract_entry(SAMPLE_CHANGELOG, "0.3.1")
        self.assertNotIn("0.3.10", body)

    def test_extracts_final_entry_through_end_of_file(self) -> None:
        body = extract_entry(SAMPLE_CHANGELOG_LAST_ENTRY, "0.1.0")
        self.assertIn("The first release", body)
        self.assertNotIn("Something in a later release", body)

    def test_trims_leading_and_trailing_blank_lines(self) -> None:
        body = extract_entry(SAMPLE_CHANGELOG, "0.3.1")
        self.assertFalse(body.startswith("\n"))
        self.assertFalse(body.endswith("\n\n"))
        self.assertTrue(body.endswith("\n"))

    def test_does_not_include_the_version_heading_line_itself(self) -> None:
        body = extract_entry(SAMPLE_CHANGELOG, "0.3.1")
        self.assertNotIn("## [0.3.1]", body)

    def test_raises_when_version_is_absent(self) -> None:
        with self.assertRaises(VersionNotFoundError):
            extract_entry(SAMPLE_CHANGELOG, "9.9.9")


class MainTests(unittest.TestCase):
    def _write_changelog(self, text: str) -> str:
        fd, path = tempfile.mkstemp(suffix=".md")
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(text)
        return path

    def _run_main(self, argv: list[str]) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = main(argv)
        return code, stdout.getvalue(), stderr.getvalue()

    def test_prints_entry_to_stdout_and_returns_zero(self) -> None:
        path = self._write_changelog(SAMPLE_CHANGELOG)
        try:
            code, stdout, _stderr = self._run_main(["extract_release_notes.py", path, "0.3.1"])
        finally:
            os.remove(path)
        self.assertEqual(code, 0)
        self.assertIn("Prebuilt binaries for five platforms", stdout)

    def test_missing_version_errors_loudly_not_silently(self) -> None:
        path = self._write_changelog(SAMPLE_CHANGELOG)
        try:
            code, stdout, stderr = self._run_main(["extract_release_notes.py", path, "9.9.9"])
        finally:
            os.remove(path)
        self.assertEqual(code, 1)
        self.assertEqual(stdout, "")
        self.assertIn("9.9.9", stderr)

    def test_missing_file_errors_loudly_not_a_traceback(self) -> None:
        code, stdout, stderr = self._run_main(
            ["extract_release_notes.py", "/no/such/CHANGELOG.md", "0.3.1"]
        )
        self.assertEqual(code, 1)
        self.assertEqual(stdout, "")
        self.assertIn("could not read", stderr)

    def test_wrong_arg_count_is_a_usage_error(self) -> None:
        code, _stdout, stderr = self._run_main(["extract_release_notes.py", "only-one-arg"])
        self.assertEqual(code, 2)
        self.assertIn("usage", stderr)


if __name__ == "__main__":
    unittest.main()
