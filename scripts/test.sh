#!/usr/bin/env bash
# Runs the test suite under a memory ceiling.
#
# WHY. A loop that could not end once took every byte of this machine, and the
# kernel killed the session that started it along with the run. A test that
# grows without stopping is a fault to report, never something to make room
# for, so the ceiling is deliberately far below what the machine has.
#
# THE CEILING IS 6144 MB, AND THE NUMBERS BEHIND IT ARE MEASURED, NOT GUESSED.
# On this machine, which has four cores: the whole test suite peaks at 229 MB,
# and a cold build of the workspace followed by a full run peaks at 1451 MB,
# both measured at the cgroup rather than per process. The ceiling is about
# four times the worse of the two. The run that made the kernel step in reached
# 16 GB. These numbers belong to this machine and this core count: a machine
# with more cores builds more crates at once and needs them measured again.
#
# `MemorySwapMax=0` is not decoration. There is swap here, and without that
# line a runaway is paged out instead of stopped, which drags the whole machine
# down slowly rather than failing one run quickly.
#
# Usage:
#   scripts/test.sh                 cargo test --workspace, under the ceiling
#   scripts/test.sh ARGS...         the same, with these arguments instead
#
# The workspace run turns on `typdoc/test-stand-in`, without which the shell
# examples harness has no stand-in binary to put on `PATH` and its tests fail.
# The feature is off by default so that `cargo install` does not offer that
# binary to a caller. A passthrough run that reaches those tests has to ask for
# the feature itself; it is not added here, because a run that selects another
# package alone is refused outright for naming a feature that package has not
# got.
#   scripts/test.sh --self-test     prove the ceiling stops a runaway
set -u

CEILING_MB=6144

# Fails loudly rather than running uncapped: a safety net that disappears
# quietly is worse than none, because everyone still believes it is there.
require_scope() {
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
}

capped() { # capped MB -- COMMAND...
  local mb=$1
  shift
  systemd-run --user --scope -q -p "MemoryMax=${mb}M" -p MemorySwapMax=0 -- "$@"
}

if [ "${1:-}" = "--self-test" ]; then
  require_scope
  # A runaway with a stop in it. Under a working ceiling it is killed long
  # before the loop ends; if the ceiling does nothing, the loop still ends of
  # its own accord, having taken a few hundred megabytes rather than the
  # machine. Reaching the end is the failure this proves.
  runaway='s=x; for _ in $(seq 1 27); do s="$s$s"; done; echo REACHED-THE-END'
  out=$(capped 64 timeout 60 bash -c "$runaway" 2>/dev/null)
  status=$?
  if [ "$out" = "REACHED-THE-END" ]; then
    echo "self-test FAILED: a program that grows past the ceiling ran to completion, so the ceiling stops nothing"
    exit 1
  fi
  if [ $status -eq 0 ]; then
    echo "self-test FAILED: the runaway ended with success, which it should never do"
    exit 1
  fi
  echo "self-test ok: a program that grows past the ceiling is stopped (exit $status, no output)"
  exit 0
fi

require_scope
cd "$(git rev-parse --show-toplevel)" || exit 2
if [ $# -gt 0 ]; then
  capped "$CEILING_MB" cargo test "$@"
else
  capped "$CEILING_MB" cargo test --workspace --features typdoc/test-stand-in
fi
