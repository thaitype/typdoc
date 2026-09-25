#!/usr/bin/env python3
"""Self-test for scripts/windows_test_report.py.

Exercises the parsing/rendering logic against fixed sample `cargo test`
output, independent of any real `cargo test` run -- the same "prove the
mechanism before trusting it" shape as scripts/test.sh's own --self-test.
Run in CI as its own step, before the real Windows test run, in the
windows-pass-rate job of .github/workflows/ci.yml.

Run standalone:
    python scripts/test_windows_test_report.py -v
"""

from __future__ import annotations

import contextlib
import io
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from windows_test_report import (  # noqa: E402
    ZeroTestsError,
    check_nonzero,
    main,
    render_step_summary,
    summarize,
)

# Three "test result:" lines, the same shape cargo produces for unit tests,
# integration tests, and doctests in one workspace run: 2+5+4 = 11 passed,
# 1+0+0 = 1 failed, across 3 suites.
SAMPLE_OUTPUT = """
running 3 tests
test foo ... ok
test bar ... ok
test baz ... FAILED

failures:

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 5 tests
test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.02s

   Doc-tests typdoc
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
"""


class SummarizeTests(unittest.TestCase):
    def test_sums_across_multiple_suites(self) -> None:
        summary = summarize(SAMPLE_OUTPUT)
        self.assertEqual(summary.passed, 11)
        self.assertEqual(summary.failed, 1)
        self.assertEqual(summary.suite_count, 3)
        self.assertEqual(summary.total, 12)

    def test_no_result_lines_gives_zero_summary(self) -> None:
        summary = summarize("error: could not compile `typdoc`\n")
        self.assertEqual(summary.passed, 0)
        self.assertEqual(summary.failed, 0)
        self.assertEqual(summary.suite_count, 0)
        self.assertEqual(summary.total, 0)

    def test_rate_percent_rounds_to_one_decimal(self) -> None:
        summary = summarize(SAMPLE_OUTPUT)
        # 11 / 12 * 100 = 91.666... -> 91.7
        self.assertAlmostEqual(summary.rate_percent, 91.7)

    def test_rate_percent_raises_on_zero_total(self) -> None:
        summary = summarize("no test result lines here\n")
        with self.assertRaises(ZeroTestsError):
            _ = summary.rate_percent


class CheckNonzeroTests(unittest.TestCase):
    def test_raises_on_zero_total(self) -> None:
        summary = summarize("no test result lines here\n")
        with self.assertRaises(ZeroTestsError):
            check_nonzero(summary)

    def test_does_not_raise_when_total_is_nonzero(self) -> None:
        summary = summarize(
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
        )
        check_nonzero(summary)  # must not raise


class RenderStepSummaryTests(unittest.TestCase):
    def test_includes_counts_and_rate(self) -> None:
        summary = summarize(SAMPLE_OUTPUT)
        rendered = render_step_summary(summary)
        self.assertIn("11 / 12", rendered)
        self.assertIn("91.7%", rendered)
        self.assertIn("3 suite(s)", rendered)
        self.assertIn("never blocks a merge", rendered)

    def test_raises_instead_of_rendering_zero_total(self) -> None:
        summary = summarize("no test result lines here\n")
        with self.assertRaises(ZeroTestsError):
            render_step_summary(summary)


class MainTests(unittest.TestCase):
    def _run_main(self, output_path: str) -> tuple[int, str, str, str]:
        """Runs main() against output_path, returning (exit_code, stdout, stderr,
        step_summary_contents), with $GITHUB_STEP_SUMMARY pointed at a temp file for
        the duration of the call."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            summary_path = os.path.join(tmp_dir, "step-summary.md")
            old_env = os.environ.get("GITHUB_STEP_SUMMARY")
            os.environ["GITHUB_STEP_SUMMARY"] = summary_path
            try:
                stdout, stderr = io.StringIO(), io.StringIO()
                with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                    code = main(["windows_test_report.py", output_path])
                summary_text = ""
                if os.path.exists(summary_path):
                    with open(summary_path, "r", encoding="utf-8") as f:
                        summary_text = f.read()
                return code, stdout.getvalue(), stderr.getvalue(), summary_text
            finally:
                if old_env is None:
                    os.environ.pop("GITHUB_STEP_SUMMARY", None)
                else:
                    os.environ["GITHUB_STEP_SUMMARY"] = old_env

    def test_reports_success_and_writes_step_summary(self) -> None:
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".txt") as f:
            f.write(SAMPLE_OUTPUT)
            path = f.name
        try:
            code, stdout, _stderr, summary_text = self._run_main(path)
        finally:
            os.remove(path)
        self.assertEqual(code, 0)
        self.assertIn("11 / 12", stdout)
        self.assertIn("91.7%", summary_text)

    def test_zero_result_lines_errors_loudly(self) -> None:
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".txt") as f:
            f.write("error: could not compile `typdoc`\n")
            path = f.name
        try:
            code, _stdout, stderr, summary_text = self._run_main(path)
        finally:
            os.remove(path)
        self.assertEqual(code, 1)
        self.assertIn("found zero tests", stderr)
        self.assertIn("ERROR: zero tests found", summary_text)

    def test_missing_output_file_is_treated_as_zero_tests_not_a_crash(self) -> None:
        # A test step that never even produced output (e.g. `cargo` not found before
        # it could run) must read the same as zero tests found -- a loud, clear error,
        # not an uncaught traceback.
        missing_path = os.path.join(tempfile.mkdtemp(), "does-not-exist.txt")
        code, _stdout, stderr, summary_text = self._run_main(missing_path)
        self.assertEqual(code, 1)
        self.assertIn("could not read", stderr)
        self.assertIn("found zero tests", stderr)
        self.assertIn("ERROR: zero tests found", summary_text)


if __name__ == "__main__":
    unittest.main()
