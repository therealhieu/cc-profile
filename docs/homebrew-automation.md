# Homebrew automation bootstrap

One-time setup so the `bump-formula` release job (`.github/workflows/release.yml`) can push
the rendered formula to the separate `therealhieu/homebrew-tap` repo, plus a safety net for
formula drift.

## 1. Create the PAT

`GITHUB_TOKEN` is scoped to the repo the workflow runs in (`therealhieu/cc-profile`) and
cannot push to a different repo. The `bump-formula` job needs a Personal Access Token that
can write to `therealhieu/homebrew-tap`.

Create a **fine-grained PAT** at <https://github.com/settings/tokens?type=beta>:

| Setting | Value |
|---|---|
| Repository access | Only select repositories -> `therealhieu/homebrew-tap` |
| Permissions | Contents: Read and write |

## 2. Add the secret

Add the PAT as a secret on the `cc-profile` repo (the repo the workflow runs in), not the tap
repo:

```bash
gh secret set HOMEBREW_TAP_TOKEN --repo therealhieu/cc-profile
# paste the PAT when prompted
```

## 3. First formula push

Either:

- **Wait for the next tagged release.** The `bump-formula` job runs automatically on tag push
  once the secret exists.
- **Or push once manually** against the latest release's `SHA256SUMS`:

```bash
tag="$(gh release view --repo therealhieu/cc-profile --json tagName -q .tagName)"
gh release download "$tag" --repo therealhieu/cc-profile -p SHA256SUMS -D /tmp
scripts/render-formula.sh "${tag#v}" /tmp/SHA256SUMS > cc-profile.rb
brew style cc-profile.rb
# then copy cc-profile.rb into therealhieu/homebrew-tap at Formula/cc-profile.rb and push
```

## 4. Livecheck safety net

This workflow belongs in the **tap repo** (`therealhieu/homebrew-tap`), not this repo. Add it
as `.github/workflows/livecheck.yml` there. It runs on a schedule and fails if the formula's
`sha256`/`version` drift from the latest `cc-profile` release, catching a missed or broken
`bump-formula` run.

```yaml
name: livecheck

on:
  schedule:
    - cron: "0 6 * * *"
  workflow_dispatch: {}

jobs:
  livecheck:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: brew livecheck
        run: |
          brew livecheck --formula Formula/cc-profile.rb --json --quiet > livecheck.json
          cat livecheck.json
          if jq -e '.[0].version.current != .[0].version.latest' livecheck.json >/dev/null; then
            echo "cc-profile formula is outdated" >&2
            exit 1
          fi
```
