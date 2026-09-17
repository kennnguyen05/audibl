# Third-Party Notices

## Handy

Parts of Audible are ported from [Handy](https://github.com/cjpais/Handy), used under the MIT License below.

Ported or adapted code, in `src-tauri/`:

- `src/audio/recorder.rs`, `src/audio/resampler.rs`, `src/audio/visualizer.rs`, `src/audio/vad.rs`, `src/audio/mute.rs`: microphone capture, resampling, level visualizer, Silero VAD smoothing, output mute
- `src/model.rs`: resumable, sha256-verified model download
- `src/transcription.rs`: model lifecycle (lazy load, idle unload)
- `src/coordinator.rs`: shortcut state machine pattern
- `src/shortcut.rs`: handy-keys and global-shortcut integration, shortcut capture
- `src/overlay.rs`: recording overlay panel
- `src/paste.rs`: clipboard paste with layout-aware Cmd+V
- `src/text/filler.rs`, `src/text/dictionary.rs`: stutter collapse and custom-word correction
- `src/secure_input.rs`: Secure Input detection
- `src/tray.rs`, `src/tray_i18n.rs`, `build.rs`: tray menu and its generated translations
- `src/autostart.rs`: SMAppService login item
- `resources/silero_vad_v4.onnx`: Silero VAD model as distributed with Handy

```
MIT License

Copyright (c) 2025 CJ Pais

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Silero VAD

`resources/silero_vad_v4.onnx` is from [snakers4/silero-vad](https://github.com/snakers4/silero-vad) (MIT License).

## Speech model

The Qwen3-ASR 1.7B GGUF model downloaded at onboarding is published by the Qwen team under the Apache License 2.0 and converted to GGUF by handy-computer on Hugging Face. It is downloaded at runtime and not bundled with the app.
