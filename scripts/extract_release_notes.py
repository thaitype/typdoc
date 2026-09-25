#!/usr/bin/env python3
"""Extracts one version's entry out of CHANGELOG.md, for use as a generated GitHub Release
body.

Used by the `release` job in .github/workflows/publish.yml: earlier releases on this repo
(v0.1.0, v0.2.0, v0.3.0) had their release body hand-written by whoever cut the release. This
script generates that body instead, straight from the CHANGELOG.md entry already written for
the version being released, so the two can never drift apart.

An entry runs from its own "## [<version>] ..." heading (exclusive -- the release page already
shows the tag/version in its own title) up to, but not including, the next "## [" heading, or
end of file if there isn't one. The heading match is exact-bracket, so asking for "0.3.1" never
picks up a neighboring "0.3.10" entry.

Run standalone:
    python extract_release_notes.py CHANGELOG.md 0.3.1

See scripts/test_extract_release_notes.py for the self-test that exercises the extraction logic
below against fixed sample changelog text, independent of the repo's real CHANGELOG.md -- the
same "prove the mechanism before trusting it" shape as scripts/test.sh's own --self-test.
"""

from __future__ import annotations

import re
import sys


class VersionNotFoundError(ValueError):
    """Raised when CHANGELOG.md has no "## [<version>]" heading for the requested version."""


def extract_entry(changelog_text: str, version: str) -> str:
    heading = re.compile(r"^## \[" + re.escape(version) + r"\]")
    any_heading = re.compile(r"^## \[")

    lines = changelog_text.splitlines()
    start = None
    for i, line in enumerate(lines):
        if heading.match(line):
            start = i + 1
            break
    if start is None:
        raise VersionNotFoundError(
            f"no CHANGELOG.md entry found for version '{version}' "
            f"(expected a heading line matching '## [{version}]...')"
        )

    end = len(lines)
    for j in range(start, len(lines)):
        if any_heading.match(lines[j]):
            end = j
            break

    body = "\n".join(lines[start:end]).strip("\n")
    return body + "\n"


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: extract_release_notes.py <changelog-path> <version>", file=sys.stderr)
        return 2

    changelog_path, version = argv[1], argv[2]

    try:
        with open(changelog_path, "r", encoding="utf-8") as f:
            changelog_text = f.read()
    except OSError as exc:
        print(f"extract_release_notes: could not read '{changelog_path}': {exc}", file=sys.stderr)
        return 1

    try:
        body = extract_entry(changelog_text, version)
    except VersionNotFoundError as exc:
        print(f"extract_release_notes: {exc}", file=sys.stderr)
        return 1

    print(body, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
