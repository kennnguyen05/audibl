//! Frontmost app context, captured when recording starts: app name and bundle
//! id (NSWorkspace) plus the focused window title (Accessibility API). History
//! records the app name; Clean and Reformat sends all three to Groq.

use objc2_app_kit::NSWorkspace;
use std::ffi::{c_char, c_void, CStr};
use std::sync::Mutex;
use tauri::AppHandle;

const MAX_TITLE_CHARS: usize = 120;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppContext {
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
}

static LAST: Mutex<Option<AppContext>> = Mutex::new(None);

pub fn capture(_app: &AppHandle) {
    let context = frontmost();
    log::debug!(
        "Context: app={:?} bundle={:?} title_chars={}",
        context.app_name,
        context.bundle_id,
        context.window_title.as_ref().map_or(0, |t| t.chars().count())
    );
    *LAST.lock().unwrap() = Some(context);
}

/// Context of the current dictation (captured at recording start).
pub fn current() -> AppContext {
    LAST.lock().unwrap().clone().unwrap_or_default()
}

fn frontmost() -> AppContext {
    let workspace = NSWorkspace::sharedWorkspace();
    let Some(app) = workspace.frontmostApplication() else {
        return AppContext::default();
    };
    let app_name = app.localizedName().map(|s| s.to_string());
    let bundle_id = app.bundleIdentifier().map(|s| s.to_string());
    let window_title = focused_window_title(app.processIdentifier()).map(truncate_title);
    AppContext {
        app_name,
        bundle_id,
        window_title,
    }
}

pub fn truncate_title(title: String) -> String {
    let title = title.trim();
    if title.chars().count() <= MAX_TITLE_CHARS {
        title.to_string()
    } else {
        let cut: String = title.chars().take(MAX_TITLE_CHARS).collect();
        format!("{cut}…")
    }
}

type CFTypeRef = *const c_void;
type AXUIElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFTypeRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(alloc: CFTypeRef, c_str: *const c_char, encoding: u32) -> CFTypeRef;
    fn CFStringGetCString(string: CFTypeRef, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFRelease(cf: CFTypeRef);
}

const UTF8: u32 = 0x0800_0100;

struct Owned(CFTypeRef);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

fn cf_string(s: &CStr) -> Owned {
    Owned(unsafe { CFStringCreateWithCString(std::ptr::null(), s.as_ptr(), UTF8) })
}

fn copy_attribute(element: CFTypeRef, name: &CStr) -> Option<Owned> {
    let attribute = cf_string(name);
    let mut value: CFTypeRef = std::ptr::null();
    let status = unsafe { AXUIElementCopyAttributeValue(element, attribute.0, &mut value) };
    (status == 0 && !value.is_null()).then(|| Owned(value))
}

fn to_string(value: &Owned) -> Option<String> {
    if unsafe { CFGetTypeID(value.0) != CFStringGetTypeID() } {
        return None;
    }
    let mut buffer = vec![0 as c_char; 2048];
    let ok = unsafe { CFStringGetCString(value.0, buffer.as_mut_ptr(), buffer.len() as isize, UTF8) };
    ok.then(|| unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy().into_owned())
}

/// Needs the Accessibility permission Audible already requires for paste.
fn focused_window_title(pid: i32) -> Option<String> {
    let app = Owned(unsafe { AXUIElementCreateApplication(pid) });
    if app.0.is_null() {
        return None;
    }
    let window = copy_attribute(app.0, c"AXFocusedWindow")?;
    let title = copy_attribute(window.0, c"AXTitle")?;
    to_string(&title).filter(|t| !t.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::truncate_title;

    #[test]
    fn long_titles_are_truncated() {
        let long = "a".repeat(300);
        let cut = truncate_title(long);
        assert_eq!(cut.chars().count(), 121);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn short_titles_are_trimmed_only() {
        assert_eq!(truncate_title("  Inbox — Gmail ".into()), "Inbox — Gmail");
    }
}
