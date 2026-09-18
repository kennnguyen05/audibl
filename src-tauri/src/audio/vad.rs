//! Voice activity detection: Silero VAD wrapped in onset/hangover smoothing.
//! Ported from Handy (`audio_toolkit/vad/{mod,silero,smoothed}.rs`, MIT),
//! trimmed to the offline profile Audibl uses.

use anyhow::Result;
use std::collections::VecDeque;
use std::path::Path;
use vad_rs::Vad;

pub const SAMPLE_RATE: u32 = 16_000;
const SILERO_FRAME_MS: u32 = 30;
pub const SILERO_FRAME_SAMPLES: usize = (SAMPLE_RATE * SILERO_FRAME_MS / 1000) as usize;

const PREFILL_MS: u64 = 450;
const HANGOVER_MS: u64 = 450;
const ONSET_MS: u64 = 60;
pub const SILERO_THRESHOLD: f32 = 0.3;

/// Whole detector frames for a duration, rounded up so padding is never shortened.
pub const fn frames_for_duration_ms(duration_ms: u64, frame_samples: usize) -> usize {
    let numerator = duration_ms * SAMPLE_RATE as u64;
    let denominator = frame_samples as u64 * 1000;
    numerator.div_ceil(denominator) as usize
}

pub enum VadFrame<'a> {
    /// Speech; may aggregate several frames (prefill + current).
    Speech(&'a [f32]),
    Noise,
}

impl VadFrame<'_> {
    pub fn is_speech(&self) -> bool {
        matches!(self, VadFrame::Speech(_))
    }
}

pub trait VoiceActivityDetector: Send + Sync {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>>;
    fn frame_samples(&self) -> usize;
    fn is_voice(&mut self, frame: &[f32]) -> Result<bool> {
        Ok(self.push_frame(frame)?.is_speech())
    }
    fn reset(&mut self) {}
}

pub struct SileroVad {
    engine: Vad,
    threshold: f32,
}

impl SileroVad {
    pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self> {
        Ok(Self {
            engine: Vad::new(&model_path, SAMPLE_RATE as usize)
                .map_err(|e| anyhow::anyhow!("Failed to create VAD: {e}"))?,
            threshold,
        })
    }
}

impl VoiceActivityDetector for SileroVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        if frame.len() != SILERO_FRAME_SAMPLES {
            anyhow::bail!(
                "expected {SILERO_FRAME_SAMPLES} samples, got {}",
                frame.len()
            );
        }
        let result = self
            .engine
            .compute(frame)
            .map_err(|e| anyhow::anyhow!("Silero VAD error: {e}"))?;
        if result.prob > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }

    fn frame_samples(&self) -> usize {
        SILERO_FRAME_SAMPLES
    }

    fn reset(&mut self) {
        // A new recording must not inherit the previous LSTM state.
        self.engine.reset();
    }
}

/// Requires a few voiced frames before speech starts (onset), prepends the
/// frames buffered just before it (prefill), and keeps a short tail after it
/// ends (hangover), so words are not clipped at either edge.
pub struct SmoothedVad {
    inner: Box<dyn VoiceActivityDetector>,
    prefill_frames: usize,
    hangover_frames: usize,
    onset_frames: usize,
    frame_buffer: VecDeque<Vec<f32>>,
    hangover_counter: usize,
    onset_counter: usize,
    in_speech: bool,
    temp_out: Vec<f32>,
}

impl SmoothedVad {
    pub fn new(
        inner: Box<dyn VoiceActivityDetector>,
        prefill_frames: usize,
        hangover_frames: usize,
        onset_frames: usize,
    ) -> Self {
        Self {
            inner,
            prefill_frames,
            hangover_frames,
            onset_frames,
            frame_buffer: VecDeque::new(),
            hangover_counter: 0,
            onset_counter: 0,
            in_speech: false,
            temp_out: Vec::new(),
        }
    }

    /// Audibl's profile: 450 ms prefill, 450 ms hangover, 60 ms onset.
    pub fn with_default_timing(inner: Box<dyn VoiceActivityDetector>) -> Self {
        let frame = inner.frame_samples();
        Self::new(
            inner,
            frames_for_duration_ms(PREFILL_MS, frame),
            frames_for_duration_ms(HANGOVER_MS, frame),
            frames_for_duration_ms(ONSET_MS, frame),
        )
    }
}

impl VoiceActivityDetector for SmoothedVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        self.frame_buffer.push_back(frame.to_vec());
        while self.frame_buffer.len() > self.prefill_frames + 1 {
            self.frame_buffer.pop_front();
        }

        let is_voice = self.inner.is_voice(frame)?;
        match (self.in_speech, is_voice) {
            (false, true) => {
                self.onset_counter += 1;
                if self.onset_counter >= self.onset_frames {
                    self.in_speech = true;
                    self.hangover_counter = self.hangover_frames;
                    self.onset_counter = 0;
                    self.temp_out.clear();
                    for buffered in &self.frame_buffer {
                        self.temp_out.extend(buffered.iter());
                    }
                    Ok(VadFrame::Speech(&self.temp_out))
                } else {
                    Ok(VadFrame::Noise)
                }
            }
            (true, true) => {
                self.hangover_counter = self.hangover_frames;
                Ok(VadFrame::Speech(frame))
            }
            (true, false) => {
                if self.hangover_counter > 0 {
                    self.hangover_counter -= 1;
                    Ok(VadFrame::Speech(frame))
                } else {
                    self.in_speech = false;
                    Ok(VadFrame::Noise)
                }
            }
            (false, false) => {
                self.onset_counter = 0;
                Ok(VadFrame::Noise)
            }
        }
    }

    fn frame_samples(&self) -> usize {
        self.inner.frame_samples()
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.frame_buffer.clear();
        self.hangover_counter = 0;
        self.onset_counter = 0;
        self.in_speech = false;
        self.temp_out.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScriptedVad(VecDeque<bool>);

    impl VoiceActivityDetector for ScriptedVad {
        fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
            if self.0.pop_front().unwrap_or(false) {
                Ok(VadFrame::Speech(frame))
            } else {
                Ok(VadFrame::Noise)
            }
        }
        fn frame_samples(&self) -> usize {
            4
        }
    }

    fn run(script: &[bool]) -> Vec<bool> {
        let mut vad = SmoothedVad::new(
            Box::new(ScriptedVad(script.iter().copied().collect())),
            1,
            1,
            2,
        );
        script
            .iter()
            .map(|_| vad.push_frame(&[0.0; 4]).unwrap().is_speech())
            .collect()
    }

    #[test]
    fn timing_matches_silero_frames() {
        assert_eq!(frames_for_duration_ms(PREFILL_MS, SILERO_FRAME_SAMPLES), 15);
        assert_eq!(frames_for_duration_ms(ONSET_MS, SILERO_FRAME_SAMPLES), 2);
    }

    #[test]
    fn single_voiced_frame_is_ignored() {
        assert_eq!(run(&[false, true, false]), vec![false, false, false]);
    }

    #[test]
    fn speech_keeps_hangover_tail() {
        assert_eq!(
            run(&[true, true, true, false, false]),
            vec![false, true, true, true, false]
        );
    }
}
