//! The overlay's microphone level: one amplitude per frame, in time order.
//!
//! The overlay draws a waveform — bar `n` is how loud the room was a moment
//! ago, not how much energy sat in some frequency band — so all this has to
//! produce is a single 0..1 number per ~1/30 s window.

/// Loudness in dBFS mapped onto 0..1 against a noise floor learned from the
/// microphone itself, so room tone reads as zero on a quiet built-in mic and
/// on a hissy USB one alike, and only speech lifts the bars.
///
/// The top of the range: ordinary dictation sits around -30 dBFS, and
/// anything louder than this fills the bar.
const DB_MAX: f32 = -22.0;
/// Room tone has to clear the learned floor by this much before it moves a
/// bar, which keeps breathing and fan noise flat.
const FLOOR_MARGIN_DB: f32 = 4.0;
/// Where the floor starts before the meter has heard anything.
const FLOOR_DEFAULT_DB: f32 = -55.0;
/// The floor never learns a level above this, so a long unbroken stretch of
/// speech cannot teach the meter that speech is silence.
const FLOOR_MAX_DB: f32 = -45.0;
const FLOOR_MIN_DB: f32 = -80.0;
/// The floor drops to any quieter reading at once and creeps back up this
/// slowly, following a room that got noisier without chasing words.
const FLOOR_RISE_DB_PER_SEC: f32 = 2.0;
/// Above 1 expands: the quiet end is pressed towards zero and speech takes
/// most of the height, which is the contrast the waveform is for.
const CURVE_POWER: f32 = 1.2;

pub struct LevelMeter {
    window_size: usize,
    buffer: Vec<f32>,
    floor_db: f32,
    floor_rise_per_window: f32,
}

impl LevelMeter {
    /// One reading per `1 / fps` of audio, which is also the rate the overlay
    /// shifts its waveform at.
    pub fn new(sample_rate: u32, fps: u32) -> Self {
        let fps = fps.max(1);
        let window_size = (sample_rate / fps).max(1) as usize;
        Self {
            window_size,
            buffer: Vec::with_capacity(window_size * 2),
            floor_db: FLOOR_DEFAULT_DB,
            floor_rise_per_window: FLOOR_RISE_DB_PER_SEC / fps as f32,
        }
    }

    /// Returns a level once a full window has arrived, else `None`.
    pub fn feed(&mut self, samples: &[f32]) -> Option<f32> {
        self.buffer.extend_from_slice(samples);
        if self.buffer.len() < self.window_size {
            return None;
        }

        let window = &self.buffer[..self.window_size];
        // Remove DC first: a biased mic would otherwise read as a constant
        // loudness floor.
        let mean = window.iter().sum::<f32>() / self.window_size as f32;
        let sum_sq: f32 = window.iter().map(|s| (s - mean) * (s - mean)).sum();
        let rms = (sum_sq / self.window_size as f32).sqrt();
        self.buffer.clear();

        let db = if rms > 1e-6 {
            20.0 * rms.log10()
        } else {
            -120.0
        };

        self.floor_db = if db < self.floor_db {
            db
        } else {
            self.floor_db + self.floor_rise_per_window
        }
        .clamp(FLOOR_MIN_DB, FLOOR_MAX_DB);

        let low = self.floor_db + FLOOR_MARGIN_DB;
        let normalized = ((db - low) / (DB_MAX - low)).clamp(0.0, 1.0);
        Some(normalized.powf(CURVE_POWER))
    }

    /// Clears the partial window. The learned floor is kept: the next
    /// recording is most likely in the same room on the same microphone.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(samples: usize, amplitude: f32) -> Vec<f32> {
        (0..samples)
            .map(|i| amplitude * (i as f32 * 0.1).sin())
            .collect()
    }

    #[test]
    fn no_reading_until_a_full_window() {
        let mut meter = LevelMeter::new(48_000, 30);
        assert_eq!(meter.feed(&vec![0.0; 100]), None);
        assert!(meter.feed(&vec![0.0; 1_600]).is_some());
    }

    #[test]
    fn silence_reads_zero_and_full_scale_reads_one() {
        let mut meter = LevelMeter::new(48_000, 30);
        assert_eq!(meter.feed(&vec![0.0; 1_600]), Some(0.0));
        assert_eq!(meter.feed(&sine(1_600, 1.0)), Some(1.0));
    }

    /// Sine amplitude for a given RMS level in dBFS.
    fn amplitude(db: f32) -> f32 {
        10f32.powf(db / 20.0) * std::f32::consts::SQRT_2
    }

    /// A few seconds of room tone, so the floor has settled on it.
    fn warmed_up(room_db: f32) -> LevelMeter {
        let mut meter = LevelMeter::new(48_000, 30);
        for _ in 0..120 {
            meter.feed(&sine(1_600, amplitude(room_db)));
        }
        meter
    }

    #[test]
    fn room_tone_reads_near_zero() {
        for room_db in [-60.0, -50.0, -46.0] {
            let mut meter = warmed_up(room_db);
            let level = meter.feed(&sine(1_600, amplitude(room_db))).unwrap();
            assert!(level < 0.05, "room tone at {room_db} dBFS read {level}");
        }
    }

    #[test]
    fn dictation_reads_tall() {
        let mut meter = warmed_up(-50.0);
        // ~-30 dBFS RMS, the measured level of ordinary dictation.
        let level = meter.feed(&sine(1_600, amplitude(-30.0))).unwrap();
        assert!(level >= 0.6, "level was {level}");
    }

    #[test]
    fn sustained_speech_does_not_become_the_floor() {
        let mut meter = warmed_up(-50.0);
        let mut level = 0.0;
        for _ in 0..300 {
            level = meter.feed(&sine(1_600, amplitude(-30.0))).unwrap();
        }
        assert!(level >= 0.45, "level after 10 s of speech was {level}");
    }

    #[test]
    fn a_dc_offset_is_not_loudness() {
        let mut meter = LevelMeter::new(48_000, 30);
        assert_eq!(meter.feed(&vec![0.4; 1_600]), Some(0.0));
    }
}
