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
# directly and can be told to refuse swap.
#
# macOS has no cgroups, and `ulimit -v` (RLIMIT_AS) — the first mechanism
# tried here — turned out not to be a real option either: confirmed on a
# real `macos-latest` GitHub-hosted runner, 2026-09-24, not assumed from
# documentation, that plain `ulimit -v` is refused outright ("ulimit -v is
# not settable in this shell") on that platform's bash. There is no
# per-process kernel memory cap a plain bash script can reach on Darwin.
#
# The mechanism here instead is a background watchdog: run the command with
# job control on (so it gets its own process group), poll the *system's*
# free memory every 0.2 s via `vm_stat`, and SIGKILL the whole process group
# the moment free memory drops by more than the ceiling *from the baseline
# measured right before the command started* — not from total physical
# memory, which a first real run (2026-09-24) showed was the wrong
# reference point: a fixed "total minus ceiling" threshold assumed a
# baseline free-at-idle figure that was never measured, and on the real
# runner it was already below that threshold before the command even ran,
# so the watchdog fired on its first sample and killed nothing but time.
# Measuring the baseline per run instead answers "did this command's own
# usage grow by more than the ceiling," regardless of whatever the runner's
# baseline happens to be. This works because a GitHub-hosted runner is
# otherwise idle — nothing else of consequence competes for memory during
# the run — so "free memory fell by more than the ceiling since baseline"
# and "our command used more than the ceiling" are the same fact there,
# even though the mechanism doesn't touch the command's own limits at all.
# It is soft (a fast-growing process can overshoot between two 0.2 s
# samples) rather than kernel-enforced, but it is real enforcement, not a
# no-op — which `ulimit -v` turned out to be here.
#
# `--self-test` is how it proves itself: a runaway that would reach a stop
# point if nothing intervened. It also proves job control's process-group
# kill actually reaches every descendant, not just the top process — a
# `cargo test` runaway is rarely the top-level `cargo` process itself.
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

# Measured, not guessed, as of the second real run this size was tried
# (2026-09-24): available memory (free + inactive + speculative + purgeable,
# see free_kb_macos) on a fresh `macos-latest` runner, right before the real
# `cargo test` invocation, was ~3.3 GB — nowhere near the 7 GB total the
# machine reports, because most of a Mac's spare RAM sits in categories
# `vm_stat`'s raw "free" doesn't count, and a meaningful chunk of even the
# *available* figure is already the toolchain install and checkout that ran
# moments before. 2048 MB leaves about 1.3 GB of that 3.3 GB as margin,
# comparable to the Linux ceiling sitting well above its own 1451 MB measured
# peak. If a real run's ordinary usage still doesn't fit, `--self-test`'s own
# PROBE-OK check and `capped_macos`'s own preflight (both refuse loudly
# rather than silently) are what surface that, not a mysterious kill.
CEILING_MB_MACOS=2048

os_kind() {
  case "$(uname -s)" in
    Linux) echo linux ;;
    Darwin) echo macos ;;
    *) echo other ;;
  esac
}

OS_KIND=$(os_kind)

# macOS's *available* memory, in KB — not raw "Pages free" alone, which a
# real run (2026-09-24) showed reads as only ~940 MB on a runner with 7 GB
# total: macOS deliberately keeps very little literally free, using spare
# RAM as file-backed disk cache rather than leaving it idle, unlike Linux
# where free/available track closely together. "Pages free" undercounts what
# a process can actually get by a wide margin there. The standard
# approximation for what's really available without paging or writeback is
# free + inactive + speculative + purgeable (each reclaimable instantly);
# active and wired-down pages are the ones genuinely in use and excluded.
# Multiplied by the page size `vm_stat` names in its own header line — Apple
# Silicon and Intel Macs use different page sizes, so this is read, never
# assumed.
free_kb_macos() {
  local page_size pages
  page_size=$(vm_stat | sed -n '1s/.*page size of \([0-9]*\) bytes.*/\1/p')
  pages=$(vm_stat | awk -F: '
    /^Pages free/ || /^Pages inactive/ || /^Pages speculative/ || /^Pages purgeable/ {
      gsub(/[. ]/, "", $2)
      total += $2
    }
    END { print total + 0 }
  ')
  echo $(( pages * page_size / 1024 ))
}

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
      # `ulimit -v` is not an option here (see the header note) — this checks
      # the watchdog's own two dependencies instead: `sysctl` for total
      # memory, `vm_stat` for the free-memory samples it polls.
      if ! command -v sysctl >/dev/null 2>&1 || ! command -v vm_stat >/dev/null 2>&1; then
        echo "test.sh: sysctl or vm_stat is not on this machine, so free memory cannot be watched." >&2
        echo "test.sh: refusing to run the tests uncapped. Run them under another ceiling yourself." >&2
        exit 2
      fi
      local total_kb
      total_kb=$(( $(sysctl -n hw.memsize) / 1024 ))
      if [ "$total_kb" -le "$((CEILING_MB_MACOS * 1024))" ]; then
        echo "test.sh: this machine has less total memory (${total_kb} KB) than the ceiling (${CEILING_MB_MACOS} MB) leaves no room for." >&2
        echo "test.sh: refusing to run the tests uncapped. Lower the ceiling for a machine this size." >&2
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
      capped_macos "$mb" "$@"
      ;;
  esac
}

# The watchdog itself (see the header note for why this exists instead of a
# kernel-enforced limit): runs COMMAND in the background under job control,
# so it gets its own process group, and polls available memory every 0.2 s.
# Available memory dropping by more than MB from the baseline measured right
# before COMMAND started is treated as COMMAND having used more than MB, and
# the whole process group is killed at once — not just the top process,
# since a `cargo test` runaway is almost never `cargo` itself, and an
# orphaned child left running would keep growing.
capped_macos() { # capped_macos MB -- COMMAND...
  local mb=$1
  shift
  local baseline_kb min_free_kb pid status
  baseline_kb=$(free_kb_macos)
  # Refuses the same way an absent systemd-run does on Linux, rather than
  # silently running with no effective cap: if there isn't even MB's worth
  # of available memory to lose in the first place, the threshold below
  # would go negative and "available memory below a negative number" can
  # never be true, so the watchdog would never fire.
  if [ "$baseline_kb" -le "$((mb * 1024))" ]; then
    echo "test.sh: only ${baseline_kb} KB available right now, not enough to lose ${mb} MB and still have a real ceiling." >&2
    echo "test.sh: refusing to run the tests uncapped. Lower the ceiling, or find out what's using the rest first." >&2
    exit 2
  fi
  min_free_kb=$((baseline_kb - mb * 1024))
  echo "test.sh: macOS watchdog: ${baseline_kb} KB available at start, ceiling ${mb} MB, killing if available drops below ${min_free_kb} KB." >&2
  set -m
  "$@" &
  pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$(free_kb_macos)" -lt "$min_free_kb" ]; then
      kill -KILL -- "-$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
      set +m
      return 137
    fi
    sleep 0.2
  done
  wait "$pid"
  status=$?
  set +m
  return "$status"
}

if [ "${1:-}" = "--self-test" ]; then
  require_cap
  self_test_cap=64
  # The runaway below reaches about 128 MB (2^27 bytes) in its final string,
  # having passed through every smaller power of two on the way — a
  # free-memory-drop cap has to sit under that, not near CEILING_MB_MACOS
  # itself, or the runaway finishes before it ever shows up as "missing"
  # memory. 96 MB is picked to land inside the doubling's last two or three
  # steps, the same reasoning the 64 MB Linux cap already uses, just with a
  # little more margin for this mechanism's own overhead (job control,
  # `vm_stat` calls) and macOS's smaller minimum-viable command footprint.
  # See the PROBE-OK check right below: if this guess is wrong and even an
  # ordinary command can't run under it, that check says so loudly instead
  # of the self-test reporting "ok" for the wrong reason.
  [ "$OS_KIND" = "macos" ] && self_test_cap=96

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

# `--no-fail-fast`: cargo's own default stops running further test binaries
# once one reports a failure, which a real macOS run (2026-09-24) showed
# hiding real information -- the run stopped at the first failing suite
# having run only about half the workspace's test binaries, so any other
# platform-specific failure past that point was invisible, not passing. One
# red suite must never look like "everything after it is fine."
if [ $# -gt 0 ]; then
  capped "$ceiling" cargo test --no-fail-fast "$@" 2>&1 | tee "$tmp_out"
else
  capped "$ceiling" cargo test --workspace --no-fail-fast --features typdoc/test-stand-in 2>&1 | tee "$tmp_out"
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
