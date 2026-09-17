# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Audible v1 is being built milestone by milestone from the plan at
`~/.claude/plans/let-s-now-create-audible-crystalline-codd.md` (read its "Handoff" section first).

- Milestone 1 (scaffold) done: Tauri 2 + React/TS/Vite/Tailwind v4, tray icon, hidden-window lifecycle, sidebar with 4 pages, tauri-specta bindings.
- Milestone 2 (settings, pages, i18n) done: setting commands, General/Dictionary/Advanced UIs, History placeholder, Keychain key commands, en/vi locales with parity check.

## What this project is

**Audible**: a macOS-only voice-to-text dictation app, cloning Wispr Flow. Press a shortcut, speak English or Vietnamese, and the text is pasted into the focused app. v1 scope:

- Transcribe Shortcut with Hold / Toggle modes (Toggle: Esc cancels, Return finishes)
- Local model (Qwen3-ASR 1.7B via `transcribe-cpp`, Metal), auto language detect, no model options in the UI
- Dictionary (custom words + replacements), local filler removal, History (last 10)
- Optional Groq "Clean and Reformat" step (key in the macOS Keychain)
- UI in English or Vietnamese

Out of scope for v1: Command Mode, snippets beyond replacements, per-app style settings.

## Commands

cargo is not on PATH on this machine. Prefix commands with `export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`.

```bash
bun install                        # JS deps
bun run tauri dev                  # run the app (use the `run` skill; always background it)
bun run build                      # tsc + vite build
bun run check:translations         # en/vi key parity (vi must match en exactly)
cd src-tauri && cargo test         # Rust unit tests; also regenerates src/bindings.ts
cd src-tauri && cargo clippy
bun scripts/generate-icons.ts      # regenerate placeholder app + tray icons
```

## Architecture

- `src-tauri/src/lib.rs`: Tauri setup, plugins, main window (created hidden, shown unless Start Hidden applies), close-to-hide with Accessory activation policy, specta command/event registration. Debug builds export `src/bindings.ts` at startup; `cargo test export_bindings` does the same without launching.
- `settings.rs`: one `AppSettings` struct (serde defaults, per-field salvage) in `settings_store.json`. `update_settings` persists and emits `settings-changed`. First-launch `app_language` comes from the macOS locale.
- `tray.rs`: menu bar icon + menu (Open, Copy Last Transcript, Cancel while busy, Quit, Secure Input warning). Updates coalesce into one main-thread apply. Icons are `include_bytes!` template PNGs from `src-tauri/resources/`.
- `tray_i18n.rs` + `build.rs`: tray strings generated from the `tray` section of `src/i18n/locales/{en,vi}/translation.json`, looked up by `app_language`.
- `commands.rs`: one setter command per setting; each returns the updated `AppSettings`. `set_clean_and_reformat(true)` fails with `no_api_key` until a key exists; `clear_groq_api_key` also turns Clean and Reformat off.
- `keychain.rs`: Groq key in the Keychain (service `com.kennnguyen.audible`, account `groq_api_key`) via `keyring`. `has_groq_api_key` reads attributes only (no Keychain prompt); reading the secret can prompt once per rebuild in unsigned dev builds.
- `autostart.rs`: Launch on Startup via `SMAppService` (fails harmlessly in `tauri dev`).
- Frontend: `src/main.tsx` → `App.tsx` (sidebar + pages in `src/pages/`), `src/stores/settings.ts` (zustand mirror; `applySettings(commands.setX(..))` stores the returned settings; `settings-changed` keeps it live), `src/components/ui/` (Group/Row, Toggle, Segmented, Select, TextField/Button, InfoTip), `src/i18n/` (react-i18next, language follows the stored setting and `settings-changed`), `src/overlay/` (recording overlay webview entry), `src/index.css` (color tokens, light/dark).

## UI rules

- Minimal, neutral design; it will be redesigned later.
- No hover info (ⓘ, `title=`) anywhere except the Advanced page.
- No model picker, model options, or unload-timeout setting.
- Every user-visible string goes in both `en` and `vi` locale files.

## Reference skeleton: `ref/Handy`

Read-only, never edit. Handy (MIT) is the architectural reference Audible ports selected modules from (audio/VAD, transcription manager, handy-keys shortcuts, overlay NSPanel, paste, tray). Ported code keeps Handy's MIT notice in `THIRD_PARTY_NOTICES.md`. Full detail: `ref/Handy/AGENTS.md`.

## Skill: `.claude/skills/run`

Launches `bun run tauri dev` in the background and checks `target/debug/audible` in the log. To screenshot only Audible's window (never the whole screen), find its window id with `CGWindowListCopyWindowInfo` and use `screencapture -x -o -l<id>`.
