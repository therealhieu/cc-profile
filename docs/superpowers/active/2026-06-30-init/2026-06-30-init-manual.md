# 2026-06-30-init — Manual Verification

## Purpose

Verify interactive `cc-profile` behavior that is not fully covered by automated integration tests.

## Setup

Use a temporary home directory so local `~/.cc-profile` is not touched:

```bash
export CC_PROFILE_MANUAL_HOME="$(mktemp -d)"
export HOME="$CC_PROFILE_MANUAL_HOME"
cargo build
```

If testing `Start Claude` without launching a real Claude Code session, create a shim:

```bash
cat > "$CC_PROFILE_MANUAL_HOME/claude" <<'EOF'
#!/usr/bin/env bash
env | grep -E '^(HTTP_PROXY|HTTPS_PROXY|CUSTOM_FLAG|ANTHROPIC_)=' | sort
printf 'args=%s\n' "$*"
EOF
chmod +x "$CC_PROFILE_MANUAL_HOME/claude"
export PATH="$CC_PROFILE_MANUAL_HOME:$PATH"
```

## Flow 1 — First Run With No Active Profile

1. Run:

   ```bash
   cargo run --
   ```

2. Verify the main screen shows:
   - `No active profile configured.`
   - options include `New profile`, `List profiles`, `Show config`, `Args`, `Envs`, `Quit`
   - `Start Claude` is not shown.

3. Select `Quit`.

Expected: command exits successfully and no profile is active.

## Flow 2 — Create Profile and Set Active

1. Run:

   ```bash
   cargo run --
   ```

2. Select `New profile`.
3. Enter:

   ```text
   Profile name: profile-a
   Endpoint: https://api.anthropic.com
   API key: sk-ant-manual-secret
   Fable model: claude-fable-5
   Opus model: claude-opus-4-8
   Sonnet model: claude-sonnet-4-6
   Haiku model: claude-haiku-4-5-20251001
   Set as active profile? Yes
   ```

4. Verify output shows:

   ```text
   Profile "profile-a" saved.
   Profile "profile-a" is now active.
   ```

5. Verify the main screen now shows:
   - `Active profile: profile-a`
   - `API key: sk-ant-manual-secret` unmasked
   - `Start Claude` option present.

Expected: profile is saved to `$HOME/.cc-profile` and active.

## Flow 3 — List, View, Edit, Rename, and Set Active

1. Create a second profile through `New profile` named `profile-b` and choose `Set as active profile? No`.
2. Select `List profiles`.
3. Verify `profile-a  active` is marked and `profile-b` is present.
4. Select `profile-b`.
5. Verify profile detail shows endpoint, API key, Fable, Opus, Sonnet, and Haiku values unmasked.
6. Select `Set active`.
7. Verify output:

   ```text
   Profile "profile-b" is now active.
   ```

8. Select `profile-b` again, then `Edit`, then `Profile name`.
9. Rename it to `profile-c`.
10. Verify the main screen shows `Active profile: profile-c`.

Expected: renaming an active profile updates top-level `active_profile`.

## Flow 4 — Delete Active Profile

1. Select `List profiles`.
2. Select the active profile.
3. Select `Delete`.
4. At confirmation, choose `No`.
5. Verify profile remains.
6. Select `Delete` again.
7. At confirmation, choose `Yes`.
8. Verify output shows:

   ```text
   Profile "profile-c" deleted.
   No active profile is currently set.
   ```

9. Verify the main screen hides `Start Claude`.

Expected: deleting the active profile clears `active_profile`.

## Flow 5 — Args Toggle

1. Select `Args`.
2. Verify initial line:

   ```text
   --dangerously-skip-permissions: false
   ```

3. Select `Toggle dangerously-skip-permissions`.
4. Verify value becomes `true`.
5. Toggle again.
6. Verify value becomes `false`.
7. Select `Back`.

Expected: `$HOME/.cc-profile` updates immediately after each toggle.

## Flow 6 — Add, Edit, and Delete Env Vars

1. Select `Envs`.
2. Select `Add env var`.
3. Enter:

   ```text
   Env key: HTTP_PROXY
   Env value: http://localhost:7890
   ```

4. Verify output:

   ```text
   Saved env var HTTP_PROXY.
   ```

5. Select `Edit env var`, choose `HTTP_PROXY`, and enter:

   ```text
   New value: http://localhost:8888
   ```

6. Verify output:

   ```text
   Updated HTTP_PROXY.
   ```

7. Select `Delete env var`, choose `HTTP_PROXY`, confirm `Yes`.
8. Verify output:

   ```text
   Deleted HTTP_PROXY.
   ```

Expected: env keys use `[A-Z_][A-Z0-9_]*`, invalid lowercase or dash-containing keys are rejected with a clear error.

## Flow 7 — Show Config

1. Select `Show config`.
2. Verify output includes:
   - `Config file: <temporary-home>/.cc-profile`
   - `Active profile: <name or <none>>`
   - `[args]`
   - `[envs]` when envs exist
   - `[profiles.<name>]` when profiles exist
   - API keys shown as stored.
3. Select `Back`.

Expected: config is readable and API keys are visible only in this normal display path.

## Flow 8 — Start Claude With Env Precedence

1. Ensure an active profile exists.
2. Add a global env var with key `ANTHROPIC_API_KEY` and value `custom-env-key`.
3. Toggle `--dangerously-skip-permissions` to `true`.
4. Select `Start Claude`.
5. With the shim from setup, verify output includes:

   ```text
   ANTHROPIC_API_KEY=sk-ant-...
   ANTHROPIC_BASE_URL=<active profile endpoint>
   ANTHROPIC_DEFAULT_FABLE_MODEL=<active profile fable>
   ANTHROPIC_DEFAULT_OPUS_MODEL=<active profile opus>
   ANTHROPIC_DEFAULT_SONNET_MODEL=<active profile sonnet>
   ANTHROPIC_DEFAULT_HAIKU_MODEL=<active profile haiku>
   args=--dangerously-skip-permissions
   ```

Expected: profile Claude env vars override duplicate global custom env vars.

## Flow 9 — Missing Active Profile Warning

1. Exit the app.
2. Create a config with a missing active profile:

   ```bash
   cat > "$HOME/.cc-profile" <<'EOF'
version = 1
active_profile = "missing-profile"
EOF
   ```

3. Run:

   ```bash
   cargo run --
   ```

4. Verify the main screen shows:
   - `Active profile 'missing-profile' does not exist.`
   - `Create or select a profile before starting Claude.`
   - `Start Claude` is not shown.

Expected: user is guided to create or select a profile before launch.

## Flow 10 — Invalid TOML Recovery

1. Exit the app.
2. Corrupt the config file:

   ```bash
   printf 'not valid toml = [' > "$HOME/.cc-profile"
   ```

3. Run:

   ```bash
   cargo run -- show
   ```

4. Verify command exits non-zero and prints a clear invalid TOML error.
5. Verify the corrupt file was not overwritten:

   ```bash
   cat "$HOME/.cc-profile"
   ```

Expected: invalid TOML is reported and preserved.

## Cleanup

```bash
rm -rf "$CC_PROFILE_MANUAL_HOME"
unset CC_PROFILE_MANUAL_HOME
```
