<p align="center">
  <img src="brand-assets/Audibl%20full%20logo.png" alt="Audibl" width="320">
</p>

<p align="center"><em>Be heard. Be Audibl.</em></p>

Audibl is a voice-to-text dictation app for macOS. Press a shortcut in any app, speak English or Vietnamese, and the text is pasted where your cursor is. Speech recognition runs locally on your Mac.

## Features

- **One shortcut, two modes.** Hold: hold the shortcut while you speak and release to paste. Toggle: press to start, then Return pastes and Esc cancels. The default shortcut is Option + Space.
- **Local transcription.** Qwen3-ASR 1.7B runs on the GPU through Metal. It detects English or Vietnamese automatically, including mixed speech.
- **Dictionary.** Custom words fix names and terms the model misspells. Replacements turn a spoken phrase into fixed text.
- **Filler removal.** Removes "um", "uh", "ờ", "ừm" and repeated words.
- **Magic Touch (optional).** Sends the transcript to Groq for punctuation and formatting that suit the app you are typing in. You need your own Groq API key. Audibl stores the key in the macOS Keychain.
- **History.** Shows your last 10 dictations. You can copy any of them again.
- **English or Vietnamese interface.**

## Install

Requirements: a Mac with Apple Silicon (M1 or later) and macOS 13 Ventura or newer.

1. Download `Audibl_1.0.0_aarch64.dmg` from the [latest release](../../releases/latest).
2. Follow [INSTALL.md](INSTALL.md).

This build is not notarized by Apple. On the first launch, macOS blocks it until you allow it. INSTALL.md shows how.

On the first launch, Audibl asks for Microphone and Accessibility access. Then it downloads the speech model once (1.4 GB).

## Privacy

Your audio never leaves your Mac. Only when Magic Touch is on does Audibl send the transcript text to Groq. It also sends your dictionary, the name of the frontmost app and the title of its window, for context.

## Build from source

You need [Bun](https://bun.sh), [Rust](https://rustup.rs) and the Xcode Command Line Tools.

```bash
bun install
bun run tauri dev      # run a development build
bun run release        # build Audibl.app and the DMG (Apple Silicon)
```

The release output is in `src-tauri/target/aarch64-apple-darwin/release/bundle/`.

Other checks:

```bash
bun run build                   # TypeScript check and frontend build
bun run check:translations      # English and Vietnamese strings match
cd src-tauri && cargo test      # Rust unit tests
```

## Credits

- [Handy](https://github.com/cjpais/Handy) (MIT) is the architectural reference. Audibl ports its audio capture, voice activity detection, shortcut handling, overlay and paste code. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- [Qwen3-ASR](https://huggingface.co/Qwen) is the speech model.
- Built with [Tauri](https://tauri.app).

## License

[MIT](LICENSE)
