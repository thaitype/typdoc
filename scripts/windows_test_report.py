#!/usr/bin/env python3
"""Parses `cargo test` output and reports a Windows pass-rate summary.

Used by the Windows pass-rate job in .github/workflows/ci.yml. That job is a
non-blocking measurement, not a gate: it reports how many of the suite's
tests currently pass on Windows, it does not require that they all do.

This script is the part of that job that must never itself fail silently: if
it finds zero "test result:" lines, that means the captured run produced no
test output at all, and reporting "0%" for that would read as a real, if
bad, measurement rather than as "this measurement did not happen." That case
is refused loudly instead (see ZeroTestsError below), the same way
scripts/test.sh refuses to report success on a zero count.

A zero-test result has two different causes, and the message names which one
it is instead of using one generic wording for both:
  - the build never compiled (today's actual state on Windows: typdoc-fs,
    the crate implementing typdoc_core::Fs, is Unix-only by design and has
    no Windows branch at all, so `cargo test` fails before any test binary
    exists to run) -- reported as "does not compile -- 0 tests run";
  - the build compiled but the captured run still produced no "test result:"
    lines for some other reason (wrong working directory, cargo not on
    PATH, a test binary that ran but printed nothing recognizable, ...) --
    reported with the older, more general "found zero tests" wording.
Supporting Windows is a two-step ladder: step one is "compiles," step two is
"pass rate." Today's run is stuck at step one, and this script's job is to
say exactly that in the step summary, not to blur it into a generic error
that reads the same whether the build compiled or not.

Run standalone:
    python windows_test_report.py <captured-cargo-test-output-file>

See scripts/test_windows_test_report.py for the self-test that exercises the
parsing/rendering logic below against fixed sample output, independent of any
real `cargo test` run -- the same "prove the mechanism before trusting it"
shape as scripts/test.sh's own --self-test.
"""

from __future__ import annotations

import dataclasses
import os
import re
import sys

# Sums every "N passed; M failed" out of cargo's own "test result:" lines, one
# per test binary (unit tests, integration tests, doctests) -- the same shape
# as scripts/test.sh's awk parsing, ported to Python because Windows runners
# have no awk.
_RESULT_LINE = re.compile(r"^test result:.*?(\d+) passed;\s*(\d+) failed", re.MULTILINE)

# Best-effort markers of "the build never compiled," used only to make a zero-test result's
# error message specific instead of generic. `error[E` matches any rustc diagnostic code, which
# only appears when compilation itself failed (a passing test run never prints one); the two
# phrases are cargo's own terminal wording for a build that didn't produce a test binary.
_COMPILE_FAILURE_MARKERS = ("error: could not compile", "error: aborting due to")


def _looks_like_compile_failure(cargo_test_output: str) -> bool:
    if "error[E" in cargo_test_output:
        return True
    return any(marker in cargo_test_output for marker in _COMPILE_FAILURE_MARKERS)


class ZeroTestsError(RuntimeError):
    """Raised when a captured run has no "test result:" lines at all -- either because the
    build never compiled, or because it compiled but genuinely produced no test result for some
    other reason. The message names which one, when it can tell (see compile_failed below)."""


@dataclasses.dataclass(frozen=True)
class Summary:
    passed: int
    failed: int
    suite_count: int
    compile_failed: bool = False

    @property
    def total(self) -> int:
        return self.passed + self.failed

    @property
    def rate_percent(self) -> float:
        check_nonzero(self)
        return round(self.passed / self.total * 100, 1)


def summarize(cargo_test_output: str) -> Summary:
    passed = failed = suites = 0
    for match in _RESULT_LINE.finditer(cargo_test_output):
        passed += int(match.group(1))
        failed += int(match.group(2))
        suites += 1
    return Summary(
        passed=passed,
        failed=failed,
        suite_count=suites,
        compile_failed=_looks_like_compile_failure(cargo_test_output),
    )


def check_nonzero(summary: Summary) -> None:
    """Refuses to treat a zero-test run as a valid measurement -- mirrors
    scripts/test.sh's own "refuses to report success on a zero count." The message is specific
    about *why* there are zero tests: a build that never compiled is a different, earlier
    failure than one that compiled but happened to run nothing."""
    if summary.total == 0:
        if summary.compile_failed:
            raise ZeroTestsError(
                "does not compile -- 0 tests run. The Windows build failed before any test "
                "binary could run. Supporting Windows is two steps, compiles then pass rate, "
                "and this run is stuck at step one -- there is no pass rate to report yet."
            )
        raise ZeroTestsError(
            f"found zero tests across {summary.suite_count} suite(s) -- the build compiled but "
            "no test binary reported a result; refusing to report a pass rate for a run that "
            "tested nothing (a silent no-op must not read as a valid 0% or 100%)."
        )


def render_step_summary(summary: Summary) -> str:
    rate = summary.rate_percent
    return (
        "## Windows test-suite pass rate\n\n"
        f"**{summary.passed} / {summary.total}** tests passed (**{rate}%**) "
        f"across {summary.suite_count} suite(s).\n\n"
        "This is a baseline measurement, not a gate -- this job never blocks a merge, "
        "and this story does not fix any Windows failure it finds.\n"
    )


def _append_step_summary(text: str) -> None:
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary_path:
        return
    with open(summary_path, "a", encoding="utf-8") as f:
        f.write(text)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: windows_test_report.py <captured-cargo-test-output-file>", file=sys.stderr)
        return 2

    # A missing or unreadable output file means the test step never even produced output
    # (e.g. `cargo` wasn't found before it could run) -- that is exactly the same "this
    # measurement did not happen" case as zero "test result:" lines, not a separate crash.
    try:
        with open(argv[1], "r", encoding="utf-8", errors="replace") as f:
            output = f.read()
    except OSError as exc:
        print(f"windows_test_report: could not read '{argv[1]}': {exc}", file=sys.stderr)
        output = ""

    summary = summarize(output)

    try:
        check_nonzero(summary)
    except ZeroTestsError as exc:
        print(f"windows_test_report: {exc}", file=sys.stderr)
        if summary.compile_failed:
            # A known state, not a failure: step one of supporting Windows is "compiles,"
            # step two is "pass rate," and this run is stuck at step one. The job must stay
            # green for this (a `::warning::` annotation is how it stays visible on the PR
            # without turning the check red) -- only a build that compiled yet still
            # produced no test result is treated as broken, below.
            print("::warning::Windows build does not compile -- 0 tests run")
            _append_step_summary(
                "## Windows test-suite pass rate\n\n"
                "**Does not compile -- 0 tests run.** The Windows build failed before any test "
                "could execute. Supporting Windows is two steps, compiles then pass rate, and "
                "this run is stuck at step one; there is no pass rate to report yet.\n"
            )
            return 0
        _append_step_summary(
            "## Windows test-suite pass rate\n\n"
            "**ERROR: zero tests found.** The build compiled, but this run executed no "
            "tests; treat this as a failed measurement, not a 0% or 100% pass rate.\n"
        )
        return 1

    print(
        f"windows_test_report: {summary.passed} / {summary.total} passed "
        f"({summary.rate_percent}%) across {summary.suite_count} suite(s)."
    )
    _append_step_summary(render_step_summary(summary))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
