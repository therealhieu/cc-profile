#!/usr/bin/env bash
# Homebrew formula template checks (RED/GREEN harness).
# Task 1.2 will append more test functions to this same file (e.g. rendering
# behavior of scripts/render-formula.sh). Keep cases as separate functions so
# new ones can be added without disturbing existing ones.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE="${ROOT}/packaging/homebrew/cc-profile.rb.tmpl"

failures=0

fail() {
  local label="${1}"
  local message="${2}"
  echo "FAIL ${label}: ${message}" >&2
  failures=$((failures + 1))
}

test_template_file_exists() {
  local label="template_file_exists"
  if [[ ! -f "${TEMPLATE}" ]]; then
    fail "${label}" "expected template at ${TEMPLATE}"
  fi
}

test_template_contains_placeholder_tokens() {
  local label="template_contains_placeholder_tokens"
  if [[ ! -f "${TEMPLATE}" ]]; then
    fail "${label}" "template missing, cannot check placeholders"
    return
  fi
  local token
  for token in "__VERSION__" "__SHA_DARWIN_ARM64__" "__SHA_DARWIN_X86_64__" "__SHA_LINUX_X86_64__"; do
    if ! grep -q -- "${token}" "${TEMPLATE}"; then
      fail "${label}" "missing placeholder token ${token}"
    fi
  done
}

test_template_has_no_hardcoded_version() {
  local label="template_has_no_hardcoded_version"
  if [[ ! -f "${TEMPLATE}" ]]; then
    fail "${label}" "template missing, cannot check for hardcoded version"
    return
  fi
  if grep -Eq 'v[0-9]+\.[0-9]+\.[0-9]+' "${TEMPLATE}"; then
    fail "${label}" "found a hardcoded real version string (e.g. v0.1.0) in template"
  fi
}

test_template_file_exists
test_template_contains_placeholder_tokens
test_template_has_no_hardcoded_version

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} render_formula test(s) failed" >&2
  exit 1
fi

echo "render_formula_test: ok"
