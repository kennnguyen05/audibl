---
name: run
description: Launch Audible (Tauri dev) for manual testing or screenshots. Use whenever asked to run, start, launch, or verify the app works.
---

# Running Audible

Audible is a menu bar app. On first launch (or while onboarding is incomplete) the
main window opens. After onboarding, with Start Hidden on, only the menu bar icon
appears.

## Always kill stale processes first

`bun run tauri dev` runs `target/debug/audible` in the foreground and never returns.
A background launch from an earlier session can still be running. Audible enforces
single-instance, so a new launch only refocuses the stale instance (and a foreground
launch hangs the tool call forever).

Before every launch:

```bash
pkill -f "target/debug/audible" 2>/dev/null
pkill -f "tauri dev" 2>/dev/null
sleep 1
```

## Launch in background

Never run `bun run tauri dev` in the foreground. Always background it, with cargo and
bun on PATH (cargo is not on PATH by default on this machine):

```bash
export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"
bun run tauri dev > /path/to/scratchpad/dev.log 2>&1
```

Use `run_in_background: true` on the Bash call. The first build is slow
(`transcribe-cpp` compiles C++ through cmake); later runs are fast.

Debug-only env overrides:
- `AUDIBLE_UNLOAD_SECS=30` shortens the 10-minute idle model unload.

## Confirm it is up

Poll the log, don't guess:

```bash
grep -q "Running \`target/debug/audible\`" /path/to/scratchpad/dev.log
```

Vite's `ready in Nms` and `Local: http://localhost:1420/` lines come first.

Useful log greps: `Model loaded`, `Model unloaded`, `Transcribed`, `Pasted`,
`Groq cleanup`. The persistent log file is
`~/Library/Logs/com.kennnguyen.audible/audible.log`.

## Fresh onboarding

Delete app data before launch to see first-run onboarding:

```bash
rm -rf ~/Library/Application\ Support/com.kennnguyen.audible
```

This also deletes the downloaded model (about 1.5 GB).

## Cleanup

Kill the same two process patterns when done (see above) so the next launch doesn't
inherit a stale instance.
