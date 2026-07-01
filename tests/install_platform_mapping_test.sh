#!/usr/bin/env bash
# Platform-to-release-target mapping for install.sh (RED/GREEN harness).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=../install.sh
source "${ROOT}/install.sh"

failures=0

assert_eq() {
  local got="${1}"
  local want="${2}"
  local label="${3}"
  if [[ "${got}" != "${want}" ]]; then
    echo "FAIL ${label}: got '${got}', want '${want}'" >&2
    failures=$((failures + 1))
  fi
}

assert_eq "$(cc_profile_target_for_platform Darwin arm64)" "aarch64-apple-darwin" "darwin arm64"
assert_eq "$(cc_profile_target_for_platform Darwin x86_64)" "x86_64-apple-darwin" "darwin x86_64"
assert_eq "$(cc_profile_target_for_platform Linux x86_64)" "x86_64-unknown-linux-gnu" "linux x86_64"

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} mapping test(s) failed" >&2
  exit 1
fi

echo "install_platform_mapping_test: ok"