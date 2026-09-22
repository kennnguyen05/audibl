# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
It is context for future work, not a changelog — git history records what changed and when.

## What this project is

**Audibl**: a macOS-only voice-to-text dictation app, cloning Wispr Flow. Press a shortcut, speak English or Vietnamese, and the text is pasted into the focused app. v1 scope:

- Transcribe Shortcut with Auto / Hold / Toggle modes (Auto is the default: a press under 300 ms is a tap that locks recording on, a longer one is hold-to-talk; a locked session is pasted by the next tap, Esc cancels)
- Local model (Qwen3-ASR 1.7B via `transcribe-cpp`, Metal), auto language detect, no model options in the UI
- Dictionary (custom words + replacements), local filler removal, History (last 10)
- Optional Groq "Magic Touch" step, formerly "Clean and Reformat" (key in the macOS Keychain)
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

## Release build

```bash
bun run release   # tauri build --target aarch64-apple-darwin (~3 min clean)
```

Output in `src-tauri/target/aarch64-apple-darwin/release/bundle/`: `macos/Audibl.app` (~30 MB) and `dmg/Audibl_1.0.0_aarch64.dmg` (~13 MB). The 1.4 GB model is not bundled; onboarding downloads it. Apple Silicon only; no auto-updater.

- **Signing is ad-hoc** (`signingIdentity: "-"`), hardened runtime on, entitlements from `Entitlements.plist`. Not notarized, so a downloaded DMG hits Gatekeeper; `INSTALL.md` has the "Open Anyway" / `xattr -dr com.apple.quarantine` steps for testers.
- Every rebuild changes the ad-hoc signature, so macOS re-asks for Microphone and Accessibility each time. The release app and the dev build are separate TCC entries but share app data, logs and the Keychain entry (same identifier) and the single-instance lock — quit one before launching the other.
- **To ship signed + notarized** (needs the Apple Developer Program): install a "Developer ID Application" certificate, then build with `APPLE_SIGNING_IDENTITY="Developer ID Application: … (TEAMID)"` plus either `APPLE_ID`/`APPLE_PASSWORD` (app-specific password)/`APPLE_TEAM_ID` or `APPLE_API_KEY`/`APPLE_API_ISSUER`/`APPLE_API_KEY_PATH`. The env var overrides `signingIdentity` in `tauri.conf.json`; Tauri notarizes and staples automatically.
- Bump the version in `tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json` together; the DMG name follows it.

Dev builds compile `transcribe-cpp-sys`, `sha2`, `rubato` at opt-level 3 (see `Cargo.toml`); unoptimized they make transcription and model verification far slower.

## Architecture

- `src-tauri/src/lib.rs`: Tauri setup, plugins, main window (created hidden, shown unless Start Hidden applies), close-to-hide with Accessory activation policy, specta command/event registration. Debug builds export `src/bindings.ts` at startup; `cargo test export_bindings` does the same without launching.
- `settings.rs`: one `AppSettings` struct (serde defaults, per-field salvage) in `settings_store.json`. `update_settings` persists and emits `settings-changed`. First-launch `app_language` comes from the macOS locale.
- `tray.rs`: menu bar icon + menu (Open, Copy Last Transcript, Cancel while busy, Quit, Secure Input warning). Updates coalesce into one main-thread apply. Icons are `include_bytes!` template PNGs from `src-tauri/resources/`.
- `tray_i18n.rs` + `build.rs`: tray strings generated from the `tray` section of `src/i18n/locales/{en,vi}/translation.json`, looked up by `app_language`.
- `commands.rs`: one setter command per setting; each returns the updated `AppSettings`. `set_groq_api_key` trims, validates via Groq `GET /models` (`cleanup::validate_api_key`), and saves only on 200; errors `empty_key` / `invalid_key` (401/403) / `unreachable`. The General page never receives the key: a saved key shows as a fixed disabled `gsk_••••` placeholder with Replace/Remove. `set_clean_and_reformat(true)` fails with `no_api_key` until a key exists; `clear_groq_api_key` also turns Clean and Reformat off. `quit_app` cancels any download and exits (onboarding's way out). `open_groq_keys_page` shells out to `/usr/bin/open https://console.groq.com/keys` — a fixed URL, never one passed in from the frontend.
- `keychain.rs`: Groq key in the Keychain (service `com.kennnguyen.audibl`, account `groq_api_key`) via `keyring`. `has_groq_api_key` reads attributes only (no Keychain prompt); the secret is read once per launch and cached in memory (reading it can prompt once per rebuild in unsigned dev builds).
- `audio/`: `recorder.rs` (cpal → wait-free ring → consumer thread: resample to 16 kHz, VAD filter, level meter), `vad.rs` (Silero + onset/prefill/hangover smoothing), `resampler.rs`, `level.rs` (RMS per 1/30 s window, mapped to 0..1 above an adaptive noise floor — one amplitude, not a spectrum), `devices.rs`, `mute.rs` (AppleScript output mute, restores prior state), `mod.rs` `AudioManager` (opens the mic on start, closes on stop; `mic-level` events (one `f32`) to the overlay at ~30 FPS; pads speech under 1 s).
- `model.rs`: the one `MODEL` constant (Qwen3-ASR 1.7B Q5_K_M from `handy-computer/Qwen3-ASR-1.7B-gguf` at a pinned revision, size + sha256), stored at `~/Library/Application Support/com.kennnguyen.audibl/models/`. `ModelManager` runs the resumable download (`.partial` + Range, 60 s stall timeout, disk-space check, sha256 verify) and emits `model-download-progress` / `-failed{reason}` / `-complete`. Startup readiness checks size only.
- `transcription.rs`: `TranscriptionManager` (Arc state). Loads on demand, unloads after 600 s idle (`AUDIBLE_UNLOAD_SECS` in debug), `CancelToken` aborts a run. Language guard: detected language not en/vi → re-run forced `vi`, keep if whatlang reliably says Vietnamese, else re-run forced `en`. Custom words go to Whisper as `initial_prompt` only if the model arch is whisper.
- `coordinator.rs`: pure `CoordinatorState::on_input` (Idle → Recording{session, mode} → Processing{session}); mode is fixed at recording start; Esc cancels recording or processing; shortcut presses are ignored while processing; `ProcessingDone(session)` only ends its own session. `Coordinator` runs it on one thread and calls `actions::run_effect`. A `hold: Option<Hold>` field (`pressed_at` + `locked`, ported from Handy) sits beside `stage` and is `Some` exactly while recording: `locked` is set at once in Toggle, in Auto once a release under `HOLD_THRESHOLD` (300 ms) is classified as a tap, and never in Hold. A locked session is finished by the next press; Handy's X11 release-grace timer is deliberately not ported, since handy-keys collapses a held key into one press and one release.
- `actions.rs`: effects. Start checks `is_setup_complete` (else opens the window), captures context, opens the mic, preloads the model, arms Esc, shows overlay + tray state. Finish stops the mic, then a pipeline thread transcribes → `pipeline::process` → disarms Esc → pastes → history. `ACTIVE_SESSION` guards paste/teardown so a cancelled session never touches a newer one.
- `shortcut.rs`: `ShortcutManager` owns the handy-keys `HotkeyManager` on its own thread (blocking tap swallows registered keys); falls back to tauri-plugin-global-shortcut if the tap can't start. Ids `transcribe` and `cancel` (escape); Return is never registered, so it always reaches the focused app. Registration waits on that thread, so key events only forward to the coordinator. `start_capture`/`stop_capture` stream `shortcut-capture-event` to `components/ShortcutInput.tsx` and suspend the current shortcut meanwhile.
- `overlay.rs`: tauri-nspanel non-activating panel (label `recording_overlay`, 300×56; the pill holds the record dot, the waveform and, once busy, a status word — no key hints) centred above the Dock on the cursor's screen. Nothing in it is clickable, so `ignore_cursor` calls `set_ignore_cursor_events(true)` at create and again on every `show`: the panel is wider than the pill and would otherwise swallow clicks near the bottom of the screen. `show-overlay {state: recording|transcribing|cleaning}` / `hide-overlay` events to `src/overlay/Overlay.tsx`.
- `paste.rs`: on the main thread: save clipboard (text, else image) → write text → Cmd + layout-aware V keycode via enigo → restore. `cursor_location()` shares the enigo instance.
- `text/filler.rs`: English (um, uh, uhm, umm, er, ah, hmm) and Vietnamese (ờ, ờm, ừm, ưm, ơ, hừm) fillers, always both lists; "à"/"ừ" are kept. Also collapses a word repeated 3+ times and tidies punctuation/capitalization.
- `text/dictionary.rs`: `apply_custom_words` matches each custom word in two forms. **Written** (NFC, char Levenshtein ≤ 18% of the longer key, 1–3-word n-grams that never cross punctuation and must use every word, keys ≥ 4 chars; words of 4–5 chars therefore only match exactly; single words in `text/common_words_en.txt` (SCOWL levels 10–20, 4+ letters) are never replaced or recased; words with diacritics only match exactly ignoring case). **Spoken**, for a word holding one of `SPOKEN_SYMBOLS` (`. @ / - _ + #`): the symbol becomes its dictated word, so "CLAUDE.md" is also `["claude", "dot", "md"]`. A spoken form only matches an n-gram of exactly its own length (up to `MAX_SPOKEN_NGRAM` 6) with the symbol words present verbatim; that anchor buys a looser `SPOKEN_MAX_DISTANCE_RATIO` (0.34) on the words around it, which is what lets "cloud dot md" reach "CLAUDE.md" while bare "cloud" stays a cloud. Two limits keep the anchor from rewriting ordinary speech: a word shorter than `SPOKEN_LOOSE_MIN_CHARS` (6) must match exactly, since at that length the loose ratio swaps a whole syllable ("ben at sample dot com" is not Ken's address), and a form whose every non-symbol word is a common English word is never registered at all ("@home" is said "at home"). `apply_replacements` (one leftmost-first regex, longest trigger first, `\b` only on word-character edges, always `(?i)` — capitalization is the recognizer's guess, never an instruction; returns the inserted values). `apply_dictionary_spelling` rewrites whole-phrase case-insensitive occurrences of a custom word to the spelling the user typed, skipping a single common English word so "apple" survives custom "Apple"; it also takes the replacement values already inserted and matches them first, so a custom word can never re-case the inside of a value the user typed ("audibl.app" stays lowercase under custom "Audibl").
- `pipeline.rs`: `process_local` (NFC → fillers if on → custom words → replacements) then Magic Touch (Milestone 6), then `apply_dictionary_spelling` over whichever text won — so a custom word reaches the user spelled the way it was typed whether or not Groq ran.
- `cleanup.rs`: Groq `openai/gpt-oss-120b`, `reasoning_effort: "low"`, `include_reasoning: false`, 10 s timeout. User message is JSON `{app, bundle_id, window_title, language, dictionary, keep_verbatim, transcript}`. `sanitize_response` strips `<think>`, wrapping quotes, whitespace. The prompt applies dictionary spellings only where the speaker means that term, character for character, and says a term's punctuation is dictated as a word ("cloud dot md" → "CLAUDE.md"); `pipeline.rs` then enforces the spelling with `dictionary::apply_dictionary_spelling`, so the model's casing is never the last word. `choose_output` keeps the local text if Groq failed, returned nothing, or lost/re-cased any `keep_verbatim` value (the replacement values inserted locally), or translated mixed-language words (`translated_mixed_words`: Groq dropped a word of one language and added a word of the other, e.g. "support" → "hỗ trợ"; one such word rejects the output). The prompt's mixed-language section comes first and overrides the formatting rules; context changes formatting, not vocabulary. `groq_live_mixed_language_not_translated` (`RUNS=n`) replays Ken's real mixed transcripts. `pipeline.rs` races the request against the session being cancelled.
- `context.rs`: at recording start, frontmost app name + bundle id (NSWorkspace) and focused window title (AX `AXFocusedWindow` → `AXTitle`, truncated to 120 chars).
- `history.rs`: `history.json` store, newest first, capped at 10 (`push_capped`); entries hold pasted text, raw transcript, app name, timestamp (ms, also the id). Emits `history-changed`; tray Copy Last Transcript copies the newest.
- `secure_input.rs`: polls `IsSecureEventInputEnabled()` every second (only while the handy-keys tap is the backend); held ≥ 3 s (`SustainTracker`) shows the tray warning item, released clears it.
- `permissions.rs`: sync mic/Accessibility checks. `lib.rs` `on_ready` registers the shortcut once onboarding is done and Accessibility is granted (a missing model doesn't block it: the press opens the download screen). `is_setup_complete` (onboarding + permissions + model) decides whether launch shows the window and whether a press records.
- `sfx.rs`: On/Off chimes (`NSSound`, mp3s embedded from `resources/sfx/`), played by `actions.rs` at recording start and stop/cancel.
- `autostart.rs`: Launch on Startup via `SMAppService` (fails harmlessly in `tauri dev`).
- `src/components/ShaderBackground.tsx` + `src/shaders/plasma.ts`: the animated WebGL background, a "Plasma" field made with the 21st.dev Shader Builder on the scaffold Audibl's earlier Metaballs background took from Paper Shaders (Apache-2.0, `THIRD_PARTY_NOTICES.md`). Plain WebGL1, no dependency, one fullscreen triangle. **The GLSL is generated upstream code and is kept verbatim — retune the look through the uniform tables at the top of `ShaderBackground.tsx`, never by editing the shader.** Softness is the shader's own 5-tap blur (`u_finish.z`), so there is no compositor `filter: blur()` and no canvas bleed: the canvas is exactly the viewport and the drawing buffer is CSS pixels times `devicePixelRatio` capped at 2. `App.tsx` mounts it (plus the `.shader-veil`) above both the onboarding and shell branches, so the background is continuous across the outro handoff; the recording overlay is deliberately excluded. The RAF loop pauses on `visibilitychange` — the main window hides to the tray rather than closing, so an unpaused loop would burn the GPU indefinitely.
- Frontend stores: `src/stores/settings.ts`, `src/stores/history.ts`.
- Frontend: `src/main.tsx` → `App.tsx` (shows `components/Onboarding.tsx` for new users, or for returning users at the first missing permission/model; else sidebar + pages in `src/pages/`), `src/stores/settings.ts` (zustand mirror; `applySettings(commands.setX(..))` stores the returned settings; `settings-changed` keeps it live), `src/components/ui/` (Group/Row/EmptyRow/PageTitle/PageHeader, Button/IconButton, Toggle, Segmented, Select, TextField, InfoTip, ConfirmDialog — the app's one modal, used for Dictionary deletes — and `icons.tsx`), `src/components/ShortcutInput.tsx` (`formatShortcut` returns the readable name for aria-labels/errors; the button itself renders keycap spans per key — symbols for modifiers/space/return/escape/tab/arrows, uppercase text otherwise, `_left`/`_right` suffixes stripped — plus an exported `ResetShortcutButton` that calls `change_shortcut` with the hardcoded default, kept in sync with `settings::DEFAULT_SHORTCUT`), `src/i18n/` (react-i18next, language follows the stored setting and `settings-changed`), `src/overlay/` (recording overlay webview entry), `src/index.css` (the whole design system: one dark palette, fonts, radii and a px-pinned type scale in `@theme`; element resets in `@layer base`; global focus-visible ring; `.animate-fade-in` and `sweep` keyframes; `prefers-reduced-motion`; global scrollbar hiding — scroll stays functional, just no visible track/thumb).

## UI rules

The design system is Wispr Flow's visual language in a dark-only palette. All
tokens live in `src/index.css`; nothing else defines a color, radius or size.

- **Dark only.** One palette, no `prefers-color-scheme` branch. `lib.rs` pins
  `NSAppearanceNameDarkAqua` so native chrome (traffic lights, `<select>`
  popups) stays dark whatever macOS is set to.
- **Colors:** `sidebar` `#0e0d0c` → `bg` `#161412` → `surface` `#1e1b18` →
  `control` `#272320`, hairline `border` `#2b2723`, text `#f5f1ea`, muted
  `#9a9186`. `accent` `#f5f1ea` marks one thing per screen: what is focused,
  what is on, or what is listening. It is a brightness highlight, not a hue —
  the same off-white as `text`, one step of light above the surface ladder.
  Saturated color is rationed and each hue has one job: `warning` `#e8b33c`
  (amber, the missing-Groq-key line on General — a caution, not an error),
  `danger` `#e8705a` (error text and the destructive button variant),
  `success` `#7fbf7a` (a granted permission, a copied line) and `record`
  `#e5484d`, used by nothing but the overlay's recording light, which has to
  read as red over any window.
- **The interface is glass over a live shader.** Depth is no longer an opaque
  surface ladder: every container is a translucent pane over the animated
  background (see `ShaderBackground` in Architecture), and the palette tokens
  above are now what those panes are *mixed from*, not what they paint. The
  recipe lives in exactly one `@layer components` block in `src/index.css`;
  nothing else may mix its own glass:
  - `.glass-rail` — the sidebar, the densest pane, it anchors the window
  - `.glass-card` — `Group`'s card and the onboarding step cards
  - `.glass-float` — things that genuinely float: `InfoTip`, `ConfirmDialog`
  - `.glass-control` — inline controls (Button, Select, Toggle, ShortcutInput)
  - `.glass-well` — an inset well: the Segmented track, Dictionary's form row
  - `.shader-veil` — the single dim layer over the canvas, and the one
    brightness/legibility knob for the whole app; tune it before touching
    anything else. It trades off against text that sits on the bare
    background — the page titles and section headings, which have no card
    under them — not against the cards, which darken their own backdrop.
- **Blur belongs to containers, never to the controls inside them.** A
  `backdrop-filter` element nested in another one samples only its ancestor's
  already-blurred result, so a second blur buys nothing and costs a GPU pass
  per control. `.glass-control` and `.glass-well` are translucent-only by
  design — they let their card's glass show through.
- **Borders on glass use `--color-hairline`** (`rgb(245 241 234 / 0.08)`), not
  the opaque `--color-border`, which reads as a drawn line rather than an edge
  catching light. `--color-border` survives only where a surface is still
  opaque.
- **Shadow still only for what floats** — the recording overlay, `InfoTip`,
  `ConfirmDialog`. Glass gets its depth from the blur, not from a shadow.
- **The page title is neon.** `.title-glow` in `index.css` lights `PageTitle`,
  `PageHeader` and the onboarding outro's slogan like a tube: a white core (`--color-title-core`) under
  shadows that widen and cool into `--color-title-halo`, the one cool cast in
  the warm palette. Both tokens exist for this class alone. The black shadows
  after them are the other half and are not optional — a halo only reads as
  light against something darker than its faintest ring, and without them the
  glow disappears wherever a bright band of the field drifts behind the title. They come
  last because the first shadow listed paints on top, so they land beneath the
  glow; being shadows of the same glyphs, the dark field is cut to the
  letterforms rather than a plate the text sits on. Each black blur is
  repeated because text-shadow has no spread, and one pass that wide is far
  too faint to hide anything. Every radius is in em so the tube stays the same
  thickness relative to the letterform at both the 30px page titles and the
  56px slogan. Nothing here costs layout. The onboarding headline is
  deliberately *not* lit — the glow is for the titles and the slogan only.

- **Three radii:** `rounded-card` (14px) for containers, `rounded-control` (8px)
  for controls, `rounded-chip` (6px) for a pill inside a control, `rounded-full`
  for toggles.
- **One typeface.** Newsreader sets the whole app, titles and controls alike;
  `--font-sans` and `--font-serif` both resolve to it, so no rule can bring a
  second family back. Self-hosted via `@fontsource-variable/newsreader`, so the
  app renders offline, and it carries a Vietnamese subset. Sizes and
  `--spacing` are pinned in px so the root font-size can never resize the
  layout.
- **Element resets must go in `@layer base`.** Unlayered element rules outrank
  every Tailwind utility — a bare `button { color: inherit }` silently beats
  `text-sidebar` and paints the primary button cream on cream.
- Captions (`Row`'s `caption` prop), not tooltips. Hover info (`InfoTip`)
  survives on the Advanced page only. Its `glyph="?"` variant has no caller
  left but is kept for reuse. A caption that needs a longer explanation ends
  in an inline link instead — see General's "Show me how".
- No template chrome: no tracked-out ALL-CAPS eyebrows, no `A · B · C` meta
  strings, no `→` appended to button text, no `font-mono` for non-code.
- **Motion:** the shader background is the app's ambient motion. Everything
  else answers an action — `animate-page-enter` on a tab change (the content
  column is keyed on the page, so switching remounts it and replays the
  animation), the sidebar's sliding pill, `animate-dialog-enter`, the overlay's
  sweep. Still no decorative section entrances. `prefers-reduced-motion` is
  honoured globally by the `@media` block in `index.css`, but that block only
  neuters *CSS* animation — anything driven from JS (the shader) must check
  `matchMedia` itself.
- No model picker, model options, or unload-timeout setting.
- Every user-visible string goes in both `en` and `vi` locale files.

## Reference skeleton: `ref/Handy`

Read-only, never edit. Handy (MIT) is the architectural reference Audibl ports selected modules from (audio/VAD, transcription manager, handy-keys shortcuts, overlay NSPanel, paste, tray). Ported code keeps Handy's MIT notice in `THIRD_PARTY_NOTICES.md`. Full detail: `ref/Handy/AGENTS.md`.

## Skill: `.claude/skills/run`

Launches `bun run tauri dev` in the background and waits for `Shortcut 'option+space' ready` in the log. It also documents the automated dictation smoke test (synthetic keys + `say` into TextEdit; ask Ken first) and how to screenshot only Audibl's windows.

## Skill: `.claude/skills/onboarding-test`

Deletes `settings_store.json` only (keeps `models/`, so the model step auto-passes instead of re-downloading) to reset `onboarding_complete`, then launches the dev build. Mic/Accessibility steps read live macOS TCC state, not the settings file, so they auto-pass if already granted — ask Ken before revoking those to test the permission screens themselves. Documents the full fresh-install path (`rm -rf` the whole app-data dir) for testing the model download too.

## Skill: `.claude/skills/full-onboarding-test`

Heavier sibling of `onboarding-test`: `rm -rf` the whole `~/Library/Application Support/com.kennnguyen.audibl` dir (settings, history, and the downloaded model together) for a true start-to-finish run including the model re-download, then launches the dev build. Also documents deleting the Keychain `groq_api_key` entry and `tccutil reset Microphone|Accessibility` for exercising the no-key and permission-prompt states — both ask-Ken-first since they're broader than the app itself. Confirm with Ken before running: it discards the local model and all settings.

## Gotchas

- **Naming.** The product is **Audibl** (renamed off "Audible" for the Amazon trademark); the bundle identifier is `com.kennnguyen.audibl`, used for app data, logs and the Keychain service. The Rust crate, the binary (`target/debug/audible`) and the `AUDIBLE_*` dev env vars keep the old spelling — internal only. Old app data at `~/Library/Application Support/com.kennnguyen.audible` is orphaned and can be deleted.
- **Model choice is settled.** Bake-off (15 synthetic `say` clips): Qwen3-ASR 1.7B WER 6.7% at 0.86 s per 10 s audio; Whisper large-v3-turbo WER 5.3% at 1.98 s. Whisper only clearly won on mixed vi/en (15% vs 32%). Qwen kept for speed. Re-run with real clips (`cargo run --release --example bench`) before changing models.
- **Internal names are frozen for compatibility.** The UI says "Magic Touch", but the setting (`clean_and_reformat`), the command (`set_clean_and_reformat`), the i18n keys (`general.cleanAndReformat…`) and the overlay state (`cleaning`) keep the old "Clean and Reformat" names so saved settings survive.
- **Two duplicated tables must change together:** the brand mark bars in `src/components/ui/icons.tsx` (`AudiblMark`) and `scripts/generate-icons.ts` (`MARK_BARS`); and the default shortcut in `settings::DEFAULT_SHORTCUT` and `ShortcutInput`'s `ResetShortcutButton`.
- **Ad-hoc signing means TCC resets.** Every rebuild changes the signature, so macOS re-asks for Microphone and Accessibility. When Accessibility is granted while the app is already open, `ensure_shortcut` (called from `App.tsx`'s focus check) is what starts the shortcut manager — without it `option+space` silently does nothing.
- **Dev and release builds share app data, logs, the Keychain entry and the single-instance lock** (same identifier). Quit one before launching the other.
- **New Tauri permissions are not automatic.** `capabilities/default.json` needs them explicitly — e.g. window dragging needed `core:window:allow-start-dragging`, which is not in `core:default`.

## Known limitations

- Consecutive dictations are pasted with no separator (no automatic leading space).
- Custom words of 4–5 characters are only corrected on an exact case-insensitive match (e.g. "Tory" is not corrected to "Tauri"); the Groq step still sees them as preferred spellings.
- Custom words that sound like a common word ("Claude" heard as "cloud") or differ only in diacritics are left to Groq — unless the word is dictated with its punctuation ("cloud dot md"), which the spoken form catches; uncommon English words outside the SCOWL list can still be replaced; ordinary word pairs can still join into a custom term ("all bright" → "Albright").
- A custom word spelled entirely out of common English words plus a symbol ("@home", "work-life") is never matched from speech, only written out — "at home" in a sentence has to stay "at home". Groq still sees it as a preferred spelling.
- Replacement triggers ignore case but not punctuation or spacing: with the trigger "my IG", "my I.G." and "my I G" are still misses.
- Secure Input only warns; Handy's Carbon fallback re-registration is not ported.
- Groq may still normalize Vietnamese particles ("nha" → "nhé") despite the prompt; the translation guard only catches lost words, not swapped particles.
- No streaming transcription: Qwen3-ASR has no streaming mode, so the overlay can never show live text.

## Not yet verified (run by hand)

- Launch on Startup on a real bundle (`SMAppService` may refuse an ad-hoc app), and a first launch on a second Mac from the DMG.
- Fresh onboarding on a clean data dir: Wi-Fi off mid-download (Retry resumes), quit mid-download and relaunch, deleting the model after onboarding.
- Groq key UX on General: invalid/unreachable errors, redacted field, no-key warning, Replace/Remove — visual pass plus Vietnamese copy.
- Interface Language switch updating main window, overlay and tray live; Vietnamese-locale first launch.
- Show Menu Bar Icon / Start Hidden interaction.
- Vietnamese copy review by Ken.
- The tray icon in a light menu bar; the app icon in the Dock and Finder.
- The On/Off chimes by ear, with Mute While Recording on and off, and whether the On chime leaks into the transcript.
- The overlay waveform against real speech (the adaptive floor in `audio/level.rs` is tested on synthetic sines only) and its cleaning state.
- Screens never seen on screen: the overlay's cleaning state, the model download's downloading/verifying/failed views, the Groq key's saved/error states, Dictionary's open add form, History's copied and expanded states.
