//! Start and stop chimes around a recording, played with `NSSound` from the
//! embedded mp3s in `resources/sfx/`.

use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_app_kit::NSSound;
use objc2_foundation::NSData;
use std::cell::RefCell;
use std::time::Duration;
use tauri::AppHandle;

static ON_MP3: &[u8] = include_bytes!("../resources/sfx/On.mp3");
static OFF_MP3: &[u8] = include_bytes!("../resources/sfx/Off.mp3");

/// Length of the On chime; Mute While Recording waits this long before muting
/// so the chime is heard.
pub const ON_DURATION: Duration = Duration::from_millis(600);

#[derive(Clone, Copy)]
pub enum Sound {
    On,
    Off,
}

thread_local! {
    // NSSound stops when released, so both sounds live for the whole run.
    // Only touched on the main thread.
    static SOUNDS: RefCell<Option<(Retained<NSSound>, Retained<NSSound>)>> =
        const { RefCell::new(None) };
}

fn load(bytes: &[u8]) -> Option<Retained<NSSound>> {
    let data = NSData::with_bytes(bytes);
    NSSound::initWithData(NSSound::alloc(), &data)
}

/// Plays `sound` without blocking; a repeat restarts it from the beginning.
pub fn play(app: &AppHandle, sound: Sound) {
    let _ = app.run_on_main_thread(move || {
        SOUNDS.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_none() {
                match (load(ON_MP3), load(OFF_MP3)) {
                    (Some(on), Some(off)) => *slot = Some((on, off)),
                    _ => {
                        log::error!("Could not decode the sound effects");
                        return;
                    }
                }
            }
            let (on, off) = slot.as_ref().unwrap();
            let target = match sound {
                Sound::On => on,
                Sound::Off => off,
            };
            target.stop();
            target.play();
        });
    });
}
