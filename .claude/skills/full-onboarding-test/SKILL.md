---
name: full-onboarding-test
description: Wipe all app data (settings + downloaded model) and launch Audibl for a full start-to-finish manual test, including the model download. Use whenever asked to fully retest, fresh-install test, or test the model download / onboarding end to end.
---

# Full onboarding test

Deletes the whole app-data dir, not just `settings_store.json`, so onboarding
starts truly fresh: mic/Accessibility steps re-prompt at the OS level only if
you also reset TCC (see below), but the model step re-downloads from scratch
(~1.5 GB) and every setting (shortcut, language, custom words, history,
Groq key redaction state) returns to default.

Confirm with Ken before running this — it deletes the downloaded model and
all local settings/history, unlike the lighter `onboarding-test` skill.

## Reset

```bash
rm -rf ~/Library/Application\ Support/com.kennnguyen.audibl
```

This removes `settings_store.json`, `history.json`, and `models/` together.
The Keychain entry (service `com.kennnguyen.audibl`, account `groq_api_key`)
is untouched by this — it's OS Keychain, not app-data — so a previously saved
Groq key still validates once entered again; delete it separately if you also
want to test the no-key state:

```bash
security delete-generic-password -s com.kennnguyen.audibl -a groq_api_key 2>/dev/null
```

Mic and Accessibility permission steps read live macOS TCC state, not app
data, so they auto-pass if already granted. To also force those screens:

```bash
tccutil reset Microphone com.kennnguyen.audibl
tccutil reset Accessibility com.kennnguyen.audibl
```

Ask Ken first — this makes every app (not just Audibl) re-prompt for those
permissions and can't be scoped narrower.

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

## What to expect, start to finish

1. Main window opens straight to onboarding (mic step first, or later steps
   if permissions are already granted and weren't reset).
2. Mic step, then Accessibility step — each auto-passes if TCC already
   grants Audibl, otherwise prompts.
3. Model step downloads Qwen3-ASR 1.7B from scratch with progress; test
   Wi-Fi-off-mid-download (Retry resumes from `.partial`) and quit-mid-download
   (relaunch resumes) if exercising the full checklist in the project's
   "Not yet verified" list.
4. Finish → sidebar. Shortcut only registers (`Shortcut 'option+space' ready`
   in the log) once onboarding is complete and Accessibility is granted.
5. From here, walk the rest of the app: General (shortcut, Magic Touch +
   Groq key entry), Dictionary (custom words/replacements), Advanced,
   dictation itself (Hold/Toggle), History.

## Cleanup

Kill the same process patterns when done — but per project convention, leave
the dev process running after a successful verification so Ken can check the
app himself; only kill stale processes right before the *next* launch, or if
Ken explicitly asks you to stop it.
