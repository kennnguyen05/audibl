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
lsof -ti tcp:1420 | xargs kill 2>/dev/null   # orphaned Vite keeps the port
sleep 1
```

Symptom of a leftover Vite: `Error: Port 1420 is already in use` and
`The "beforeDevCommand" terminated with a non-zero status code`.

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

Ready line: `Shortcut 'option+space' ready` (only after onboarding is complete and
Accessibility is granted).

Useful log greps: `Model loaded`, `Model unloaded`, `Transcribed`, `Pasted`,
`Groq cleanup`. The persistent log file is
`~/Library/Logs/com.kennnguyen.audible/audible.log`.

## Fresh onboarding

Delete app data before launch to see first-run onboarding:

```bash
rm -rf ~/Library/Application\ Support/com.kennnguyen.audible
```

This also deletes the downloaded model (about 1.5 GB).

## Automated dictation smoke test

Ask Ken first: it takes keyboard focus and plays speech through the speakers.
The shell running Claude has Accessibility, so synthetic keys reach the event tap.

1. Build a tiny Swift helper that posts `CGEvent` key events to `.cghidEventTap`
   (Option = keycode 58 with `.maskAlternate`, Space = 49, Return = 36, Esc = 53).
2. `osascript -e 'tell application "TextEdit" to activate'` and confirm TextEdit is
   frontmost before pressing anything (a fresh TextEdit launch can show an Open
   panel that swallows the paste).
3. Hold mode: Option+Space down, `say -v Samantha "…"`, keys up. Toggle mode: tap
   Option+Space, `say`, tap Return (or Esc to cancel).
4. Read the result with `osascript -e 'tell application "TextEdit" to get text of front document'`.

Screenshot only Audible's windows: list window ids with `CGWindowListCopyWindowInfo`
(the overlay is "Audible Overlay", 240×48, layer 25) and `screencapture -x -o -l<id>`.

## Cleanup

Kill the same two process patterns when done (see above) so the next launch doesn't
inherit a stale instance.
