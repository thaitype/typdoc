#!/usr/bin/env python3
"""Render templates/skills/ into skills/, filling in the typdoc version.

`skills/` is what `npx skills add thaitype/typdoc` installs, so it is committed. It is written
from `templates/skills/`, never edited by hand: the version it names comes from
`crates/typdoc/Cargo.toml`, the one place a release sets it.

A template names a value as `{{name}}`. The only value is `version`. Any other `{{...}}` is an
error rather than text copied through, so a misspelled name cannot reach a published skill.

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
NOTICE = (
    "<!-- Generated from {template} by scripts/render_skills.py. "
    "Edit the template, not this file. -->\n"
)


class TemplateError(Exception):
    pass


def read_version(root: Path) -> str:
    with open(root / VERSION_SOURCE, "rb") as f:
        return tomllib.load(f)["package"]["version"]


def fill(text: str, values: dict[str, str], template: str) -> str:
    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in values:
            raise TemplateError(f"{template}: unknown placeholder {{{{{name}}}}}")
        return values[name]

    return PLACEHOLDER.sub(replace, text)


def add_notice(text: str, template: str) -> str:
    """The notice goes after the frontmatter, which a skill loader expects on the first line."""
    notice = NOTICE.format(template=template)
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            cut = end + len("\n---\n")
            return text[:cut] + notice + text[cut:]
    return notice + text


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
            rendered[relative] = add_notice(text, template).encode("utf-8")
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
