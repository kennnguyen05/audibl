# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Audibl is a macOS-only desktop app (Tauri 2 + React 18 + Tailwind 4), so its UI is a webview, not a native AppKit surface. Design work follows web practice inside macOS conventions — menu bar item, non-activating floating panel, Dock/Accessory activation policy — not iOS or Android design languages. Apple Silicon only, macOS 13 Ventura or newer.

## Users

Primary: bilingual Vietnamese ↔ English speakers who code-switch inside a single sentence and are failed by monolingual dictation. They dictate into whatever app is focused — editor, browser, chat, email — with the app itself never in the foreground. The interaction is: press a shortcut, speak, get text pasted at the cursor. The settings window is visited rarely, usually once during onboarding and afterwards only to edit the Dictionary.

Secondary, not optimized for: privacy-conscious Mac users who want local transcription, and developers comfortable building from source.

## Product Purpose

Turn speech into pasted text in any macOS app, with transcription running locally on the Mac. Success is that a dictation lands correctly the first time — right words, right language, right names — so the user never has to open Audibl to fix it. The app's visible surface area during normal use is one small overlay and a menu bar icon.

## Positioning

Vietnamese and English handled unsegregated. The language is auto-detected per utterance, mixed vi/en speech is transcribed as spoken, and the optional cleanup step is explicitly guarded (`cleanup::translated_mixed_words`) so it cannot translate a mixed word into the other language. Competitors either force a language choice or quietly normalize mixed speech into one of them.

## Operating Context

- The app is used from inside other apps. The main window is closed to the menu bar during real use; the only on-screen UI while dictating is a 300×56 non-interactive overlay pill above the Dock.
- Recording is driven by one shortcut (default Option + Space) in Auto / Hold / Toggle modes; Esc cancels. Return is deliberately never registered so it always reaches the focused app.
- macOS gates the product: Microphone and Accessibility permissions, Secure Input (which silently blocks the key tap and only surfaces as a tray warning), and Gatekeeper on an unnotarized build. Ad-hoc signing means permissions are re-requested after every rebuild.
- First run is a hard gate: onboarding must complete permissions plus a one-time 1.4 GB model download before the shortcut does anything.
- Optional Magic Touch sends the transcript, dictionary, frontmost app name and window title to Groq; the key lives in the macOS Keychain.

## Capabilities and Constraints

- Transcription: Qwen3-ASR 1.7B Q5_K_M via transcribe-cpp on Metal, loaded on demand, unloaded after 600 s idle. Chosen over Whisper large-v3-turbo for speed (0.86 s vs 1.98 s per 10 s of audio) despite slightly worse WER. No model picker, no model options in the UI — settled.
- **No streaming.** Qwen3-ASR has no streaming mode, so the overlay can never show live text while the user speaks. Any design that implies live transcription is unbuildable.
- Local text pipeline: NFC → filler removal (en + vi lists always both) → custom words → replacements, then optional Groq step, then dictionary spelling enforced last.
- History is capped at 10 entries. Dictionary holds custom words and replacements.
- Every user-visible string exists in both `en` and `vi` (`bun run check:translations` enforces parity). Vietnamese copy is longer than English and layouts must absorb it.
- Internal names are frozen for settings compatibility: the UI says "Magic Touch" but the setting, command, i18n keys and overlay state keep the `clean_and_reformat` / `cleaning` names. This is a data-migration constraint, not a naming preference.
- Out of scope for v1: Command Mode, snippets beyond replacements, per-app style settings.

## Brand Commitments

- Name: **Audibl** — renamed off "Audible" to avoid the Amazon trademark. The old spelling survives only in internal identifiers (Rust crate, binary, `AUDIBLE_*` env vars). Any new user-facing text uses "Audibl".
- Existing assets: `brand-assets/Audibl full logo.png`, `brand-assets/Audibl icon.jpg`, the bar mark duplicated in `src/components/ui/icons.tsx` (`AudiblMark`) and `scripts/generate-icons.ts` (`MARK_BARS`), tray PNGs in `src-tauri/resources/`, On/Off chimes in `sfx/`. README tagline: "Be heard. Be Audibl."
- The user did not mark the name, mark or tagline as un-redesignable; they are the incumbent identity, not a locked one. The trademark constraint on "Audible" is binding regardless.

## Evidence on Hand

- Real: the full running UI (`src/pages/`, `src/components/`), the design system in `src/index.css`, both locale files, brand assets above, a model bake-off measured on 15 synthetic clips, and Ken's own mixed-language transcripts replayed in `groq_live_mixed_language_not_translated`.
- Absent, and not to be invented: user counts, testimonials, press, benchmarks against named competitors, pricing (the app is free and MIT), and any claim of notarization or App Store presence. There is no public release, website or landing page yet; the app is being refined and ported to other platforms before launch.

## Product Principles

1. **Invisible in use.** The product is the paste, not the window. Anything that pulls the user's attention to Audibl during a dictation is a defect.
2. **Mixed language is the default case, not an edge case.** Vietnamese and English in one sentence is what the product is for; nothing may quietly normalize toward one of them.
3. **Local first, disclosed second.** Audio never leaves the Mac. The one path that sends text out (Magic Touch) is opt-in, key-owned by the user, and states what it sends.
4. **The first run is the product's hardest moment.** Permissions, a 1.4 GB download and (until notarized) Gatekeeper all stand between install and first success; onboarding carries more design weight than any settings page.
5. **Correct the first time.** Dictionary, filler removal and spelling enforcement exist so the user never edits after pasting.

## Accessibility & Inclusion

No formal standard was established. Product-specific requirements that follow from the above: full English/Vietnamese parity including the tray and overlay; Vietnamese diacritics must render everywhere (Newsreader is self-hosted with a Vietnamese subset); `prefers-reduced-motion` is honoured globally; the overlay is non-interactive by design and must never be the only channel for information the user needs.
