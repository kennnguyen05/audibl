# Installing Audibl

Audibl runs on Apple Silicon Macs (M1 or later) with macOS 13 Ventura or newer.

1. Open `Audibl_1.0.0_aarch64.dmg`.
2. Drag **Audibl** onto the **Applications** folder.
3. Eject the disk image.

## First launch

This build is not yet notarized by Apple, so macOS blocks the first launch with
"Audibl cannot be opened" or "Apple could not verify Audibl".

- Open **System Settings → Privacy & Security**, scroll to the message about
  Audibl, and click **Open Anyway**. Confirm with your password.
- Or, in Terminal: `xattr -dr com.apple.quarantine /Applications/Audibl.app`

You only need to do this once.

## Setup

Audibl walks you through three steps on first launch:

1. **Microphone**: allow access so Audibl can hear you.
2. **Accessibility**: allow access so Audibl can paste text into other apps.
3. **Speech model**: a one-time 1.4 GB download. Transcription then runs fully on your Mac.

Then press **Option + Space** in any app, speak English or Vietnamese, and the text is pasted where your cursor is.
