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

# Files where the owner's own name is the correct thing to find, so the name
# patterns are not applied to them. The rule exists to stop a line revealing
# that somebody else wrote or approved the text; a copyright line does the
# opposite and says whose work it is, which is what every line here is meant
# to read as. Paths are as this script is given them, relative to the repository
# root. Keep this list short and specific: it is a list, and not a pattern,
# because each entry should be a deliberate decision. The generic patterns
# below still apply to these files, and the name patterns still apply
# everywhere else — weakening a pattern, or dropping a name from the terms
# file, would hide that name where it does not belong.
NAME_OK_FILES=(
  LICENSE
)

name_ok() { # name_ok FILE ; true when an owner's name belongs in this file
  local candidate=$1 allowed
  for allowed in "${NAME_OK_FILES[@]}"; do
    [ "$candidate" = "$allowed" ] && return 0
  done
  return 1
}

run_scan() { # run_scan FILE... ; prints hits, returns 1 if any
  local hits=0 pat file
  local -a name_files=()
  for pat in "${GENERIC[@]}"; do
    grep -InE -i -- "$pat" "$@" && hits=1
  done
  for file in "$@"; do
    name_ok "$file" || name_files+=("$file")
  done
  if [ -n "${PUBLIC_TEXT_TERMS_FILE:-}" ] && [ -r "$PUBLIC_TEXT_TERMS_FILE" ]; then
    if [ ${#name_files[@]} -gt 0 ]; then
      while IFS= read -r pat; do
        [ -z "$pat" ] && continue
        grep -InE -i -- "$pat" "${name_files[@]}" && hits=1
      done < "$PUBLIC_TEXT_TERMS_FILE"
    fi
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
  printf 'Copyright (c) 2026 zzqxfake\n' > "$tmp/name-ok.md"
  ( NAME_OK_FILES=("$tmp/name-ok.md")
    PUBLIC_TEXT_TERMS_FILE=$tmp/terms run_scan "$tmp/name-ok.md" >/dev/null 2>&1 ) \
    || { echo "self-test FAILED: a name was flagged in a file where a name belongs"; fail=1; }
  ( NAME_OK_FILES=("$tmp/name-ok.md")
    PUBLIC_TEXT_TERMS_FILE=$tmp/terms run_scan "$tmp/planted-name.md" >/dev/null 2>&1 ) \
    && { echo "self-test FAILED: the exception let a name through in an ordinary file"; fail=1; }
  ( NAME_OK_FILES=("$tmp/name-ok.md")
    printf 'Confirmed by the human.\n' > "$tmp/name-ok-but-provenance.md"
    NAME_OK_FILES=("$tmp/name-ok-but-provenance.md") run_scan "$tmp/name-ok-but-provenance.md" >/dev/null 2>&1 ) \
    && { echo "self-test FAILED: a file exempt from the name patterns escaped the generic ones too"; fail=1; }
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
