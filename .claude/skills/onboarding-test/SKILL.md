---
name: onboarding-test
description: Reset onboarding state and launch Audibl so the onboarding flow (microphone → accessibility → model) can be tested manually. Use whenever asked to test, retest, or verify onboarding.
---

# Onboarding test

Onboarding shows when `onboarding_complete` is `false` in `settings_store.json`
(see `src-tauri/src/settings.rs`). Delete just that file, not the whole app-data
dir, so the downloaded model stays and the model step auto-passes instead of
re-downloading 1.5 GB.

## Reset

```bash
rm -f ~/Library/Application\ Support/com.kennnguyen.audibl/settings_store.json
```

This also resets every other setting to default (shortcut, language, custom
words, etc.) — expected, since onboarding is normally a new-install path.
`history.json` and `models/` are untouched.

Mic and Accessibility permission steps read live macOS TCC state, not the
settings file. If Audibl is already authorized, those two steps auto-pass
regardless of this reset — deliberately: revoking them requires the user to
act in System Settings (Privacy & Security → Microphone / Accessibility, or
`tccutil reset Microphone|Accessibility com.kennnguyen.audibl`), so ask Ken
first if the permission screens themselves need exercising, not just the
model/completion step.

## Launch

Same as the `run` skill: kill stale instances first, then background the dev
build.

```bash
pkill -f "target/debug/audible" 2>/dev/null
pkill -f "tauri dev" 2>/dev/null
lsof -ti tcp:1420 | xargs kill 2>/dev/null
sleep 1

export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"
bun run tauri dev > /path/to/scratchpad/dev.log 2>&1
```

Use `run_in_background: true`. Poll the log instead of guessing:

```bash
grep -q "Running \`target/debug/audible\`" /path/to/scratchpad/dev.log
```

The main window should open straight to the onboarding UI (mic step, or
accessibility/model if mic is already granted) instead of the sidebar.
`Shortcut 'option+space' ready` only appears once onboarding finishes and
Accessibility is granted.

## Full fresh-install test (with model download)

To also exercise the model download step, delete the whole app-data dir
instead (see the `run` skill's "Fresh onboarding" section) — this deletes the
model too, so the download takes a while:

```bash
rm -rf ~/Library/Application\ Support/com.kennnguyen.audibl
```

## Cleanup

Kill the same process patterns when done so the next launch doesn't inherit a
stale instance.
