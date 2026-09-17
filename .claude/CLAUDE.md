# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Audible v1 was built milestone by milestone (one commit each) from the plan at
`~/.claude/plans/let-s-now-create-audible-crystalline-codd.md`. All seven milestones are implemented; the plan's manual E2E checklist is only partly run (see "Not yet verified").

- Milestone 1 (scaffold) done: Tauri 2 + React/TS/Vite/Tailwind v4, tray icon, hidden-window lifecycle, sidebar with 4 pages, tauri-specta bindings.
- Milestone 2 (settings, pages, i18n) done: setting commands, General/Dictionary/Advanced UIs, History placeholder, Keychain key commands, en/vi locales with parity check.
- Milestone 3 (audio and model) done: recorder + Silero VAD, mic/channel/mute, onboarding (mic → Accessibility → required model download with resume/verify), transcription manager with lazy load, 10-min unload, en/vi language guard.
  - Bake-off (2026-09-17, 15 synthetic `say` clips: 6 en, 6 vi, 3 mixed): Qwen3-ASR 1.7B WER 6.7%, 0.86 s per 10 s audio; Whisper large-v3-turbo WER 5.3%, 1.98 s per 10 s. Whisper was clearly better only on mixed vi/en clips (15% vs 32% WER) and wrote numbers as digits. Qwen kept per the gate; re-run with Ken's real clips before release (see Commands).
- Milestone 4 (first end-to-end dictation) done: coordinator, handy-keys shortcut + capture UI, dynamic Esc/Return, overlay, paste. Smoke-tested in TextEdit (Hold; Toggle with Return, Esc, second press).
- Milestone 5 (text pipeline) done: en/vi filler removal + stutter collapse, Unicode fuzzy custom words, replacements, History (10 newest) page, tray Copy Last Transcript, frontmost app context. Smoke-tested in TextEdit (en + vi).
- Milestone 6 (Clean and Reformat) done: Groq client, prompt, verbatim guard, cancel-aware request, "Cleaning…" overlay. Unit-tested plus ignored live tests (`GROQ_API_KEY=… cargo test groq_live -- --ignored --nocapture`); not yet exercised through the in-app key field.
- Milestone 7 (wrap-up) done: Secure Input detection with a tray warning, `THIRD_PARTY_NOTICES.md`, clippy clean.
- Post-v1 fixes (2026-09-17, after Ken's manual testing; Clean and Reformat casual/formal output confirmed good): Groq key is validated before saving with inline errors, saved key shown redacted, warning with instructions when enabling Clean and Reformat without a key; custom words no longer replace common words ("right" stayed "Wright" bug); Groq no longer translates words in mixed vi/en speech (prompt rule + `translated_mixed_words` guard).
- UI pass (2026-09-18): Clean and Reformat (toggle + Groq key field) moved from the Advanced page to its own group at the bottom of the General page, since it needs no hover info; the moved rows use short visible captions instead of InfoTips, and the no-key warning now renders below the group as its own fade-in line with a clickable `console.groq.com/keys` link (opens via the new `open_groq_keys_page` command, a fixed `/usr/bin/open` call with no frontend-supplied URL — no new Tauri plugin). The General shortcut button now renders each key as a keycap (⌥ ⌘ ⇧ ⌃ ␣ ↩ ⎋ ⇥ arrows, or uppercase text for letters/digits/F-keys, `_left`/`_right` suffixes handled) instead of words, with a readable `aria-label`; a small reset-to-default icon button sits to its right (disabled when already `option+space`). Vietnamese "Phím tắt chép lời" shortened to "Phím tắt" (the tray menu has no matching string).

### Not yet verified (run by hand)

- Fresh onboarding on a clean data dir, Wi-Fi off mid-download (Retry resumes), quit mid-download and relaunch, deleting the model after onboarding.
- General page Groq key UX after its move off Advanced: invalid/unreachable errors, redacted field, no-key warning (position/fade-in below the group, link opens the browser), Replace/Remove (visual pass + Vietnamese copy).
- Interface Language switch updating main window, overlay, and tray live; Vietnamese-locale first launch.
- Launch on Startup (needs a signed bundle, not `tauri dev`), Show Menu Bar Icon / Start Hidden interaction, `AUDIBLE_UNLOAD_SECS=30` unload/reload in the log.
- Visual pass of General (new Clean and Reformat group, keycap shortcut button, reset button), Dictionary, History, and Advanced pages (tooltips only on Advanced) and Vietnamese copy review by Ken.

### Known limitations

- Consecutive dictations are pasted with no separator (no automatic leading space).
- Custom words of 4–5 characters are only corrected on an exact case-insensitive match (e.g. "Tory" is not corrected to "Tauri"); the Groq step still sees them as preferred spellings.
- Custom words that sound like a common word ("Claude" heard as "cloud") or differ only in diacritics are left to Groq; uncommon English words outside the SCOWL list can still be replaced; ordinary word pairs can still join into a custom term ("all bright" → "Albright").
- Secure Input only warns; Handy's Carbon fallback re-registration is not ported.
- Groq may still normalize Vietnamese particles ("nha" → "nhé") despite the prompt; the translation guard only catches lost words, not swapped particles.
- "Audible" is an Amazon trademark: rename before public distribution.
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

# Model bake-off: folder of 16 kHz mono WAV clips, each with a same-named .txt reference
# (convert: afconvert -f WAVE -d LEI16@16000 -c 1 in.m4a out.wav). BENCH_LANG=vi|en forces a language.
cd src-tauri && cargo run --release --example bench -- <model.gguf> <clips-dir> [<model2.gguf> ...]
```

Dev builds compile `transcribe-cpp-sys`, `sha2`, `rubato`, `rustfft` at opt-level 3 (see `Cargo.toml`); unoptimized they make transcription and model verification far slower.

## Architecture

- `src-tauri/src/lib.rs`: Tauri setup, plugins, main window (created hidden, shown unless Start Hidden applies), close-to-hide with Accessory activation policy, specta command/event registration. Debug builds export `src/bindings.ts` at startup; `cargo test export_bindings` does the same without launching.
- `settings.rs`: one `AppSettings` struct (serde defaults, per-field salvage) in `settings_store.json`. `update_settings` persists and emits `settings-changed`. First-launch `app_language` comes from the macOS locale.
- `tray.rs`: menu bar icon + menu (Open, Copy Last Transcript, Cancel while busy, Quit, Secure Input warning). Updates coalesce into one main-thread apply. Icons are `include_bytes!` template PNGs from `src-tauri/resources/`.
- `tray_i18n.rs` + `build.rs`: tray strings generated from the `tray` section of `src/i18n/locales/{en,vi}/translation.json`, looked up by `app_language`.
- `commands.rs`: one setter command per setting; each returns the updated `AppSettings`. `set_groq_api_key` trims, validates via Groq `GET /models` (`cleanup::validate_api_key`), and saves only on 200; errors `empty_key` / `invalid_key` (401/403) / `unreachable`. The General page never receives the key: a saved key shows as a fixed disabled `gsk_••••` placeholder with Replace/Remove. `set_clean_and_reformat(true)` fails with `no_api_key` until a key exists; `clear_groq_api_key` also turns Clean and Reformat off. `open_groq_keys_page` shells out to `/usr/bin/open https://console.groq.com/keys` — a fixed URL, never one passed in from the frontend.
- `keychain.rs`: Groq key in the Keychain (service `com.kennnguyen.audible`, account `groq_api_key`) via `keyring`. `has_groq_api_key` reads attributes only (no Keychain prompt); the secret is read once per launch and cached in memory (reading it can prompt once per rebuild in unsigned dev builds).
- `audio/`: `recorder.rs` (cpal → wait-free ring → consumer thread: resample to 16 kHz, VAD filter, level visualizer), `vad.rs` (Silero + onset/prefill/hangover smoothing), `resampler.rs`, `visualizer.rs` (ported from Handy), `devices.rs`, `mute.rs` (AppleScript output mute, restores prior state), `mod.rs` `AudioManager` (opens the mic on start, closes on stop; `mic-level` events to the overlay at ~30 FPS; pads speech under 1 s).
- `model.rs`: the one `MODEL` constant (Qwen3-ASR 1.7B Q5_K_M from `handy-computer/Qwen3-ASR-1.7B-gguf` at a pinned revision, size + sha256), stored at `~/Library/Application Support/com.kennnguyen.audible/models/`. `ModelManager` runs the resumable download (`.partial` + Range, 60 s stall timeout, disk-space check, sha256 verify) and emits `model-download-progress` / `-failed{reason}` / `-complete`. Startup readiness checks size only.
- `transcription.rs`: `TranscriptionManager` (Arc state). Loads on demand, unloads after 600 s idle (`AUDIBLE_UNLOAD_SECS` in debug), `CancelToken` aborts a run. Language guard: detected language not en/vi → re-run forced `vi`, keep if whatlang reliably says Vietnamese, else re-run forced `en`. Custom words go to Whisper as `initial_prompt` only if the model arch is whisper.
- `coordinator.rs`: pure `CoordinatorState::on_input` (Idle → Recording{session, mode} → Processing{session}); mode is fixed at recording start; Esc cancels recording or processing; shortcut presses are ignored while processing; `ProcessingDone(session)` only ends its own session. `Coordinator` runs it on one thread and calls `actions::run_effect`.
- `actions.rs`: effects. Start checks `is_setup_complete` (else opens the window), captures context, opens the mic, preloads the model, arms Esc (and Return in Toggle), shows overlay + tray state. Finish disarms Return, stops the mic, then a pipeline thread transcribes → `pipeline::process` → disarms Esc → pastes → history. `ACTIVE_SESSION` guards paste/teardown so a cancelled session never touches a newer one.
- `shortcut.rs`: `ShortcutManager` owns the handy-keys `HotkeyManager` on its own thread (blocking tap swallows registered keys); falls back to tauri-plugin-global-shortcut if the tap can't start. Ids `transcribe`, `cancel` (escape), `finish` (return). Registration waits on that thread, so key events only forward to the coordinator. `start_capture`/`stop_capture` stream `shortcut-capture-event` to `components/ShortcutInput.tsx` and suspend the current shortcut meanwhile.
- `overlay.rs`: tauri-nspanel non-activating panel (label `recording_overlay`, 240×48) centred above the Dock on the cursor's screen; `show-overlay {state: recording|transcribing|cleaning, toggle}` / `hide-overlay` events to `src/overlay/Overlay.tsx`.
- `paste.rs`: on the main thread: save clipboard (text, else image) → write text → Cmd + layout-aware V keycode via enigo → restore. `cursor_location()` shares the enigo instance.
- `text/filler.rs`: English (um, uh, uhm, umm, er, ah, hmm) and Vietnamese (ờ, ờm, ừm, ưm, ơ, hừm) fillers, always both lists; "à"/"ừ" are kept. Also collapses a word repeated 3+ times and tidies punctuation/capitalization.
- `text/dictionary.rs`: `apply_custom_words` (NFC, char Levenshtein ≤ 18% of the longer key, 1–3-word n-grams that never cross punctuation and must use every word, keys ≥ 4 chars; words of 4–5 chars therefore only match exactly; single words in `text/common_words_en.txt` (SCOWL levels 10–20, 4+ letters) are never replaced or recased; words with diacritics only match exactly ignoring case) and `apply_replacements` (one leftmost-first regex, longest trigger first, `\b` only on word-character edges, `(?i)` unless Clean and Reformat is on; returns the inserted values).
- `pipeline.rs`: `process_local` (NFC → fillers if on → custom words → replacements) then Clean and Reformat (Milestone 6). Case sensitivity follows the Clean and Reformat setting even when Groq later fails.
- `cleanup.rs`: Groq `openai/gpt-oss-120b`, `reasoning_effort: "low"`, `include_reasoning: false`, 10 s timeout. User message is JSON `{app, bundle_id, window_title, language, dictionary, keep_verbatim, transcript}`. `sanitize_response` strips `<think>`, wrapping quotes, whitespace. The prompt applies dictionary spellings only where the speaker means that term. `choose_output` keeps the local text if Groq failed, returned nothing, or lost/re-cased any `keep_verbatim` value (the replacement values inserted locally), or translated mixed-language words (`translated_mixed_words`: Groq dropped a word of one language and added a word of the other, e.g. "support" → "hỗ trợ"; one such word rejects the output). The prompt's mixed-language section comes first and overrides the formatting rules; context changes formatting, not vocabulary. `groq_live_mixed_language_not_translated` (`RUNS=n`) replays Ken's real mixed transcripts. `pipeline.rs` races the request against the session being cancelled.
- `context.rs`: at recording start, frontmost app name + bundle id (NSWorkspace) and focused window title (AX `AXFocusedWindow` → `AXTitle`, truncated to 120 chars).
- `history.rs`: `history.json` store, newest first, capped at 10 (`push_capped`); entries hold pasted text, raw transcript, app name, timestamp (ms, also the id). Emits `history-changed`; tray Copy Last Transcript copies the newest.
- `secure_input.rs`: polls `IsSecureEventInputEnabled()` every second (only while the handy-keys tap is the backend); held ≥ 3 s (`SustainTracker`) shows the tray warning item, released clears it.
- `permissions.rs`: sync mic/Accessibility checks. `lib.rs` `on_ready` registers the shortcut once onboarding is done and Accessibility is granted (a missing model doesn't block it: the press opens the download screen). `is_setup_complete` (onboarding + permissions + model) decides whether launch shows the window and whether a press records.
- `autostart.rs`: Launch on Startup via `SMAppService` (fails harmlessly in `tauri dev`).
- Frontend stores: `src/stores/settings.ts`, `src/stores/history.ts`.
- Frontend: `src/main.tsx` → `App.tsx` (shows `components/Onboarding.tsx` for new users, or for returning users at the first missing permission/model; else sidebar + pages in `src/pages/`), `src/stores/settings.ts` (zustand mirror; `applySettings(commands.setX(..))` stores the returned settings; `settings-changed` keeps it live), `src/components/ui/` (Group/Row, Toggle, Segmented, Select, TextField/Button, InfoTip), `src/components/ShortcutInput.tsx` (`formatShortcut` returns the readable name for aria-labels/errors; the button itself renders keycap spans per key — symbols for modifiers/space/return/escape/tab/arrows, uppercase text otherwise, `_left`/`_right` suffixes stripped — plus an exported `ResetShortcutButton` that calls `change_shortcut` with the hardcoded default, kept in sync with `settings::DEFAULT_SHORTCUT`), `src/i18n/` (react-i18next, language follows the stored setting and `settings-changed`), `src/overlay/` (recording overlay webview entry), `src/index.css` (color tokens, light/dark, `.animate-fade-in` keyframe for transient warnings).

## UI rules

- Minimal, neutral design; it will be redesigned later.
- No hover info (ⓘ, `title=`) anywhere except the Advanced page.
- No model picker, model options, or unload-timeout setting.
- Every user-visible string goes in both `en` and `vi` locale files.

## Reference skeleton: `ref/Handy`

Read-only, never edit. Handy (MIT) is the architectural reference Audible ports selected modules from (audio/VAD, transcription manager, handy-keys shortcuts, overlay NSPanel, paste, tray). Ported code keeps Handy's MIT notice in `THIRD_PARTY_NOTICES.md`. Full detail: `ref/Handy/AGENTS.md`.

## Skill: `.claude/skills/run`

Launches `bun run tauri dev` in the background and waits for `Shortcut 'option+space' ready` in the log. It also documents the automated dictation smoke test (synthetic keys + `say` into TextEdit; ask Ken first) and how to screenshot only Audible's windows.
