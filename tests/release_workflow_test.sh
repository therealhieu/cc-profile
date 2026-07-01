#!/usr/bin/env bash
# Structural checks for the bump-formula job in .github/workflows/release.yml.
# Asserts the job exists and wires up the pieces the plan requires, without
# actually running the workflow.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="${ROOT}/.github/workflows/release.yml"

failures=0

fail() {
  local label="${1}"
  local message="${2}"
  echo "FAIL ${label}: ${message}" >&2
  failures=$((failures + 1))
}

test_workflow_file_exists() {
  local label="workflow_file_exists"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "expected workflow at ${WORKFLOW}"
  fi
}

test_bump_formula_job_present() {
  local label="bump_formula_job_present"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check for job"
    return
  fi
  if ! grep -Eq '^[[:space:]]*bump-formula:' "${WORKFLOW}"; then
    fail "${label}" "missing bump-formula: job in ${WORKFLOW}"
  fi
}

test_bump_formula_needs_release() {
  local label="bump_formula_needs_release"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check needs"
    return
  fi
  if ! grep -Eq '^[[:space:]]*needs:[[:space:]]*release[[:space:]]*$' "${WORKFLOW}"; then
    fail "${label}" "missing 'needs: release' in ${WORKFLOW}"
  fi
}

test_bump_formula_tag_guard() {
  local label="bump_formula_tag_guard"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check tag guard"
    return
  fi
  if ! grep -Fq "if: startsWith(github.ref, 'refs/tags/')" "${WORKFLOW}"; then
    fail "${label}" "missing tag guard 'if: startsWith(github.ref, ...)' in ${WORKFLOW}"
  fi
}

test_bump_formula_brew_style() {
  local label="bump_formula_brew_style"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check brew style"
    return
  fi
  if ! grep -Fq "brew style" "${WORKFLOW}"; then
    fail "${label}" "missing 'brew style' invocation in ${WORKFLOW}"
  fi
}

test_bump_formula_tap_token() {
  local label="bump_formula_tap_token"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check tap token"
    return
  fi
  if ! grep -Fq "HOMEBREW_TAP_TOKEN" "${WORKFLOW}"; then
    fail "${label}" "missing reference to HOMEBREW_TAP_TOKEN in ${WORKFLOW}"
  fi
}

test_bump_formula_render_script() {
  local label="bump_formula_render_script"
  if [[ ! -f "${WORKFLOW}" ]]; then
    fail "${label}" "workflow missing, cannot check render-formula.sh"
    return
  fi
  if ! grep -Fq "render-formula.sh" "${WORKFLOW}"; then
    fail "${label}" "missing reference to render-formula.sh in ${WORKFLOW}"
  fi
}

test_workflow_file_exists
test_bump_formula_job_present
test_bump_formula_needs_release
test_bump_formula_tag_guard
test_bump_formula_brew_style
test_bump_formula_tap_token
test_bump_formula_render_script

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} release_workflow test(s) failed" >&2
  exit 1
fi

echo "release_workflow_test: ok"
