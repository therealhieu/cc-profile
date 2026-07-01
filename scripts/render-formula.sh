#!/usr/bin/env bash
# Render the Homebrew formula template by substituting the version and
# per-target sha256 hashes parsed from a SHA256SUMS file.
#
# Usage: render-formula.sh <version> <sha256sums-path>
#
# Note: macOS ships bash 3.2 by default, which lacks associative arrays
# (declare -A). This script uses a case statement instead so it works on
# both macOS's stock bash and newer bash (e.g. Linux CI runners).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE="${ROOT}/packaging/homebrew/cc-profile.rb.tmpl"

usage() {
  echo "usage: $(basename "${BASH_SOURCE[0]}") <version> <sha256sums-path>" >&2
}

if [[ "${#}" -ne 2 ]]; then
  usage
  exit 1
fi

version="${1}"
sums="${2}"

if [[ ! -f "${TEMPLATE}" ]]; then
  echo "template not found at ${TEMPLATE}" >&2
  exit 1
fi

if [[ ! -f "${sums}" ]]; then
  echo "sha256sums file not found at ${sums}" >&2
  exit 1
fi

placeholder_for_target() {
  case "${1}" in
    aarch64-apple-darwin) echo "__SHA_DARWIN_ARM64__" ;;
    x86_64-apple-darwin) echo "__SHA_DARWIN_X86_64__" ;;
    x86_64-unknown-linux-gnu) echo "__SHA_LINUX_X86_64__" ;;
    *) return 1 ;;
  esac
}

rendered="$(cat "${TEMPLATE}")"

for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu; do
  placeholder="$(placeholder_for_target "${target}")"
  file="cc-profile-v${version}-${target}.tar.gz"
  sha="$(awk -v f="${file}" '$2==f{print $1; exit}' "${sums}")"
  if [[ -z "${sha}" ]]; then
    echo "missing sha256 for ${file}" >&2
    exit 1
  fi
  rendered="${rendered//${placeholder}/${sha}}"
done

rendered="${rendered//__VERSION__/${version}}"

printf '%s\n' "${rendered}"
