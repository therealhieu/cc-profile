#!/usr/bin/env bash
# Homebrew automation bootstrap runbook checks (Task 3.2).
# Ensures docs/homebrew-automation.md exists and documents the one-time
# HOMEBREW_TAP_TOKEN PAT setup, the gh secret set command, the manual
# render-formula.sh path, and the livecheck safety-net workflow.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOC="${ROOT}/docs/homebrew-automation.md"

failures=0

fail() {
  local label="${1}"
  local message="${2}"
  echo "FAIL ${label}: ${message}" >&2
  failures=$((failures + 1))
}

test_doc_exists() {
  local label="doc_exists"
  if [[ ! -f "${DOC}" ]]; then
    fail "${label}" "expected doc at ${DOC}"
  fi
}

test_doc_mentions_secret_name() {
  local label="doc_mentions_secret_name"
  if [[ ! -f "${DOC}" ]]; then
    fail "${label}" "doc missing, cannot check contents"
    return
  fi
  if ! grep -q -- "HOMEBREW_TAP_TOKEN" "${DOC}"; then
    fail "${label}" "doc missing reference to HOMEBREW_TAP_TOKEN"
  fi
}

test_doc_mentions_gh_secret_set() {
  local label="doc_mentions_gh_secret_set"
  if [[ ! -f "${DOC}" ]]; then
    fail "${label}" "doc missing, cannot check contents"
    return
  fi
  if ! grep -q -- "gh secret set" "${DOC}"; then
    fail "${label}" "doc missing reference to gh secret set"
  fi
}

test_doc_mentions_livecheck() {
  local label="doc_mentions_livecheck"
  if [[ ! -f "${DOC}" ]]; then
    fail "${label}" "doc missing, cannot check contents"
    return
  fi
  if ! grep -q -- "livecheck" "${DOC}"; then
    fail "${label}" "doc missing reference to livecheck"
  fi
}

test_doc_mentions_render_formula_script() {
  local label="doc_mentions_render_formula_script"
  if [[ ! -f "${DOC}" ]]; then
    fail "${label}" "doc missing, cannot check contents"
    return
  fi
  if ! grep -q -- "render-formula.sh" "${DOC}"; then
    fail "${label}" "doc missing reference to render-formula.sh"
  fi
}

test_doc_exists
test_doc_mentions_secret_name
test_doc_mentions_gh_secret_set
test_doc_mentions_livecheck
test_doc_mentions_render_formula_script

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} homebrew_runbook test(s) failed" >&2
  exit 1
fi

echo "homebrew_runbook_test: ok"
