//! Paste into the focused app: save the clipboard, write the text, send Cmd+V
//! with the layout-aware V keycode, then restore the clipboard. Requires the
//! Accessibility permission. Ported from Handy (`clipboard.rs`, `input.rs`,
//! MIT); Handy's receipt-sequenced "reliable paste" is not ported.

use enigo::{Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

const DELAY_BEFORE_PASTE: Duration = Duration::from_millis(60);
const DELAY_BEFORE_RESTORE: Duration = Duration::from_millis(120);
const MODIFIER_HOLD: Duration = Duration::from_millis(100);

static ENIGO: Mutex<Option<Enigo>> = Mutex::new(None);

fn with_enigo<T>(f: impl FnOnce(&mut Enigo) -> Result<T, String>) -> Result<T, String> {
    let mut guard = ENIGO.lock().unwrap_or_else(|p| p.into_inner());
    if guard.is_none() {
        *guard = Some(Enigo::new(&Settings::default()).map_err(|e| format!("enigo: {e}"))?);
    }
    f(guard.as_mut().unwrap())
}

/// Mouse location in logical points, used to pick the overlay's monitor.
pub fn cursor_location() -> Option<(i32, i32)> {
    with_enigo(|enigo| enigo.location().map_err(|e| e.to_string())).ok()
}

/// Pastes on the main thread (TIS keyboard layout APIs require it) and waits
/// for the result.
pub fn paste(app: &AppHandle, text: String) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(paste_on_main(&handle, &text));
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

fn paste_on_main(app: &AppHandle, text: &str) -> Result<(), String> {
    let clipboard = app.clipboard();
    let saved_text = clipboard.read_text().ok().filter(|t| !t.is_empty());
    let saved_image = if saved_text.is_none() {
        clipboard.read_image().ok().map(|image| image.to_owned())
    } else {
        None
    };

    clipboard
        .write_text(text)
        .map_err(|e| format!("clipboard write: {e}"))?;
    std::thread::sleep(DELAY_BEFORE_PASTE);

    let result = with_enigo(|enigo| {
        let v = macos::command_v_key();
        enigo
            .key(Key::Meta, Direction::Press)
            .map_err(|e| format!("press Cmd: {e}"))?;
        let click = enigo
            .key(v, Direction::Click)
            .map_err(|e| format!("click V: {e}"));
        std::thread::sleep(MODIFIER_HOLD);
        enigo
            .key(Key::Meta, Direction::Release)
            .map_err(|e| format!("release Cmd: {e}"))?;
        click
    });

    std::thread::sleep(DELAY_BEFORE_RESTORE);
    if let Some(previous) = saved_text {
        let _ = clipboard.write_text(previous);
    } else if let Some(image) = saved_image {
        let _ = clipboard.write_image(&image);
    } else {
        let _ = clipboard.clear();
    }
    result
}

mod macos {
    use enigo::Key;
    use std::ffi::c_void;

    type TisInputSourceRef = *const c_void;
    type CfDataRef = *const c_void;
    type CfStringRef = *const c_void;

    /// kVK_ANSI_V, the fallback when the layout can't be read.
    const ANSI_V_KEYCODE: u16 = 9;
    const KEYCODE_COUNT: u16 = 128;
    const UC_KEY_ACTION_DISPLAY: u16 = 3;
    const UC_KEY_TRANSLATE_NO_DEAD_KEYS_MASK: u32 = 1;
    /// Carbon cmdKey (bit 8) shifted right by 8 for UCKeyTranslate.
    const COMMAND_MODIFIER_STATE: u32 = 1;

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn TISCopyCurrentKeyboardLayoutInputSource() -> TisInputSourceRef;
        fn TISGetInputSourceProperty(source: TisInputSourceRef, key: CfStringRef) -> CfDataRef;
        static kTISPropertyUnicodeKeyLayoutData: CfStringRef;
        fn UCKeyTranslate(
            key_layout: *const u8,
            virtual_key_code: u16,
            key_action: u16,
            modifier_key_state: u32,
            keyboard_type: u32,
            key_translate_options: u32,
            dead_key_state: *mut u32,
            max_string_length: usize,
            actual_string_length: *mut usize,
            unicode_string: *mut u16,
        ) -> i32;
        fn LMGetKbdType() -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFDataGetBytePtr(data: CfDataRef) -> *const u8;
        fn CFRelease(value: *const c_void);
    }

    struct InputSource(TisInputSourceRef);

    impl Drop for InputSource {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0) };
            }
        }
    }

    /// The physical key macOS reads as `v` while Command is held, so paste
    /// works on Dvorak, Colemak, and non-Latin layouts. Main thread only.
    fn resolve_command_v_keycode() -> Option<u16> {
        let source = InputSource(unsafe { TISCopyCurrentKeyboardLayoutInputSource() });
        if source.0.is_null() {
            return None;
        }
        let data = unsafe { TISGetInputSourceProperty(source.0, kTISPropertyUnicodeKeyLayoutData) };
        if data.is_null() {
            return None;
        }
        let layout = unsafe { CFDataGetBytePtr(data) };
        if layout.is_null() {
            return None;
        }
        let keyboard_type = unsafe { LMGetKbdType() } as u32;
        (0..KEYCODE_COUNT).find(|&keycode| {
            let mut dead_key_state = 0;
            let mut chars = [0_u16; 4];
            let mut length = 0_usize;
            let status = unsafe {
                UCKeyTranslate(
                    layout,
                    keycode,
                    UC_KEY_ACTION_DISPLAY,
                    COMMAND_MODIFIER_STATE,
                    keyboard_type,
                    UC_KEY_TRANSLATE_NO_DEAD_KEYS_MASK,
                    &mut dead_key_state,
                    chars.len(),
                    &mut length,
                    chars.as_mut_ptr(),
                )
            };
            status == 0 && length == 1 && chars[0] == u16::from(b'v')
        })
    }

    pub fn command_v_key() -> Key {
        let keycode = resolve_command_v_keycode().unwrap_or(ANSI_V_KEYCODE);
        Key::Other(u32::from(keycode))
    }
}
