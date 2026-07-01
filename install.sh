#!/usr/bin/env bash
# Standalone installer for cc-profile from GitHub Releases.
set -euo pipefail

CC_PROFILE_REPO="${CC_PROFILE_REPO:-therealhieu/cc-profile}"
CC_PROFILE_INSTALL_DIR="${CC_PROFILE_INSTALL_DIR:-${HOME}/.local/bin}"
CC_PROFILE_RECEIPT_DIR="${CC_PROFILE_RECEIPT_DIR:-${HOME}/.cc-profile}"

cc_profile_target_for_platform() {
  local os="${1}"
  local arch="${2}"
  case "${os}:${arch}" in
    Darwin:arm64) echo "aarch64-apple-darwin" ;;
    Darwin:x86_64) echo "x86_64-apple-darwin" ;;
    Linux:x86_64) echo "x86_64-unknown-linux-gnu" ;;
    *)
      echo "unsupported platform ${os}/${arch}" >&2
      return 1
      ;;
  esac
}

cc_profile_detect_target() {
  cc_profile_target_for_platform "$(uname -s)" "$(uname -m)"
}

cc_profile_verify_sha256sums() {
  local archive_path="${1}"
  local sums_path="${2}"
  local base
  base="$(basename "${archive_path}")"
  local expected
  expected="$(awk -v f="${base}" '$2 == f { print $1; exit }' "${sums_path}")"
  if [[ -z "${expected}" ]]; then
    echo "SHA256SUMS has no entry for ${base}" >&2
    return 1
  fi
  local actual
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${archive_path}" | awk '{print $1}')"
  else
    actual="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
  fi
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Checksum mismatch for ${base}" >&2
    return 1
  fi
}

cc_profile_write_receipt() {
  local version="${1}"
  local receipt="${CC_PROFILE_RECEIPT_DIR}/install.toml"
  mkdir -p "${CC_PROFILE_RECEIPT_DIR}"
  chmod 700 "${CC_PROFILE_RECEIPT_DIR}" 2>/dev/null || true
  cat >"${receipt}" <<EOF
method = "standalone"
source = "github-releases"
installed_version = "${version}"
installed_at = "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
EOF
  chmod 600 "${receipt}"
}

cc_profile_install_main() {
  local dry_run=false
  while [[ $# -gt 0 ]]; do
    case "${1}" in
      --dry-run)
        dry_run=true
        shift
        ;;
      -h | --help)
        cat <<'EOF'
Usage: install.sh [--dry-run]

Environment:
  CC_PROFILE_REPO          GitHub repo (default: therealhieu/cc-profile)
  CC_PROFILE_INSTALL_DIR   Install prefix (default: ~/.local/bin)
  CC_PROFILE_RECEIPT_DIR   Receipt directory (default: ~/.cc-profile)
EOF
        exit 0
        ;;
      *)
        echo "Unknown argument: ${1}" >&2
        exit 1
        ;;
    esac
  done

  local target
  target="$(cc_profile_detect_target)"
  local api="https://api.github.com/repos/${CC_PROFILE_REPO}/releases/latest"
  local version asset_name archive_url sums_url

  if [[ "${dry_run}" == true ]]; then
    echo "dry-run: target=${target}"
    echo "dry-run: would query ${api}"
    echo "dry-run: would install cc-profile to ${CC_PROFILE_INSTALL_DIR}/cc-profile"
    echo "dry-run: would write receipt ${CC_PROFILE_RECEIPT_DIR}/install.toml (method = \"standalone\")"
    echo "dry-run: would verify archive against SHA256SUMS before install"
    exit 0
  fi

  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required" >&2
    exit 1
  fi

  local release_json
  release_json="$(curl -fsSL "${api}")"
  version="$(printf '%s' "${release_json}" | sed -n 's/.*"tag_name":[[:space:]]*"\(v[^"]*\)".*/\1/p' | head -1)"
  version="${version#v}"
  if [[ -z "${version}" ]]; then
    echo "Could not determine latest release version" >&2
    exit 1
  fi

  asset_name="cc-profile-v${version}-${target}.tar.gz"
  archive_url="https://github.com/${CC_PROFILE_REPO}/releases/download/v${version}/${asset_name}"
  sums_url="https://github.com/${CC_PROFILE_REPO}/releases/download/v${version}/SHA256SUMS"

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT

  curl -fsSL -o "${tmp}/archive.tar.gz" "${archive_url}"
  curl -fsSL -o "${tmp}/SHA256SUMS" "${sums_url}"
  cc_profile_verify_sha256sums "${tmp}/archive.tar.gz" "${tmp}/SHA256SUMS"

  tar -xzf "${tmp}/archive.tar.gz" -C "${tmp}"
  if [[ ! -f "${tmp}/cc-profile" ]]; then
    echo "Archive did not contain cc-profile binary" >&2
    exit 1
  fi
  chmod +x "${tmp}/cc-profile"

  mkdir -p "${CC_PROFILE_INSTALL_DIR}"
  install -m 0755 "${tmp}/cc-profile" "${CC_PROFILE_INSTALL_DIR}/cc-profile"
  cc_profile_write_receipt "${version}"

  echo "Installed cc-profile ${version} to ${CC_PROFILE_INSTALL_DIR}/cc-profile"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  cc_profile_install_main "$@"
fi