#!/usr/bin/env python3
"""Self-test for scripts/render_skills.py.

Runs the renderer against a throwaway repository layout, independent of the real
templates/skills/, the same "prove the mechanism before trusting it" shape as the other
scripts/test_*.py files.

Run standalone:
    python3 scripts/test_render_skills.py -v
"""

from __future__ import annotations

import contextlib
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from render_skills import TemplateError, add_notice, fill, main  # noqa: E402

CARGO = '[package]\nname = "typdoc"\nversion = "9.8.7"\n'
SKILL = "---\nname: demo\n---\n\nWritten for demo {{version}}; pin v{{ version }}.\n"


def make_repo(root: Path, skill: str = SKILL) -> None:
    (root / "crates/typdoc").mkdir(parents=True)
    (root / "crates/typdoc/Cargo.toml").write_text(CARGO)
    (root / "templates/skills/demo/references").mkdir(parents=True)
    (root / "templates/skills/demo/SKILL.md").write_text(skill)
    (root / "templates/skills/demo/references/a.md").write_text("no placeholder\n")


def run(root: Path, *argv: str) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        code = main(list(argv), root=root)
    return code, out.getvalue()


class Fill(unittest.TestCase):
    def test_a_known_placeholder_is_replaced_with_or_without_spaces(self):
        self.assertEqual(fill("{{version}} {{ version }}", {"version": "1"}, "t"), "1 1")

    def test_an_unknown_placeholder_is_an_error_not_copied_through(self):
        with self.assertRaises(TemplateError):
            fill("{{verion}}", {"version": "1"}, "t")

    def test_single_braces_and_json_are_left_alone(self):
        text = '{"a":{"b":[1]}} {x}'
        self.assertEqual(fill(text, {"version": "1"}, "t"), text)


class Notice(unittest.TestCase):
    def test_the_notice_goes_after_the_frontmatter(self):
        out = add_notice("---\nname: x\n---\nbody\n", "t.md")
        self.assertTrue(out.startswith("---\nname: x\n---\n<!-- Generated from t.md"))

    def test_a_file_without_frontmatter_gets_the_notice_first(self):
        self.assertTrue(add_notice("body\n", "t.md").startswith("<!-- Generated from t.md"))


class Render(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        make_repo(self.root)

    def tearDown(self):
        self.tmp.cleanup()

    def test_render_writes_the_version_from_cargo_toml(self):
        self.assertEqual(run(self.root)[0], 0)
        skill = (self.root / "skills/demo/SKILL.md").read_text()
        self.assertIn("Written for demo 9.8.7; pin v9.8.7.", skill)
        self.assertNotIn("{{", skill)
        self.assertTrue((self.root / "skills/demo/references/a.md").exists())

    def test_check_passes_right_after_a_render(self):
        run(self.root)
        self.assertEqual(run(self.root, "--check")[0], 0)

    def test_check_fails_when_the_version_is_bumped_without_rendering(self):
        run(self.root)
        cargo = self.root / "crates/typdoc/Cargo.toml"
        cargo.write_text(CARGO.replace("9.8.7", "9.8.8"))
        code, out = run(self.root, "--check")
        self.assertEqual(code, 1)
        self.assertIn("differs: skills/demo/SKILL.md", out)

    def test_check_fails_when_skills_is_edited_by_hand(self):
        run(self.root)
        (self.root / "skills/demo/references/a.md").write_text("edited\n")
        self.assertEqual(run(self.root, "--check")[0], 1)

    def test_check_names_a_file_no_template_produces(self):
        run(self.root)
        (self.root / "skills/demo/stray.md").write_text("x\n")
        code, out = run(self.root, "--check")
        self.assertEqual(code, 1)
        self.assertIn("not from a template: skills/demo/stray.md", out)

    def test_render_removes_a_file_whose_template_is_gone(self):
        run(self.root)
        (self.root / "templates/skills/demo/references/a.md").unlink()
        run(self.root)
        self.assertFalse((self.root / "skills/demo/references").exists())

    def test_an_unknown_placeholder_exits_2_and_writes_nothing(self):
        (self.root / "templates/skills/demo/SKILL.md").write_text("{{nope}}\n")
        self.assertEqual(run(self.root)[0], 2)
        self.assertFalse((self.root / "skills").exists())


if __name__ == "__main__":
    unittest.main()
