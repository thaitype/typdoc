#!/usr/bin/env bash
# End-to-end PR-time test for pages/install (POSIX sh installer), run on ubuntu-latest and
# macos-latest. See .chief/story-5/_contract/contract.md's Testing Decisions: no release of
# typdoc exists until 0.3.1 is published, so this points pages/install at a local HTTP server
# (scripts/installer_fixture_server.py) via TYPDOC_INSTALL_BASE_URL, seeded from the real
# archive+checksum this PR's own dist-build job just produced for this runner's target, plus a
# second fixture "version" of the same bytes under a different tag.
#
# The contract's four scenarios (default install, INSTALL_DIR override, TYPDOC_VERSION pin, an
# intentionally-corrupted checksum that must fail loudly and install nothing), plus three more
# covering path_advice's shell-specific "not on PATH yet" message (zsh, bash -- rc file depends
# on this runner's own OS, fish), with the first two scenarios above extended to also assert on
# the $HOME-relative vs. plain-path display and the neutral unknown-shell fallback. Exercises
# pages/install itself (not scripts/installer_fixture_server.py, which has its own self-test in
# scripts/test_installer_fixture_server.py) end to end: real curl/wget downloads, real checksum
# verification, real tar extraction, real binary execution.
#
# GNU-isms checked for, not just assumed absent (same discipline as scripts/test.sh): this file
# uses `mktemp -d` with an explicit template (BSD/macOS mktemp has no bare-argument default),
# no `mapfile`, no `declare -A`, and no GNU-only `sed`/`grep` flags.
#
# Usage:
#   scripts/test_installer_posix.sh <archive_dir> <target>
#
#   archive_dir   directory holding typdoc-<target>.tar.gz and its .sha256, as downloaded from
#                 this PR's dist-build workflow artifact (dist-<target>).
#   target        the release target this runner's own archive was built for, e.g.
#                 x86_64-unknown-linux-musl or aarch64-apple-darwin.

set -u

if [ "$#" -ne 2 ]; then
    echo "usage: scripts/test_installer_posix.sh <archive_dir> <target>" 1>&2
    exit 2
fi

ARCHIVE_DIR="$1"
TARGET="$2"
ARCHIVE_NAME="typdoc-${TARGET}.tar.gz"
ARCHIVE_PATH="${ARCHIVE_DIR}/${ARCHIVE_NAME}"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_SCRIPT="${REPO_ROOT}/pages/install"

LATEST_TAG="v0.3.1-pr-test"
PIN_TAG="v0.9.9-pr-test-pin"
CORRUPT_TAG="v0.0.0-pr-test-corrupt"

FAILURES=0

note() { printf '\n== %s ==\n' "$*"; }
pass() { printf 'ok: %s\n' "$*"; }
fail() {
    printf 'FAIL: %s\n' "$*" 1>&2
    FAILURES=$((FAILURES + 1))
}

[ -f "$ARCHIVE_PATH" ] || {
    echo "error: missing fixture archive: ${ARCHIVE_PATH}" 1>&2
    exit 2
}
[ -f "$CHECKSUM_PATH" ] || {
    echo "error: missing fixture checksum: ${CHECKSUM_PATH}" 1>&2
    exit 2
}
[ -x "$INSTALL_SCRIPT" ] || {
    echo "error: pages/install is missing or not executable" 1>&2
    exit 2
}

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/typdoc-installer-test.XXXXXXXX")"
cleanup() {
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" >/dev/null 2>&1 || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT INT TERM

FIXTURES_DIR="${WORK_DIR}/fixtures"
mkdir -p "${FIXTURES_DIR}/${LATEST_TAG}" "${FIXTURES_DIR}/${PIN_TAG}" "${FIXTURES_DIR}/${CORRUPT_TAG}"

cp "$ARCHIVE_PATH" "${FIXTURES_DIR}/${LATEST_TAG}/${ARCHIVE_NAME}"
cp "$CHECKSUM_PATH" "${FIXTURES_DIR}/${LATEST_TAG}/${ARCHIVE_NAME}.sha256"
cp "$ARCHIVE_PATH" "${FIXTURES_DIR}/${PIN_TAG}/${ARCHIVE_NAME}"
cp "$CHECKSUM_PATH" "${FIXTURES_DIR}/${PIN_TAG}/${ARCHIVE_NAME}.sha256"
cp "$ARCHIVE_PATH" "${FIXTURES_DIR}/${CORRUPT_TAG}/${ARCHIVE_NAME}"
echo "0000000000000000000000000000000000000000000000000000000000000000 *${ARCHIVE_NAME}" \
    > "${FIXTURES_DIR}/${CORRUPT_TAG}/${ARCHIVE_NAME}.sha256"

LOG_PATH="${WORK_DIR}/requests.log"
SERVER_OUT="${WORK_DIR}/server.out"
# -u (PYTHONUNBUFFERED): on macOS's python.org/framework-build python3 (what actions/setup-python
# installs there), a plain Python-level print()+flush() to a redirected, non-tty stdout has been
# observed NOT to reach the file the shell redirected it to, even long after the server itself
# finished starting and settled into its normal serve_forever() sleep (confirmed with `ps`: the
# process is alive and idle, not stuck setting up) -- diagnosed by instrumenting a prior version
# of this script, not guessed. -u forces CPython's own stdio layer unbuffered at the C level,
# which is the standard fix for exactly this class of symptom and has resolved it here.
python3 -u "${REPO_ROOT}/scripts/installer_fixture_server.py" \
    "$FIXTURES_DIR" "$LATEST_TAG" --log "$LOG_PATH" --port 0 >"$SERVER_OUT" 2>"${WORK_DIR}/server.err" &
SERVER_PID=$!

PORT=""
# 200 * 0.1s = 20s: not needed for the buffering issue above (that's now fixed at the source),
# kept as a generous safety margin for ordinary runner-load variance.
for _ in $(seq 1 200); do
    if [ -s "$SERVER_OUT" ]; then
        PORT="$(head -n1 "$SERVER_OUT" | tr -d '[:space:]')"
        [ -n "$PORT" ] && break
    fi
    kill -0 "$SERVER_PID" 2>/dev/null || {
        echo "error: fixture server exited before it started listening; see ${WORK_DIR}/server.err" 1>&2
        cat "${WORK_DIR}/server.err" 1>&2
        exit 2
    }
    sleep 0.1
done
[ -n "$PORT" ] || {
    echo "error: fixture server never reported a port within 20s; see ${WORK_DIR}/server.err" 1>&2
    echo "diag: server.err ---" 1>&2
    cat "${WORK_DIR}/server.err" 1>&2
    echo "diag: server.out ---" 1>&2
    cat "$SERVER_OUT" 1>&2
    echo "diag: ls -la of both files ---" 1>&2
    ls -la "$SERVER_OUT" "${WORK_DIR}/server.err" 1>&2 || true
    echo "diag: ps for SERVER_PID=${SERVER_PID} ---" 1>&2
    ps -p "$SERVER_PID" -o pid,ppid,etime,stat,command 1>&2 || echo "diag: ps found no such pid" 1>&2
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    exit 2
}
BASE_URL="http://127.0.0.1:${PORT}"

log_lines_after() {
    # Prints the request log lines appended since line count $1.
    _since="$1"
    tail -n "+$((_since + 1))" "$LOG_PATH" 2>/dev/null || true
}

log_line_count() {
    wc -l <"$LOG_PATH" 2>/dev/null | tr -d '[:space:]'
}

# Passed explicitly into every `env -i` call below: `env -i` clears TMPDIR along with
# everything else, and pages/install's own `mktemp -d` would otherwise silently fall back to
# the system default /tmp instead of this run's own work directory.
INSTALLER_TMPDIR="${WORK_DIR}/installer-tmp"
mkdir -p "$INSTALLER_TMPDIR"

# --- Scenario 1: default install (no TYPDOC_VERSION, no INSTALL_DIR) ------------------------
note "default install"
HOME_1="${WORK_DIR}/home1"
mkdir -p "$HOME_1"
BEFORE="$(log_line_count)"
OUT_1="$(env -i HOME="$HOME_1" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    "$INSTALL_SCRIPT" 2>&1)"
CODE_1=$?
if [ "$CODE_1" -eq 0 ] && [ -x "${HOME_1}/.local/bin/typdoc" ]; then
    pass "default install exits 0 and installs to \$HOME/.local/bin"
else
    fail "default install: exit=${CODE_1} output follows
${OUT_1}"
fi
if [ -x "${HOME_1}/.local/bin/typdoc" ] && "${HOME_1}/.local/bin/typdoc" --version >/dev/null 2>&1; then
    pass "installed binary runs (--version)"
else
    fail "installed binary did not run"
fi
if log_lines_after "$BEFORE" | grep -q "^GET /releases/latest/download/"; then
    pass "default install used the releases/latest redirect, not a pinned tag"
else
    fail "default install did not hit /releases/latest/download/... — request log:
$(log_lines_after "$BEFORE")"
fi
# env -i above never sets SHELL, so this also exercises path_advice's unknown/empty-shell
# branch, and HOME_1/.local/bin is under $HOME_1, so it also exercises the "$HOME/..." display.
if printf '%s' "$OUT_1" | grep -qF '⚠ $HOME/.local/bin is not on your PATH yet.'; then
    pass "PATH advice shows \$HOME-relative form for a dir under \$HOME"
else
    fail "PATH advice did not show the \$HOME-relative form — output follows
${OUT_1}"
fi
if printf '%s' "$OUT_1" | grep -qF "to PATH in your shell's startup file"; then
    pass "PATH advice falls back to the neutral guide when \$SHELL is unset"
else
    fail "PATH advice did not use the neutral guide for an unset \$SHELL — output follows
${OUT_1}"
fi

# --- Scenario 2: INSTALL_DIR override --------------------------------------------------------
note "INSTALL_DIR override"
CUSTOM_DIR="${WORK_DIR}/custom-bin"
OUT_2="$(env -i HOME="$HOME_1" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    INSTALL_DIR="$CUSTOM_DIR" "$INSTALL_SCRIPT" 2>&1)"
CODE_2=$?
if [ "$CODE_2" -eq 0 ] && [ -x "${CUSTOM_DIR}/typdoc" ]; then
    pass "INSTALL_DIR override exits 0 and installs into the given directory"
else
    fail "INSTALL_DIR override: exit=${CODE_2} output follows
${OUT_2}"
fi
if [ ! -e "${HOME_1}/.local/bin/typdoc" ] || [ -x "${HOME_1}/.local/bin/typdoc" ]; then
    : # scenario 1 already installed there; not a re-check for this scenario
fi
# CUSTOM_DIR is a sibling of $HOME_1, not under it — PATH advice must show the plain absolute
# path, not a "$HOME/..." shorthand that would be wrong here.
if printf '%s' "$OUT_2" | grep -qF "⚠ ${CUSTOM_DIR} is not on your PATH yet."; then
    pass "PATH advice shows the plain absolute path for a dir outside \$HOME"
else
    fail "PATH advice did not show the plain path for a non-\$HOME INSTALL_DIR — output follows
${OUT_2}"
fi
if printf '%s' "$OUT_2" | grep -qF '$HOME'; then
    fail "PATH advice wrongly used \$HOME shorthand for a dir outside \$HOME — output follows
${OUT_2}"
fi

# --- Scenario 3: TYPDOC_VERSION pin -----------------------------------------------------------
note "TYPDOC_VERSION pin"
PIN_DIR="${WORK_DIR}/pin-bin"
BEFORE="$(log_line_count)"
OUT_3="$(env -i HOME="$HOME_1" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    TYPDOC_VERSION="$PIN_TAG" INSTALL_DIR="$PIN_DIR" "$INSTALL_SCRIPT" 2>&1)"
CODE_3=$?
if [ "$CODE_3" -eq 0 ] && [ -x "${PIN_DIR}/typdoc" ]; then
    pass "TYPDOC_VERSION pin exits 0 and installs"
else
    fail "TYPDOC_VERSION pin: exit=${CODE_3} output follows
${OUT_3}"
fi
if log_lines_after "$BEFORE" | grep -q "^GET /releases/download/${PIN_TAG}/"; then
    pass "TYPDOC_VERSION pin used the direct tagged download, not the latest redirect"
else
    fail "TYPDOC_VERSION pin did not hit /releases/download/${PIN_TAG}/... — request log:
$(log_lines_after "$BEFORE")"
fi
if log_lines_after "$BEFORE" | grep -q "^GET /releases/latest/download/"; then
    fail "TYPDOC_VERSION pin unexpectedly also hit the latest redirect"
fi

# --- Scenario 4: corrupted checksum must fail loudly and install nothing --------------------
note "corrupted checksum"
CORRUPT_DIR="${WORK_DIR}/corrupt-bin"
OUT_4="$(env -i HOME="$HOME_1" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    TYPDOC_VERSION="$CORRUPT_TAG" INSTALL_DIR="$CORRUPT_DIR" "$INSTALL_SCRIPT" 2>&1)"
CODE_4=$?
if [ "$CODE_4" -ne 0 ]; then
    pass "corrupted checksum exits non-zero"
else
    fail "corrupted checksum unexpectedly exited 0"
fi
if printf '%s' "$OUT_4" | grep -qi "checksum mismatch"; then
    pass "corrupted checksum prints a checksum-mismatch error"
else
    fail "corrupted checksum did not mention a checksum mismatch — output follows
${OUT_4}"
fi
if [ ! -e "${CORRUPT_DIR}/typdoc" ]; then
    pass "corrupted checksum installed nothing"
else
    fail "corrupted checksum still installed a binary at ${CORRUPT_DIR}/typdoc"
fi

# --- Scenarios 5-7: PATH advice is shell-specific (zsh, bash, fish) -------------------------
# Same shape as scenario 1's re-run, just with $SHELL set — a fresh $HOME per scenario so each
# one hits the "not on PATH yet" branch independently of the others.

# --- Scenario 5: zsh ---------------------------------------------------------------------------
note "PATH advice: zsh"
HOME_ZSH="${WORK_DIR}/home-zsh"
mkdir -p "$HOME_ZSH"
OUT_5="$(env -i HOME="$HOME_ZSH" SHELL="/usr/bin/zsh" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    "$INSTALL_SCRIPT" 2>&1)"
if printf '%s' "$OUT_5" | grep -qF "echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc" \
    && printf '%s' "$OUT_5" | grep -qF "source ~/.zshrc"; then
    pass "zsh gets the ~/.zshrc echo + source lines"
else
    fail "zsh PATH advice missing the expected ~/.zshrc lines — output follows
${OUT_5}"
fi

# --- Scenario 6: bash (rc file depends on this runner's own OS, same as pages/install's logic) -
note "PATH advice: bash"
HOME_BASH="${WORK_DIR}/home-bash"
mkdir -p "$HOME_BASH"
case "$(uname -s)" in
    Darwin) EXPECT_BASH_RC="~/.bash_profile" ;;
    *) EXPECT_BASH_RC="~/.bashrc" ;;
esac
OUT_6="$(env -i HOME="$HOME_BASH" SHELL="/bin/bash" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    "$INSTALL_SCRIPT" 2>&1)"
if printf '%s' "$OUT_6" | grep -qF "echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ${EXPECT_BASH_RC}" \
    && printf '%s' "$OUT_6" | grep -qF "source ${EXPECT_BASH_RC}"; then
    pass "bash gets the ${EXPECT_BASH_RC} echo + source lines (this runner's own OS)"
else
    fail "bash PATH advice did not mention ${EXPECT_BASH_RC} — output follows
${OUT_6}"
fi

# --- Scenario 7: fish -----------------------------------------------------------------------
note "PATH advice: fish"
HOME_FISH="${WORK_DIR}/home-fish"
mkdir -p "$HOME_FISH"
OUT_7="$(env -i HOME="$HOME_FISH" SHELL="/usr/local/bin/fish" PATH="/usr/bin:/bin" TMPDIR="$INSTALLER_TMPDIR" \
    TYPDOC_INSTALL_BASE_URL="$BASE_URL" \
    "$INSTALL_SCRIPT" 2>&1)"
if printf '%s' "$OUT_7" | grep -qF 'fish_add_path $HOME/.local/bin'; then
    pass "fish gets the fish_add_path line, no export, no source"
else
    fail "fish PATH advice did not show fish_add_path — output follows
${OUT_7}"
fi
if printf '%s' "$OUT_7" | grep -q "^\s*export PATH="; then
    fail "fish PATH advice wrongly showed an export line (not fish syntax) — output follows
${OUT_7}"
fi

if [ "$FAILURES" -eq 0 ]; then
    printf '\nall installer scenarios passed\n'
    exit 0
else
    printf '\n%d installer scenario(s) failed\n' "$FAILURES" 1>&2
    exit 1
fi
