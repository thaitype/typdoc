#!/usr/bin/env python3
"""Render templates/skills/ into skills/, filling in the typdoc version.

`skills/` is what `npx skills@latest add thaitype/typdoc` installs, so it is committed. It is written
from `templates/skills/`, never edited by hand: the version it names comes from
`crates/typdoc/Cargo.toml`, the one place a release sets it.

Nothing in the rendered files says they are generated: an agent reads every line of a skill,
and a note meant for maintainers would be an instruction it cannot follow. The CI job that runs
`--check` is what stops a hand edit, and `.gitattributes` marks `skills/` as generated on GitHub.

A template names a value as `{{name}}`. The only value is `version`. Any other `{{...}}` is an
error rather than text copied through, so a misspelled name cannot reach a published skill, and
so is a version written out (`1.2.3`), which a release would forget to bump.

Usage (from anywhere in the repository):
    python3 scripts/render_skills.py           # rewrite skills/ from templates/skills/
    python3 scripts/render_skills.py --check   # exit 1 if skills/ is not what it would write
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEMPLATES = Path("templates/skills")
OUTPUT = Path("skills")
VERSION_SOURCE = Path("crates/typdoc/Cargo.toml")

PLACEHOLDER = re.compile(r"\{\{\s*([^{}]*?)\s*\}\}")
# A version written out in a template is one a release forgets to bump.
LITERAL_VERSION = re.compile(r"\b\d+\.\d+\.\d+\b")


class TemplateError(Exception):
    pass


def read_version(root: Path) -> str:
    with open(root / VERSION_SOURCE, "rb") as f:
        return tomllib.load(f)["package"]["version"]


def fill(text: str, values: dict[str, str], template: str) -> str:
    literal = LITERAL_VERSION.search(text)
    if literal:
        line = text.count("\n", 0, literal.start()) + 1
        raise TemplateError(
            f"{template}:{line}: a literal version {literal.group(0)}; write {{{{version}}}}"
        )

    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in values:
            raise TemplateError(f"{template}: unknown placeholder {{{{{name}}}}}")
        return values[name]

    return PLACEHOLDER.sub(replace, text)



def render(root: Path) -> dict[Path, bytes]:
    """Every file skills/ should hold, by its path relative to skills/."""
    values = {"version": read_version(root)}
    rendered: dict[Path, bytes] = {}
    for source in sorted((root / TEMPLATES).rglob("*")):
        if not source.is_file():
            continue
        relative = source.relative_to(root / TEMPLATES)
        template = (TEMPLATES / relative).as_posix()
        if source.suffix == ".md":
            text = fill(source.read_text(encoding="utf-8"), values, template)
            rendered[relative] = text.encode("utf-8")
        else:
            rendered[relative] = source.read_bytes()
    return rendered


def current(root: Path) -> dict[Path, bytes]:
    out = root / OUTPUT
    if not out.exists():
        return {}
    return {
        path.relative_to(out): path.read_bytes() for path in out.rglob("*") if path.is_file()
    }


def differences(root: Path) -> list[str]:
    want, have = render(root), current(root)
    lines = []
    for path in sorted(want.keys() | have.keys()):
        shown = (OUTPUT / path).as_posix()
        if path not in have:
            lines.append(f"missing: {shown}")
        elif path not in want:
            lines.append(f"not from a template: {shown}")
        elif want[path] != have[path]:
            lines.append(f"differs: {shown}")
    return lines


def write(root: Path) -> None:
    want = render(root)
    out = root / OUTPUT
    for path in current(root).keys() - want.keys():
        (out / path).unlink()
    for path, data in want.items():
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    for folder in sorted((p for p in out.rglob("*") if p.is_dir()), reverse=True):
        if not any(folder.iterdir()):
            folder.rmdir()


def main(argv: list[str] | None = None, root: Path = ROOT) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check", action="store_true", help="report differences instead of writing"
    )
    args = parser.parse_args(argv)
    try:
        if args.check:
            found = differences(root)
            for line in found:
                print(line)
            if found:
                print("skills/ is out of date: run python3 scripts/render_skills.py")
                return 1
            return 0
        write(root)
        return 0
    except TemplateError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
