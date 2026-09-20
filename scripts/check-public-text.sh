#!/usr/bin/env bash
# Public-text gate. This repository is public, so every line in it has to read
# as the repository owner's own work.
#
# THIS IS A FLOOR, NOT A CEILING. It only catches phrasing already known to
# fail. A pass proves nothing beyond the patterns below; the check that decides
# is reading each paragraph and asking whether it reads as the owner's own work.
# Do not answer a hit by rewording around the pattern, and do not treat a clean
# run as a clean repository.
#
# Usage:
#   scripts/check-public-text.sh              scan tracked and new files
#   scripts/check-public-text.sh PATH...      scan only these files
#   scripts/check-public-text.sh --self-test  prove the gate can fail
#
# Names cannot be listed here without putting them in the repository. They come
# from a local file, one extended regex per line, named by PUBLIC_TEXT_TERMS_FILE.
# Without it only the generic patterns run, and the gate says so.
set -u

GENERIC=(
  '\bthe human\b'
  'human (decision|approv|overrid)'
  'veto welcome'
  '\bconfirmed by\b'
  '\b(approved|reviewed|drafted|supplied|signed off) by\b'
  '\bpasted\b|\bpasting\b'
  'agent (default|note|observation)s?\b'
  "\\bthe agent's\\b"
  '\bnot asked\b'
  'stress-test'
  'self-check'
  'recommendation stands'
  'research recommendation'
  '\bresearcher\b'
  'on the same terms'
)

run_scan() { # run_scan FILE... ; prints hits, returns 1 if any
  local hits=0 pat
  for pat in "${GENERIC[@]}"; do
    grep -InE -i -- "$pat" "$@" && hits=1
  done
  if [ -n "${PUBLIC_TEXT_TERMS_FILE:-}" ] && [ -r "$PUBLIC_TEXT_TERMS_FILE" ]; then
    while IFS= read -r pat; do
      [ -z "$pat" ] && continue
      grep -InE -i -- "$pat" "$@" && hits=1
    done < "$PUBLIC_TEXT_TERMS_FILE"
  else
    echo "NOTE: PUBLIC_TEXT_TERMS_FILE is not set or not readable; names are NOT checked." >&2
  fi
  return $hits
}

if [ "${1:-}" = "--self-test" ]; then
  tmp=$(mktemp -d) || exit 2
  trap 'rm -rf "$tmp"' EXIT
  printf 'A plain decision with its reason kept.\n' > "$tmp/clean.md"
  printf 'Decided 2026-09-19.\nConfirmed by the human, veto welcome.\n' > "$tmp/planted.md"
  printf 'zzqxfake\n' > "$tmp/terms"
  printf 'text that names zzqxfake\n' > "$tmp/planted-name.md"
  fail=0
  PUBLIC_TEXT_TERMS_FILE=$tmp/terms run_scan "$tmp/clean.md" >/dev/null 2>&1 \
    || { echo "self-test FAILED: clean file was flagged"; fail=1; }
  run_scan "$tmp/planted.md" >/dev/null 2>&1 \
    && { echo "self-test FAILED: planted phrase was not flagged"; fail=1; }
  PUBLIC_TEXT_TERMS_FILE=$tmp/terms run_scan "$tmp/planted-name.md" >/dev/null 2>&1 \
    && { echo "self-test FAILED: planted name was not flagged"; fail=1; }
  [ $fail -eq 0 ] && echo "self-test ok: the gate flags planted text and passes clean text"
  exit $fail
fi

cd "$(git rev-parse --show-toplevel)" || exit 2
if [ $# -gt 0 ]; then
  files=("$@")
else
  mapfile -t files < <(git ls-files --cached --others --exclude-standard | grep -v '^scripts/check-public-text.sh$')
fi
run_scan "${files[@]}"
status=$?
[ $status -eq 0 ] && echo "public-text gate: no known-bad phrasing (a floor, not proof the text reads as the owner's own)"
exit $status
