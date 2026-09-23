#!/usr/bin/env bash
# Runs the test suite under a memory ceiling.
#
# WHY. A loop that could not end once took every byte of this machine, and the
# kernel killed the session that started it along with the run. A test that
# grows without stopping is a fault to report, never something to make room
# for, so the ceiling is deliberately far below what the machine has.
#
# THE LINUX CEILING IS 6144 MB, AND THE NUMBERS BEHIND IT ARE MEASURED, NOT
# GUESSED. On this machine, which has four cores: the whole test suite peaks
# at 229 MB, and a cold build of the workspace followed by a full run peaks at
# 1451 MB, both measured at the cgroup rather than per process. The ceiling is
# about four times the worse of the two. The run that made the kernel step in
# reached 16 GB. These numbers belong to this machine and this core count: a
# machine with more cores builds more crates at once and needs them measured
# again.
#
# `MemorySwapMax=0` is not decoration. There is swap here, and without that
# line a runaway is paged out instead of stopped, which drags the whole machine
# down slowly rather than failing one run quickly.
#
# TWO OSES, TWO MECHANISMS. Linux has cgroups, reached here through
# `systemd-run --user --scope`, which caps resident memory (`MemoryMax`)
# directly and can be told to refuse swap. macOS has no cgroups and no
# user-facing resident-memory cap; the only per-process lever a plain bash
# script can reach there is `ulimit -v` (RLIMIT_AS), which caps virtual
# address space, not resident memory. Those are not the same quantity: on
# Darwin, virtual size sits well above resident size for almost any process,
# because the dyld shared cache and unused-but-reserved allocator arenas are
# mapped into address space without ever being paid for in RSS. Reusing the
# Linux number verbatim as a macOS `ulimit -v` value would risk killing an
# ordinary, non-runaway `cargo test` before it does any work, which is worse
# than not capping at all: a ceiling that can't tell "just started" from
# "runaway" isn't a ceiling. CEILING_MB_MACOS below is picked with that
# overhead in mind, but it is a reasoned estimate made on a Linux machine —
# not a measurement — and unverified. See CEILING_MB_MACOS's own comment.
#
# Whichever mechanism runs, `--self-test` is how it proves itself: a runaway
# that would reach a stop point if nothing intervened. If a macOS run's own
# `--self-test` step doesn't come back "stopped," that's real evidence
# `ulimit -v` isn't enforcing here, not a hypothetical — see the "Extra care
# for the macOS leg" note this script's own ticket carries.
#
# GNU-ISMS CHECKED FOR, NOT JUST ASSUMED ABSENT. Read start to finish looking
# for `sed -i` with no suffix, `grep -P`, `readlink -f`, `mapfile`, and bash
# 4+ associative arrays (`declare -A`) — none of those appear anywhere in
# this file. The only additions this macOS follow-up made that touch
# portability at all are `mktemp` (called with an explicit template below,
# since GNU mktemp defaults one in with no arguments but BSD/macOS mktemp
# does not) and `awk` (written with POSIX field-splitting only, no gawk
# extensions). Everything else here — `case`, `local`, arithmetic
# `$(( ))`, `<<<` here-strings, `${PIPESTATUS[0]}` — predates bash 3.2 and
# needs nothing newer, so the macOS-shipped `/bin/bash` should read this
# file the same as any newer bash would, even if a CI job's PATH happens to
# find a Homebrew-installed bash first instead.
#
# Usage:
#   scripts/test.sh                 cargo test --workspace, under the ceiling
#   scripts/test.sh ARGS...         the same, with these arguments instead
#   scripts/test.sh --self-test     prove the ceiling stops a runaway
#
# The workspace run turns on `typdoc/test-stand-in`, without which the shell
# examples harness has no stand-in binary to put on `PATH` and its tests fail.
# The feature is off by default so that `cargo install` does not offer that
# binary to a caller. A passthrough run that reaches those tests has to ask for
# the feature itself; it is not added here, because a run that selects another
# package alone is refused outright for naming a feature that package has not
# got.
set -u

CEILING_MB=6144

# Unverified on real hardware (this script is being edited on a Linux
# machine): a starting point for macOS's `ulimit -v`, not a measurement, and
# a genuinely tight one — the reasoning below has a real tension in it that
# a real run is what resolves, not this comment.
#
# `ulimit -v` measures virtual address space, and a Rust process's baseline
# VSZ (dyld shared cache, thread-stack reservations, allocator arenas
# reserved but untouched) commonly sits in the low gigabytes before any
# workload runs at all — well above the equivalent RSS number, which is why
# this isn't just the Linux 6144 reused. But GitHub's own hosted `macos-latest`
# runner (the Apple Silicon one, which is what that label resolves to as of
# this writing) has only 3 cores and 7 GB of RAM total — the Intel variant
# has 14 GB, but is not what `macos-latest` currently means. So the number
# below has to sit under a 7168 MB ceiling to be "deliberately far below what
# the machine has" at all, while the VSZ-vs-RSS gap above argues for
# something well above the Linux figure. Those two pulls don't fully
# reconcile: 5120 MB (5 GB) is picked to leave real headroom below the box's
# own 7 GB rather than to comfortably clear the baseline-VSZ estimate above,
# because running out of real, physical memory on a 7 GB machine is the
# worse failure to risk. If that's still too tight for an ordinary run to
# even start, `require_cap`'s preflight and `--self-test`'s own probe (see
# below) are what surface that as a loud, named failure instead of a run
# that mysteriously never gets past `rustc --version`.
CEILING_MB_MACOS=5120

os_kind() {
  case "$(uname -s)" in
    Linux) echo linux ;;
    Darwin) echo macos ;;
    *) echo other ;;
  esac
}

OS_KIND=$(os_kind)

# Fails loudly rather than running uncapped: a safety net that disappears
# quietly is worse than none, because everyone still believes it is there.
require_cap() {
  case "$OS_KIND" in
    linux)
      if ! command -v systemd-run >/dev/null 2>&1; then
        echo "test.sh: systemd-run is not on this machine, so the memory ceiling cannot be applied." >&2
        echo "test.sh: refusing to run the tests uncapped. Run them under another ceiling yourself." >&2
        exit 2
      fi
      if ! systemd-run --user --scope -q -p MemoryMax=64M -p MemorySwapMax=0 -- true >/dev/null 2>&1; then
        echo "test.sh: systemd-run is here but a user scope could not be started." >&2
        echo "test.sh: refusing to run the tests uncapped. A user session bus is usually what is missing." >&2
        exit 2
      fi
      ;;
    macos)
      # This only proves the rlimit call itself is accepted by the shell, not
      # that the kernel goes on to enforce it against a real runaway — that
      # second part is exactly what --self-test is for, and what a real macOS
      # CI run confirms or disproves. A shell that can't even set the limit
      # is refused the same way an absent systemd-run is refused on Linux.
      if ! (ulimit -v $((CEILING_MB_MACOS * 1024))) >/dev/null 2>&1; then
        echo "test.sh: ulimit -v is not settable in this shell, so the memory ceiling cannot be applied." >&2
        echo "test.sh: refusing to run the tests uncapped. Run them under another ceiling yourself." >&2
        exit 2
      fi
      ;;
    *)
      echo "test.sh: no known memory-capping mechanism for this OS ($(uname -s))." >&2
      echo "test.sh: refusing to run the tests uncapped. Run them under another ceiling yourself." >&2
      exit 2
      ;;
  esac
}

capped() { # capped MB -- COMMAND...
  local mb=$1
  shift
  case "$OS_KIND" in
    linux)
      systemd-run --user --scope -q -p "MemoryMax=${mb}M" -p MemorySwapMax=0 -- "$@"
      ;;
    macos)
      # A subshell, not the running script: `ulimit -v` changes the calling
      # shell's own limit and everything it execs from then on, so it is set
      # here, scoped to a subshell, and handed off to the real command with
      # exec rather than run alongside it — the subshell's exit status then
      # *is* the command's exit status, same as systemd-run --scope reports
      # the scoped command's exit status on Linux.
      (ulimit -v $((mb * 1024)) && exec "$@")
      ;;
  esac
}

if [ "${1:-}" = "--self-test" ]; then
  require_cap
  self_test_cap=64
  # 2048 MB is as unmeasured as CEILING_MB_MACOS itself -- picked to sit
  # under it with room to spare, not from any real baseline number. See the
  # PROBE-OK check right below: if this guess is wrong and even an ordinary
  # command can't run under it, that check is what says so, loudly, instead
  # of the self-test reporting "ok" for the wrong reason.
  [ "$OS_KIND" = "macos" ] && self_test_cap=2048

  # Prove the cap doesn't refuse an ordinary command before trusting it to
  # refuse a runaway. Without this, a ceiling set so tight that nothing can
  # even start would look identical, from the checks below, to a ceiling
  # correctly killing real growth -- both end with no output and a nonzero
  # exit. Only a probe that separately confirms an unremarkable command
  # succeeds under the same cap tells those two apart.
  probe=$(capped "$self_test_cap" bash -c 'echo PROBE-OK' 2>/dev/null)
  if [ "$probe" != "PROBE-OK" ]; then
    echo "self-test FAILED: an ordinary command could not even run under a ${self_test_cap} MB cap." >&2
    echo "self-test FAILED: this proves nothing about catching a runaway -- the cap is too tight to use at this size, on this machine." >&2
    exit 1
  fi

  # A runaway with a stop in it. Under a working ceiling it is killed long
  # before the loop ends; if the ceiling does nothing, the loop still ends of
  # its own accord, having taken a few hundred megabytes rather than the
  # machine. Reaching the end is the failure this proves.
  runaway='s=x; for _ in $(seq 1 27); do s="$s$s"; done; echo REACHED-THE-END'
  out=$(capped "$self_test_cap" timeout 60 bash -c "$runaway" 2>/dev/null)
  status=$?
  if [ "$out" = "REACHED-THE-END" ]; then
    echo "self-test FAILED: a program that grows past the ceiling ran to completion, so the ceiling stops nothing"
    exit 1
  fi
  if [ $status -eq 0 ]; then
    echo "self-test FAILED: the runaway ended with success, which it should never do"
    exit 1
  fi
  echo "self-test ok: an ordinary command still runs, but a program that grows past the ceiling is stopped (exit $status, no output)"
  exit 0
fi

require_cap
cd "$(git rev-parse --show-toplevel)" || exit 2

case "$OS_KIND" in
  macos) ceiling=$CEILING_MB_MACOS ;;
  *) ceiling=$CEILING_MB ;;
esac

# Output is streamed to the terminal as it happens (as before) and also
# captured, so the run's own test count can be reported afterward -- the
# check that this was a real run of the suite, not a green step that quietly
# did nothing. mktemp is called with an explicit template because GNU
# mktemp defaults one in when none is given but BSD/macOS mktemp does not.
tmp_out=$(mktemp "${TMPDIR:-/tmp}/typdoc-test.XXXXXXXXXX")
trap 'rm -f "$tmp_out"' EXIT

if [ $# -gt 0 ]; then
  capped "$ceiling" cargo test "$@" 2>&1 | tee "$tmp_out"
else
  capped "$ceiling" cargo test --workspace --features typdoc/test-stand-in 2>&1 | tee "$tmp_out"
fi
run_status=${PIPESTATUS[0]}

# Sum every "N passed" out of cargo's own "test result: ..." lines, one per
# test binary (unit tests, integration tests, doctests). Plain awk, no GNU
# extensions, so this reads the same on BSD/macOS awk as on GNU awk.
read -r total_passed suite_count <<< "$(awk '
  /^test result:/ {
    for (i = 1; i <= NF; i++) {
      if ($i ~ /^passed;?$/) {
        n = $(i - 1) + 0
        total += n
        suites += 1
      }
    }
  }
  END { printf "%d %d\n", total + 0, suites + 0 }
' "$tmp_out")"

echo "test.sh: ${total_passed} test(s) passed across ${suite_count} suite(s) on ${OS_KIND} (ceiling ${ceiling} MB)."

if [ "$run_status" -eq 0 ] && [ "${total_passed:-0}" -eq 0 ]; then
  echo "test.sh: cargo test exited 0 but reported zero passed tests -- treating this as a failure." >&2
  echo "test.sh: a run that passes nothing is not evidence the suite ran; a green result requires real tests to have run." >&2
  exit 3
fi

exit "$run_status"
