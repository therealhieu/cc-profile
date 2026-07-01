#!/usr/bin/env bash
# README + repo-layout checks for the generated Homebrew tap formula (Task 3.1).
# Ensures the stale in-repo formula is gone and the README documents the
# generated tap formula instead of the old source-build reference.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STALE_FORMULA="${ROOT}/Formula/cc-profile.rb"
README="${ROOT}/README.md"

failures=0

fail() {
  local label="${1}"
  local message="${2}"
  echo "FAIL ${label}: ${message}" >&2
  failures=$((failures + 1))
}

test_stale_formula_removed() {
  local label="stale_formula_removed"
  if [[ -f "${STALE_FORMULA}" ]]; then
    fail "${label}" "expected ${STALE_FORMULA} to be removed"
  fi
}

test_readme_no_build_from_source_snippet() {
  local label="readme_no_build_from_source_snippet"
  if [[ ! -f "${README}" ]]; then
    fail "${label}" "README missing, cannot check contents"
    return
  fi
  if grep -q -- "build-from-source ./Formula/cc-profile.rb" "${README}"; then
    fail "${label}" "README still contains stale build-from-source ./Formula/cc-profile.rb snippet"
  fi
}

test_readme_mentions_template_and_install_command() {
  local label="readme_mentions_template_and_install_command"
  if [[ ! -f "${README}" ]]; then
    fail "${label}" "README missing, cannot check contents"
    return
  fi
  if ! grep -q -- "packaging/homebrew/cc-profile.rb.tmpl" "${README}"; then
    fail "${label}" "README missing reference to packaging/homebrew/cc-profile.rb.tmpl"
  fi
  if ! grep -q -- "brew install therealhieu/tap/cc-profile" "${README}"; then
    fail "${label}" "README missing brew install therealhieu/tap/cc-profile"
  fi
}

test_stale_formula_removed
test_readme_no_build_from_source_snippet
test_readme_mentions_template_and_install_command

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} readme_homebrew test(s) failed" >&2
  exit 1
fi

echo "readme_homebrew_test: ok"
