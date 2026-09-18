# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Audibl v1 was built milestone by milestone (one commit each) from the plan at
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
- UI fix (2026-09-18): the Clean and Reformat group's captions under "Clean and Reformat" and "Groq API Key" made those two-line rows visually misaligned against the rest of the General page's single-line rows. Replaced both captions with `InfoTip` hover icons next to the row labels (same pattern as Advanced), keeping the caption text as the tooltip. `InfoTip` is now used on General too, not just Advanced — see its updated comment in `src/components/ui/InfoTip.tsx`.
- UI simplification (2026-09-18): General's three section headers (Shortcut / Sound / Clean and Reformat) removed; all rows now sit in one untitled `Group`. `Group`'s `title` prop is optional (`src/components/ui/Row.tsx`) — the `<h2>` only renders when passed, so Dictionary and Advanced keep their titled groups unchanged. The now-unused `general.shortcutGroup`/`soundGroup`/`cleanGroup` locale keys were deleted from both `en` and `vi`.

- Onboarding exit (2026-09-18): the model step has a secondary "Quit Audibl" button next to Finish, for users who don't want the download. It calls the new `quit_app` command, which cancels an in-progress download (keeping the resumable `.partial`) and then `app.exit(0)`.
- Onboarding quit button (2026-09-18): "Quit Audibl" switched from `variant="ghost"` to the existing `Button` `danger` variant (`text-danger`, same red as Groq key errors) and moved from a `justify-between` layout (pinned to the far left) to `justify-end gap-3` right next to Finish.
- UI copy (2026-09-18): English label for `general.transcribeShortcut` shortened from "Transcribe Shortcut" to "Shortcut", matching vi's "Phím tắt".
- UI change (2026-09-18): the Mode row on General is now a full-width two-button selector instead of a label + small segmented control. `Segmented` gained `size="large"` and an optional `description` per option: large renders full-width buttons of fixed height, the unselected label at `text-base`, and the selected one shrinking to `text-sm` with its description underneath. The small variant (Advanced's Interface Language) is unchanged. New locale key `general.holdCaption`; `general.toggleCaption` now reads "Press to activate; Return pastes, Esc cancels" and is shown inside the button, not as a `Row` caption.
- UI tweak (2026-09-18): shortcut button keycaps (`src/components/ShortcutInput.tsx`) no longer render as bordered boxes; symbols now sit flush together (⌥⌘⇧␣) like macOS menu shortcut hints. Multi-character keys (letters, F-keys) keep a small `mx-0.5` gap so they don't run into neighbors; single-glyph symbol keys get none.
- UI tweak (2026-09-18): `Select` (`src/components/ui/Select.tsx`, used by General's microphone/mode pickers) now `appearance-none` with a custom chevron — the native control ignored height/font classes on its own (macOS WebKit gives `<select>` its own min-height/padding), so the OS chrome had to be stripped and rebuilt manually. Sized `h-9`/`text-base` so its option text matches the `Row` label's font size, not the shortcut button's smaller `h-7`.
- UI fix (2026-09-18): `Select`'s box no longer sits in a fixed `max-w-60` slot for every option (short picks like "System Default" had a lot of empty space; long device names truncated). Width is now `clamp(6rem, <selected label length>ch + 2.75rem, 15rem)`, recomputed from the currently selected option's own label, so the box hugs short values and only expands toward the old 240px ceiling for long ones.

- Full UI redesign (2026-09-18), from the plan at `~/.claude/plans/lively-greeting-wall.md`: Wispr Flow's visual language (reference screenshots in `ref/other assets/`) in a dark-only palette. See "UI rules" for the system itself.
  - Foundation: `src/index.css` rewritten (light tokens deleted); Newsreader + Plus Jakarta Sans self-hosted via `@fontsource-variable/*`; all `src/components/ui/` primitives rewritten, with new `Button`/`IconButton` (primary is a near-white fill — the dark inversion of the reference's black pill) and a hand-drawn `icons.tsx`.
  - Shell: window 900×640 / min 780×560, `TitleBarStyle::Overlay` + `hidden_title` + dark `background_color`, and `force_dark_appearance()` in `lib.rs`. The sidebar runs to the top edge with `pt-11` for the traffic lights, carries `data-tauri-drag-region`, nav icons, an amber current-page marker and a version footer; content is a centred 620px column.
  - General: two cards (dictation, then a titled Clean and Reformat) instead of eight rows in one; both raw `<div>` escapes are now `Row stacked`; InfoTips became captions; the `mt-[-16px]` warning hack is gone. Advanced split into "App" and "Text and language" and keeps its InfoTips. `ShortcutInput` reuses the shared `IconButton`/`ResetIcon`.
  - Dictionary: add controls moved into each `Group`'s `action` slot (a secondary button revealing an inline add row); custom words are list rows, not chips; hover-revealed delete; both cards have empty states. History uses `Group`, an app/time meta line (time later removed), hover-revealed `IconButton`s with a `CheckIcon` copied state, and click-to-expand clamping past 260 chars.
  - Onboarding: one screen with a bold sans headline and three stacked step cards (current expanded, done collapsed with a `text-success` check, later dimmed); steps auto-advance on their poll and only Finish is explicit. `onboarding.stepOf`/`continue` deleted, `onboarding.headline`/`model.ready` added.
  - Overlay: panel 300×64, pill 272×44 on tokens with a real shadow and a `ring-text/10` hairline. One 14-bar visualizer morphs across all three states — amber live bars, then a staggered settle to a muted resting row under a masked amber sweep (slower for cleaning). New `overlay.listening` key. Debug-only `AUDIBLE_DEV_OVERLAY=recording|transcribing|cleaning` shows a state at startup for screenshots, with a synthetic waveform via `ShowPayload.preview`.
  - Icons: `scripts/generate-icons.ts` now draws the real mark (a waveform resolving into a text baseline, matching `AudiblMark`); tray states differ by silhouette since template images carry no color. The Windows/Android/iOS files `bun tauri icon` emits were deleted — this app is macOS only.
  - Note: the font, the sidebar marker, the window minimum, the mark and the overlay described in this entry were all superseded by the 2026-09-18 rebrand entry below.
  - Gates at the end of the pass: `bunx tsc --noEmit`, `bun run build`, `bun run check:translations` (103 keys), `cargo clippy`, `cargo test` (96 passed) all clean.

- Rebrand and follow-up fixes (2026-09-18), from the plan at `~/.claude/plans/added-brand-assets-look-at-glowing-feigenbaum.md`:
  - **Renamed Audible → Audibl** (Amazon trademark), identifier included: `com.kennnguyen.audibl` for app data, logs and the Keychain service. The Rust crate, binary (`target/debug/audible`) and the `AUDIBLE_*` dev env vars keep the old spelling — internal only. The rename means a fresh app-data dir, a fresh Keychain entry and fresh TCC prompts; carry the model over with `cp -R ~/Library/Application\ Support/com.kennnguyen.audible/models ~/Library/Application\ Support/com.kennnguyen.audibl/` to skip the 1.4 GB download.
  - **Brand mark** traced from `brand-assets/Audibl icon.jpg`: seven square-ended bars in a symmetric arch, the middle bar lifted off the baseline. The bar table is duplicated in `src/components/ui/icons.tsx` (`AudiblMark`) and `scripts/generate-icons.ts` (`MARK_BARS`) — change both together. The app icon inverts the palette (ink mark on a warm off-white tile) because the brand asset is black on white; tray icons stay template silhouettes.
  - **One font:** Plus Jakarta Sans dropped, Newsreader sets everything.
  - General: the Mute While Recording and Groq API Key captions are gone, and the Clean and Reformat group title carries an `InfoTip glyph="?"` (`general.cleanHowTo`) with the four steps for turning it on. `Group`'s `title` is now a `ReactNode`.
  - Sidebar: the amber current-page strip is gone; the selected tab is the `bg-control` fill alone.
  - Main window is `resizable(false)` / `maximizable(false)` at 900×640; the minimum size is gone with it.
  - Overlay: panel 300×56, the pill hugs its label (a fixed width left dead space in Hold mode), bars shrank to 2px wide in a 20px track, `↩` is spelled "Return", and a `record`-red light sits outside the pill to the right, breathing while the mic is open and dark once the work starts.
  - **Waveform, not spectrum:** `audio/visualizer.rs` (FFT, 16 frequency buckets, ported from Handy) is deleted along with the `rustfft` dependency. `audio/level.rs` emits one RMS amplitude per 1/30 s and `Overlay.tsx` keeps the last 14 in a ring, so each bar is a moment in time and the row scrolls leftwards.
  - Gates: `bunx tsc --noEmit`, `bun run build`, `bun run check:translations` (102 keys), `cargo clippy`, `cargo test` (100 passed) all clean.

- Polish pass (2026-09-18), from the plan at `~/.claude/plans/audibl-logo-left-and-tingly-kite.md` (live overlay text was considered and dropped: Qwen3-ASR has no streaming mode):
  - Sidebar: the wordmark spans the tab column edge to edge (`AudiblWordmark` takes a `width`, and its viewBox is cropped to the ink), sitting in a 60px block (PageTitle's 36px line + `mb-6`), so the first tab's top lines up with the first card's top on every page. The version footer is replaced by the slogan "Be heard. Be Audibl" (`sidebar.slogan`, English in both locales).
  - Window is 900×712 (still fixed): General's full height plus `pb-10`, so it never scrolls. `Group` drops its bottom margin when it is the last child.
  - Shortcut keycaps nudged 1px down (system-font glyph fallback sits high). The reset button uses the new `IconButton variant="bare"` (no fill, hover only brightens) with a 22px icon.
  - Toggle knob is light in both states (`bg-text` on, `bg-muted` off); the dark knob on amber read smaller.
  - Onboarding: English/Tiếng Việt `Segmented` above the headline (same `set_app_language` wiring as Advanced) and the slogan under it (`onboarding.slogan`; later moved to the Finish outro).
  - `general.toggleCaption` is "Press to start; Return pastes, Esc cancels".
  - Overlay: no instructions in the pill — recording shows only the dot and the waveform; "Transcribing…"/"Cleaning…" stay. `overlay.listening`/`toggleHint` deleted, and `ShowPayload.toggle` / `overlay::show`'s `toggle` argument removed. The red dot moved inside the pill on the left, 8px. Track height 24px, pill `h-10`.
  - Waveform range: `audio/level.rs` learns the mic's noise floor (drops at once to quieter readings, rises 2 dB/s, capped at -45 dBFS so speech can't become the floor), maps `floor + 4 dB`..-22 dBFS onto 0..1, and expands with power 1.2. Room tone reads ≈0, dictation ≥0.6.
  - Gates: `bunx tsc --noEmit`, `bun run build`, `bun run check:translations` (102 keys), `cargo clippy`, `cargo test` (102 passed) all clean.

- Review fixes (2026-09-18):
  - **"Clean and Reformat" is now called "Magic Touch"** in the UI (both locales keep the English name), comments and logs. The i18n keys (`general.cleanAndReformat`…), the `clean_and_reformat` setting, `set_clean_and_reformat` and the overlay's `cleaning` state keep the old internal names so saved settings survive.
  - Slogan is "Be heard. Be Audibl." (with the period). vi: `defaultMicrophone` "Mặc định", `holdCaption` "Giữ để nói, thả để dán".
  - **On/Off chimes:** `sfx.rs` plays `resources/sfx/On.mp3` when recording starts and `Off.mp3` when it stops or is cancelled (both embedded with `include_bytes!`, played by `NSSound` on the main thread; no new crate). `sfx/` at the repo root is Ken's original drop. Mute While Recording now mutes `sfx::ON_DURATION` (600 ms) after the start so the chime is heard; the delayed mute checks `recording` under the mute lock, so a stop in between never leaves output muted. No setting to turn the chimes off.
  - General: the Groq API Key row shows a persistent status at the right of its label (✓ "Saved" in `text-success`, else muted "Not saved"); the transient "Key saved." line and `general.keySaveSuccess` are gone. Replacing a saved key shows Save / Cancel / Remove; Cancel returns to the redacted view (`general.cancel`).
  - History shows no date or time; the meta line under each entry is the app name alone.
  - Gates: `bunx tsc --noEmit`, `bun run build`, `bun run check:translations` (104 keys), `cargo clippy`, `cargo test` (102 passed) all clean.

- Onboarding language card (2026-09-18): the Interface Language `Segmented` moved off its own end-aligned row above the headline into a `StepCard`, first in the stack (`onboarding.language.title`, `GlobeIcon`), with the control sitting to the right of the title text in the card's header row — same position as each permission card's checkmark. `StepCard` gained `right` (header-row trailing slot) and `hideCheck` (suppresses the done checkmark for a card that isn't a granted permission); `children` is now optional since this card has no body. New locale key `onboarding.language.title` (en/vi, 105 keys). The whole card stack is now vertically centered in the window (`flex h-full flex-col items-center justify-center` on the scroll container, the `max-w-[560px]` column `shrink-0` with `py-12` replacing the old `pt-14`/`pb-12`), instead of pinned to the top.

- Onboarding outro (2026-09-18): the slogan no longer sits under the onboarding headline. Pressing Finish plays an outro in `Onboarding.tsx` (phases `steps → leaving → slogan → closing`): the card column fades out (500 ms), "Be heard. Be Audibl." fades in alone, centred, italic at the new `text-display` size (56px, `--text-display` in `index.css`) with a slight rise (700 ms), holds 1.8 s, fades out (600 ms); only then is `completeOnboarding` called — it emits `settings-changed`, which makes `App` swap to the main window at once, so calling it first would cut the outro off. The finish logic moved from `ModelStep` up to `Onboarding` (`ModelStep` takes `finishing`/`onFinish`); a `model_missing` failure returns to the cards and remounts `ModelStep` to re-read status. The main window then fades in via the new `.animate-enter` (500 ms), applied only when `App` rendered onboarding earlier in the session (`cameFromOnboarding` ref), not on ordinary launches.

- Magic Touch "Show me how" (2026-09-18): the `InfoTip glyph="?"` on the Magic Touch group title is gone (superseding the 2026-09-18 "UI fix" entry above), along with its `cleanHowTo` locale key. The Use Magic Touch row's caption now ends with a clickable "Show me how" (`general.showMeHow`) that calls the same `setCleanWarning(true)` path as a failed toggle attempt, revealing the existing below-group instructions line (`console.groq.com/keys` link via `open_groq_keys_page`) — so there are now two ways to reach the same instructions. `InfoTip`'s "?" glyph variant is unused again but kept on the component for future reuse; only the Advanced page's "i" tooltips use `InfoTip` now.

### Not yet verified (run by hand)

- Fresh onboarding on a clean data dir, Wi-Fi off mid-download (Retry resumes), quit mid-download and relaunch, deleting the model after onboarding.
- General page Groq key UX after its move off Advanced: invalid/unreachable errors, redacted field, no-key warning (position/fade-in below the group, link opens the browser), Replace/Remove (visual pass + Vietnamese copy).
- Interface Language switch updating main window, overlay, and tray live; Vietnamese-locale first launch.
- Launch on Startup (needs a signed bundle, not `tauri dev`), Show Menu Bar Icon / Start Hidden interaction, `AUDIBLE_UNLOAD_SECS=30` unload/reload in the log.
- Ken's own visual review of the redesign. Screenshot-verified during the pass: all four pages and onboarding in en, General/Dictionary/Advanced in vi, and the overlay's recording and transcribing states. Not yet seen on screen: the overlay's cleaning state, the model download's downloading/verifying/failed views, the Groq key's saved/error states, Dictionary's open add form, History's copied and expanded states.
- Vietnamese copy review by Ken (the redesign added captions, empty states and an onboarding headline in both locales).
- The tray icon in a light menu bar, and the new app icon in the Dock and in Finder.
- Onboarding under the new identifier: macOS re-asks for Microphone and Accessibility, and the Groq key has to be entered again (new Keychain service).
- The On/Off chimes by ear, with Mute While Recording on and off, and whether the On chime ever leaks into the transcript.
- The overlay waveform against real speech (the adaptive floor in `audio/level.rs` is tested on synthetic sines only) and its cleaning state.
- Onboarding's language switch and the Toggle knob's on state. (The Finish outro was confirmed by Ken on 2026-09-18.)

### Known limitations

- Consecutive dictations are pasted with no separator (no automatic leading space).
- Custom words of 4–5 characters are only corrected on an exact case-insensitive match (e.g. "Tory" is not corrected to "Tauri"); the Groq step still sees them as preferred spellings.
- Custom words that sound like a common word ("Claude" heard as "cloud") or differ only in diacritics are left to Groq; uncommon English words outside the SCOWL list can still be replaced; ordinary word pairs can still join into a custom term ("all bright" → "Albright").
- Secure Input only warns; Handy's Carbon fallback re-registration is not ported.
- Groq may still normalize Vietnamese particles ("nha" → "nhé") despite the prompt; the translation guard only catches lost words, not swapped particles.
- Old app data at `~/Library/Application Support/com.kennnguyen.audible` is orphaned by the rename and can be deleted once the new install is trusted.
## What this project is

**Audibl**: a macOS-only voice-to-text dictation app, cloning Wispr Flow. Press a shortcut, speak English or Vietnamese, and the text is pasted into the focused app. v1 scope:

- Transcribe Shortcut with Hold / Toggle modes (Toggle: Esc cancels, Return finishes)
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
- `coordinator.rs`: pure `CoordinatorState::on_input` (Idle → Recording{session, mode} → Processing{session}); mode is fixed at recording start; Esc cancels recording or processing; shortcut presses are ignored while processing; `ProcessingDone(session)` only ends its own session. `Coordinator` runs it on one thread and calls `actions::run_effect`.
- `actions.rs`: effects. Start checks `is_setup_complete` (else opens the window), captures context, opens the mic, preloads the model, arms Esc (and Return in Toggle), shows overlay + tray state. Finish disarms Return, stops the mic, then a pipeline thread transcribes → `pipeline::process` → disarms Esc → pastes → history. `ACTIVE_SESSION` guards paste/teardown so a cancelled session never touches a newer one.
- `shortcut.rs`: `ShortcutManager` owns the handy-keys `HotkeyManager` on its own thread (blocking tap swallows registered keys); falls back to tauri-plugin-global-shortcut if the tap can't start. Ids `transcribe`, `cancel` (escape), `finish` (return). Registration waits on that thread, so key events only forward to the coordinator. `start_capture`/`stop_capture` stream `shortcut-capture-event` to `components/ShortcutInput.tsx` and suspend the current shortcut meanwhile.
- `overlay.rs`: tauri-nspanel non-activating panel (label `recording_overlay`, 300×56; the pill holds the record dot, the waveform and, once busy, a status word — no key hints) centred above the Dock on the cursor's screen, click-through via `set_ignore_cursor_events(true)`. `show-overlay {state: recording|transcribing|cleaning}` / `hide-overlay` events to `src/overlay/Overlay.tsx`.
- `paste.rs`: on the main thread: save clipboard (text, else image) → write text → Cmd + layout-aware V keycode via enigo → restore. `cursor_location()` shares the enigo instance.
- `text/filler.rs`: English (um, uh, uhm, umm, er, ah, hmm) and Vietnamese (ờ, ờm, ừm, ưm, ơ, hừm) fillers, always both lists; "à"/"ừ" are kept. Also collapses a word repeated 3+ times and tidies punctuation/capitalization.
- `text/dictionary.rs`: `apply_custom_words` (NFC, char Levenshtein ≤ 18% of the longer key, 1–3-word n-grams that never cross punctuation and must use every word, keys ≥ 4 chars; words of 4–5 chars therefore only match exactly; single words in `text/common_words_en.txt` (SCOWL levels 10–20, 4+ letters) are never replaced or recased; words with diacritics only match exactly ignoring case) and `apply_replacements` (one leftmost-first regex, longest trigger first, `\b` only on word-character edges, `(?i)` unless Clean and Reformat is on; returns the inserted values).
- `pipeline.rs`: `process_local` (NFC → fillers if on → custom words → replacements) then Clean and Reformat (Milestone 6). Case sensitivity follows the Clean and Reformat setting even when Groq later fails.
- `cleanup.rs`: Groq `openai/gpt-oss-120b`, `reasoning_effort: "low"`, `include_reasoning: false`, 10 s timeout. User message is JSON `{app, bundle_id, window_title, language, dictionary, keep_verbatim, transcript}`. `sanitize_response` strips `<think>`, wrapping quotes, whitespace. The prompt applies dictionary spellings only where the speaker means that term. `choose_output` keeps the local text if Groq failed, returned nothing, or lost/re-cased any `keep_verbatim` value (the replacement values inserted locally), or translated mixed-language words (`translated_mixed_words`: Groq dropped a word of one language and added a word of the other, e.g. "support" → "hỗ trợ"; one such word rejects the output). The prompt's mixed-language section comes first and overrides the formatting rules; context changes formatting, not vocabulary. `groq_live_mixed_language_not_translated` (`RUNS=n`) replays Ken's real mixed transcripts. `pipeline.rs` races the request against the session being cancelled.
- `context.rs`: at recording start, frontmost app name + bundle id (NSWorkspace) and focused window title (AX `AXFocusedWindow` → `AXTitle`, truncated to 120 chars).
- `history.rs`: `history.json` store, newest first, capped at 10 (`push_capped`); entries hold pasted text, raw transcript, app name, timestamp (ms, also the id). Emits `history-changed`; tray Copy Last Transcript copies the newest.
- `secure_input.rs`: polls `IsSecureEventInputEnabled()` every second (only while the handy-keys tap is the backend); held ≥ 3 s (`SustainTracker`) shows the tray warning item, released clears it.
- `permissions.rs`: sync mic/Accessibility checks. `lib.rs` `on_ready` registers the shortcut once onboarding is done and Accessibility is granted (a missing model doesn't block it: the press opens the download screen). `is_setup_complete` (onboarding + permissions + model) decides whether launch shows the window and whether a press records.
- `sfx.rs`: On/Off chimes (`NSSound`, mp3s embedded from `resources/sfx/`), played by `actions.rs` at recording start and stop/cancel.
- `autostart.rs`: Launch on Startup via `SMAppService` (fails harmlessly in `tauri dev`).
- Frontend stores: `src/stores/settings.ts`, `src/stores/history.ts`.
- Frontend: `src/main.tsx` → `App.tsx` (shows `components/Onboarding.tsx` for new users, or for returning users at the first missing permission/model; else sidebar + pages in `src/pages/`), `src/stores/settings.ts` (zustand mirror; `applySettings(commands.setX(..))` stores the returned settings; `settings-changed` keeps it live), `src/components/ui/` (Group/Row/EmptyRow/PageTitle/PageHeader, Button/IconButton, Toggle, Segmented, Select, TextField, InfoTip, `icons.tsx`), `src/components/ShortcutInput.tsx` (`formatShortcut` returns the readable name for aria-labels/errors; the button itself renders keycap spans per key — symbols for modifiers/space/return/escape/tab/arrows, uppercase text otherwise, `_left`/`_right` suffixes stripped — plus an exported `ResetShortcutButton` that calls `change_shortcut` with the hardcoded default, kept in sync with `settings::DEFAULT_SHORTCUT`), `src/i18n/` (react-i18next, language follows the stored setting and `settings-changed`), `src/overlay/` (recording overlay webview entry), `src/index.css` (the whole design system: one dark palette, fonts, radii and a px-pinned type scale in `@theme`; element resets in `@layer base`; global focus-visible ring; `.animate-fade-in` and `sweep` keyframes; `prefers-reduced-motion`; global scrollbar hiding — scroll stays functional, just no visible track/thumb).

## UI rules

The design system is Wispr Flow's visual language in a dark-only palette. All
tokens live in `src/index.css`; nothing else defines a color, radius or size.

- **Dark only.** One palette, no `prefers-color-scheme` branch. `lib.rs` pins
  `NSAppearanceNameDarkAqua` so native chrome (traffic lights, `<select>`
  popups) stays dark whatever macOS is set to.
- **Colors:** `sidebar` `#0e0d0c` → `bg` `#161412` → `surface` `#1e1b18` →
  `control` `#272320`, hairline `border` `#2b2723`, text `#f5f1ea`, muted
  `#9a9186`. Amber `accent` `#e8b33c` is the only saturated color and marks one
  thing per screen: what is focused, what is on, or what is listening. The one
  exception is `record` `#e5484d`, used by nothing but the overlay's recording
  light, which has to read as red over any window.
- **No shadow on anything that does not float.** Depth is the surface ladder
  plus a 1px border. `shadow-*` belongs only to the recording overlay and
  popovers.
- **Two radii:** `rounded-card` (14px) for containers, `rounded-control` (8px)
  for controls, `rounded-full` for pills and toggles.
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
  survives on the Advanced page, plus the General page's Clean and Reformat
  group title, where the explanation is a four-step errand (`glyph="?"`).
- No template chrome: no tracked-out ALL-CAPS eyebrows, no `A · B · C` meta
  strings, no `→` appended to button text, no `font-mono` for non-code.
- Motion answers actions only. No section entrance animations. The overlay's
  sweep is the one piece of ambient motion, and `prefers-reduced-motion` is
  honoured globally.
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
