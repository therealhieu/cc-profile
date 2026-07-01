#!/usr/bin/env bash
# Local CI for cc-profile. Same jobs run in .github/workflows/ci.yml.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

usage() {
  cat <<'EOF'
Usage: ./scripts/ci.sh [job ...]
       ./scripts/ci.sh --from <job>

Jobs (default: run all in order):
  fmt            cargo fmt --check
  clippy         cargo clippy with -D warnings
  test           cargo test --workspace
  package        cargo package --list
  publish-dry-run  cargo publish --dry-run
  render-formula   render Homebrew formula from SHA256SUMS
  release-workflow  check release.yml wires up the bump-formula job
  readme-homebrew  check README documents the generated tap formula
  homebrew-runbook  check docs/homebrew-automation.md bootstrap runbook exists

Examples:
  ./scripts/ci.sh fmt test
  ./scripts/ci.sh --from clippy
EOF
}

job_fmt() {
  echo "==> fmt"
  cargo fmt --all -- --check
}

job_clippy() {
  echo "==> clippy"
  cargo clippy --all-targets --all-features -- -D warnings
}

job_test() {
  echo "==> test"
  cargo test --workspace
}

job_package() {
  echo "==> package"
  cargo package --list --allow-dirty
}

job_publish_dry_run() {
  echo "==> publish-dry-run"
  cargo publish --dry-run --allow-dirty
}

job_render_formula() {
  echo "==> render-formula"
  ./tests/render_formula_test.sh
}

job_release_workflow() {
  echo "==> release-workflow"
  ./tests/release_workflow_test.sh
}

job_readme_homebrew() {
  echo "==> readme-homebrew"
  ./tests/readme_homebrew_test.sh
}

job_homebrew_runbook() {
  echo "==> homebrew-runbook"
  ./tests/homebrew_runbook_test.sh
}

ALL_JOBS=(fmt clippy test package publish-dry-run render-formula release-workflow readme-homebrew homebrew-runbook)

run_job() {
  case "${1}" in
    fmt) job_fmt ;;
    clippy) job_clippy ;;
    test) job_test ;;
    package) job_package ;;
    publish-dry-run) job_publish_dry_run ;;
    render-formula) job_render_formula ;;
    release-workflow) job_release_workflow ;;
    readme-homebrew) job_readme_homebrew ;;
    homebrew-runbook) job_homebrew_runbook ;;
    *)
      echo "Unknown job: ${1}" >&2
      usage >&2
      exit 1
      ;;
  esac
}

FROM_MODE=false
START_JOB=""
JOBS=()

while [[ $# -gt 0 ]]; do
  case "${1}" in
    -h | --help)
      usage
      exit 0
      ;;
    --from)
      FROM_MODE=true
      START_JOB="${2:?--from requires a job name}"
      shift 2
      ;;
    *)
      JOBS+=("${1}")
      shift
      ;;
  esac
done

if [[ ${#JOBS[@]} -eq 0 && "${FROM_MODE}" == false ]]; then
  JOBS=("${ALL_JOBS[@]}")
elif [[ "${FROM_MODE}" == true ]]; then
  JOBS=()
  found=false
  for j in "${ALL_JOBS[@]}"; do
    if [[ "${j}" == "${START_JOB}" ]]; then
      found=true
    fi
    if [[ "${found}" == true ]]; then
      JOBS+=("${j}")
    fi
  done
  if [[ ${#JOBS[@]} -eq 0 ]]; then
    echo "Job not found for --from: ${START_JOB}" >&2
    exit 1
  fi
fi

for j in "${JOBS[@]}"; do
  run_job "${j}"
done

echo "==> ci: all requested jobs passed"