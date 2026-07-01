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

RENDER_SCRIPT="${ROOT}/scripts/render-formula.sh"
SAMPLE_SUMS="${ROOT}/tests/fixtures/SHA256SUMS.sample"

test_render_formula_substitutes_version_and_hashes() {
  local label="render_formula_substitutes_version_and_hashes"
  if [[ ! -x "${RENDER_SCRIPT}" ]]; then
    fail "${label}" "expected executable script at ${RENDER_SCRIPT}"
    return
  fi
  local output
  if ! output="$("${RENDER_SCRIPT}" "1.2.3" "${SAMPLE_SUMS}")"; then
    fail "${label}" "render-formula.sh exited nonzero for valid input"
    return
  fi
  if [[ "${output}" != *"1.2.3"* ]]; then
    fail "${label}" "rendered output missing version 1.2.3"
  fi
  local hash
  for hash in \
    "0a78ded75877c593b1e50b9e2dbd9508fd87fbf0d6482194e10c381f607f1dc3" \
    "ad8722c751c15195d578841dbd52f992bbb8a31d23b5c95f9ba7c979907d074b" \
    "3a637ef9a5c561f7323746408b5f23d715a10ee1f9b78ab7c6f57973ae49f12e"; do
    if [[ "${output}" != *"${hash}"* ]]; then
      fail "${label}" "rendered output missing expected hash ${hash}"
    fi
  done
  if [[ "${output}" == *"__"*"__"* ]]; then
    fail "${label}" "rendered output still contains a __ placeholder token"
  fi
}

test_render_formula_fails_on_missing_target() {
  local label="render_formula_fails_on_missing_target"
  if [[ ! -x "${RENDER_SCRIPT}" ]]; then
    fail "${label}" "expected executable script at ${RENDER_SCRIPT}"
    return
  fi
  local tmp_sums
  tmp_sums="$(mktemp)"
  # Only darwin arm64 and darwin x86_64 lines; linux target line omitted.
  cat >"${tmp_sums}" <<'EOF'
0a78ded75877c593b1e50b9e2dbd9508fd87fbf0d6482194e10c381f607f1dc3  cc-profile-v1.2.3-aarch64-apple-darwin.tar.gz
ad8722c751c15195d578841dbd52f992bbb8a31d23b5c95f9ba7c979907d074b  cc-profile-v1.2.3-x86_64-apple-darwin.tar.gz
EOF
  if "${RENDER_SCRIPT}" "1.2.3" "${tmp_sums}" >/dev/null 2>/dev/null; then
    fail "${label}" "expected nonzero exit when a target sha256 is missing"
  fi
  rm -f "${tmp_sums}"
}

test_template_file_exists
test_template_contains_placeholder_tokens
test_template_has_no_hardcoded_version
test_render_formula_substitutes_version_and_hashes
test_render_formula_fails_on_missing_target

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} render_formula test(s) failed" >&2
  exit 1
fi

echo "render_formula_test: ok"
