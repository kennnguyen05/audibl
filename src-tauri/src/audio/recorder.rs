//! Microphone capture: cpal input stream → wait-free ring buffer → consumer
//! thread that resamples to 16 kHz mono frames, filters them through VAD, and
//! feeds the level visualizer.
//!
//! Ported from Handy (`audio_toolkit/audio/recorder.rs`, MIT). Audible drops
//! the streaming callback and the per-session VAD policy: every recording uses
//! the same VAD profile.

use std::{
    io::Error,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SizedSample,
};
use rtrb::{Consumer, Producer, RingBuffer};

use super::resampler::FrameResampler;
use super::vad::{VadFrame, VoiceActivityDetector, SAMPLE_RATE};
use super::visualizer::AudioVisualiser;

enum Cmd {
    Start,
    Stop(mpsc::Sender<Vec<f32>>),
    Shutdown,
}

const AUDIO_RING_SECONDS: usize = 2;
const CONSUMER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_DRAIN_CHUNK: Duration = Duration::from_millis(50);
const PAUSE_ACK_TIMEOUT: Duration = Duration::from_secs(2);

/// Atomics shared by the real-time callback and the consumer thread.
#[derive(Default)]
struct CaptureTransportState {
    pause_requested: AtomicBool,
    pause_acknowledged: AtomicBool,
    overrun_samples: AtomicU64,
}

pub type LevelCallback = Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>;
type SharedVad = Arc<Mutex<Box<dyn VoiceActivityDetector>>>;

pub struct AudioRecorder {
    cmd_tx: Option<mpsc::Sender<Cmd>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
    vad: SharedVad,
    level_cb: Option<LevelCallback>,
    /// Input channel to use; `None` averages all channels.
    selected_channel: Option<usize>,
    stream_error: Arc<AtomicBool>,
}

impl AudioRecorder {
    pub fn new(vad: Box<dyn VoiceActivityDetector>) -> Self {
        Self {
            cmd_tx: None,
            worker_handle: None,
            vad: Arc::new(Mutex::new(vad)),
            level_cb: None,
            selected_channel: None,
            stream_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_level_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        self.level_cb = Some(Arc::new(cb));
        self
    }

    pub fn set_selected_channel(&mut self, channel: Option<u16>) {
        self.selected_channel = channel.map(usize::from);
    }

    pub fn open(&mut self, device: Option<Device>) -> Result<(), Box<dyn std::error::Error>> {
        if self.worker_handle.is_some() {
            if !self.needs_reopen() {
                return Ok(());
            }
            log::warn!("Capture stream failed; rebuilding microphone stream");
            self.close();
        }
        self.stream_error.store(false, Ordering::Relaxed);

        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<(), String>>(1);

        let device = match device {
            Some(dev) => dev,
            None => cpal::default_host()
                .default_input_device()
                .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "No input device found"))?,
        };

        let vad = Arc::clone(&self.vad);
        let level_cb = self.level_cb.clone();
        let selected_channel = self.selected_channel;
        let stream_error = Arc::clone(&self.stream_error);

        let worker = std::thread::spawn(move || {
            let transport = Arc::new(CaptureTransportState::default());
            let init = (|| -> Result<(cpal::Stream, u32, Consumer<f32>), String> {
                let config = preferred_config(&device)
                    .map_err(|e| format!("Failed to fetch preferred config: {e}"))?;
                let sample_rate = config.sample_rate().0;
                let channels = config.channels() as usize;
                log::info!(
                    "Using device {:?}: {} Hz, {} channel(s), {:?}",
                    device.name(),
                    sample_rate,
                    channels,
                    config.sample_format()
                );

                let t = Arc::clone(&transport);
                let e = Arc::clone(&stream_error);
                let built = match config.sample_format() {
                    cpal::SampleFormat::U8 => {
                        build_stream::<u8>(&device, &config, channels, selected_channel, t, e)
                    }
                    cpal::SampleFormat::I8 => {
                        build_stream::<i8>(&device, &config, channels, selected_channel, t, e)
                    }
                    cpal::SampleFormat::I16 => {
                        build_stream::<i16>(&device, &config, channels, selected_channel, t, e)
                    }
                    cpal::SampleFormat::I32 => {
                        build_stream::<i32>(&device, &config, channels, selected_channel, t, e)
                    }
                    cpal::SampleFormat::F32 => {
                        build_stream::<f32>(&device, &config, channels, selected_channel, t, e)
                    }
                    other => return Err(format!("Unsupported sample format: {other:?}")),
                };
                let (stream, consumer) =
                    built.map_err(|e| format!("Failed to build input stream: {e}"))?;
                stream
                    .play()
                    .map_err(|e| format!("Failed to start microphone stream: {e}"))?;
                Ok((stream, sample_rate, consumer))
            })();

            match init {
                Ok((stream, sample_rate, consumer)) => {
                    let _ = init_tx.send(Ok(()));
                    let processor = CaptureProcessor::new(sample_rate, vad, level_cb);
                    run_consumer(processor, consumer, cmd_rx, transport, stream_error);
                    drop(stream);
                }
                Err(message) => {
                    log::error!("{message}");
                    let _ = init_tx.send(Err(message));
                }
            }
        });

        match init_rx.recv() {
            Ok(Ok(())) => {
                self.cmd_tx = Some(cmd_tx);
                self.worker_handle = Some(worker);
                Ok(())
            }
            Ok(Err(message)) => {
                let _ = worker.join();
                Err(Box::new(Error::other(message)))
            }
            Err(e) => {
                let _ = worker.join();
                Err(Box::new(Error::other(format!(
                    "Failed to initialize microphone worker: {e}"
                ))))
            }
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| Error::other("Recorder is not open"))?;
        tx.send(Cmd::Start)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| Error::other("Recorder is not open"))?;
        let (resp_tx, resp_rx) = mpsc::channel();
        tx.send(Cmd::Stop(resp_tx))?;
        Ok(resp_rx.recv()?)
    }

    pub fn needs_reopen(&self) -> bool {
        self.stream_error.load(Ordering::Relaxed)
            || self
                .worker_handle
                .as_ref()
                .is_some_and(|handle| handle.is_finished())
    }

    pub fn close(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(Cmd::Shutdown);
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn input_channel_count(device: &Device) -> Result<u16, Box<dyn std::error::Error>> {
    Ok(preferred_config(device)?.channels())
}

/// The device's native rate with the best sample format; the resampler
/// converts to 16 kHz, so hardware is never forced to a non-native rate.
fn preferred_config(
    device: &Device,
) -> Result<cpal::SupportedStreamConfig, Box<dyn std::error::Error>> {
    let default_config = device.default_input_config()?;
    let target_rate = default_config.sample_rate();
    let Ok(ranges) = device.supported_input_configs() else {
        return Ok(default_config);
    };
    let score = |fmt: cpal::SampleFormat| match fmt {
        cpal::SampleFormat::F32 => 4,
        cpal::SampleFormat::I16 => 3,
        cpal::SampleFormat::I32 => 2,
        _ => 1,
    };
    let best = ranges
        .filter(|r| r.min_sample_rate() <= target_rate && r.max_sample_rate() >= target_rate)
        .max_by_key(|r| score(r.sample_format()));
    Ok(best
        .map(|r| r.with_sample_rate(target_rate))
        .unwrap_or(default_config))
}

fn build_stream<T>(
    device: &Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    selected_channel: Option<usize>,
    transport: Arc<CaptureTransportState>,
    stream_error: Arc<AtomicBool>,
) -> Result<(cpal::Stream, Consumer<f32>), cpal::BuildStreamError>
where
    T: Sample + SizedSample + Copy + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let ring_capacity = config.sample_rate().0 as usize * AUDIO_RING_SECONDS;
    let (mut producer, mut consumer) = RingBuffer::new(ring_capacity);
    // Touch the ring's pages before the stream starts to reduce callback page faults.
    producer
        .write_chunk(ring_capacity)
        .expect("new ring has full capacity")
        .commit_all();
    consumer
        .read_chunk(ring_capacity)
        .expect("pre-filled ring is readable")
        .commit_all();

    let use_channel = selected_channel.filter(|&c| c < channels);
    let stream = device.build_input_stream(
        &config.clone().into(),
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            write_input_to_ring(data, channels, use_channel, &mut producer, &transport);
        },
        move |_err| {
            // The error callback may share the audio thread: only flag it.
            stream_error.store(true, Ordering::Release);
        },
        None,
    )?;
    Ok((stream, consumer))
}

/// Real-time callback body. Allocation-, lock-, and logging-free.
fn write_input_to_ring<T>(
    data: &[T],
    channels: usize,
    use_channel: Option<usize>,
    producer: &mut Producer<f32>,
    transport: &CaptureTransportState,
) where
    T: Sample + SizedSample + Copy,
    f32: cpal::FromSample<T>,
{
    if transport.pause_requested.load(Ordering::Acquire)
        && transport.pause_acknowledged.load(Ordering::Acquire)
    {
        return;
    }

    let frame_count = data.len() / channels;
    let writable = producer.slots().min(frame_count);
    let written = if writable == 0 {
        0
    } else {
        let chunk = producer
            .write_chunk_uninit(writable)
            .expect("producer reported these slots");
        if channels == 1 {
            chunk.fill_from_iter(data.iter().take(writable).map(|&s| s.to_sample::<f32>()))
        } else if let Some(channel) = use_channel {
            chunk.fill_from_iter(
                data.chunks_exact(channels)
                    .take(writable)
                    .map(|frame| frame[channel].to_sample::<f32>()),
            )
        } else {
            chunk.fill_from_iter(data.chunks_exact(channels).take(writable).map(|frame| {
                frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>() / channels as f32
            }))
        }
    };

    let dropped = frame_count - written;
    if dropped > 0 {
        transport
            .overrun_samples
            .fetch_add(dropped as u64, Ordering::Relaxed);
    }
    if transport.pause_requested.load(Ordering::Acquire) {
        transport.pause_acknowledged.store(true, Ordering::Release);
    }
}

pub fn is_no_input_device_error(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("no input device found")
        || (normalized.contains("failed to fetch preferred config")
            && normalized.contains("coreaudio"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChunkDisposition {
    Capture,
    Discard,
}

struct CaptureProcessor {
    vad: SharedVad,
    level_cb: Option<LevelCallback>,
    visualizer: AudioVisualiser,
    resampler: FrameResampler,
    max_drain_samples: usize,
    processed: Vec<f32>,
    dropped_samples: u64,
}

impl CaptureProcessor {
    fn new(in_sample_rate: u32, vad: SharedVad, level_cb: Option<LevelCallback>) -> Self {
        let frame_samples = vad.lock().unwrap().frame_samples();
        let frame_duration = Duration::from_secs_f64(frame_samples as f64 / SAMPLE_RATE as f64);
        let resampler =
            FrameResampler::new(in_sample_rate as usize, SAMPLE_RATE as usize, frame_duration);

        let target_window = (f64::from(in_sample_rate) / 30.0).round() as usize;
        let window_size = [256usize, 512, 1024, 2048]
            .into_iter()
            .min_by_key(|w| w.abs_diff(target_window))
            .unwrap();
        let visualizer = AudioVisualiser::new(in_sample_rate, window_size, 16, 400.0, 4000.0);
        let max_drain_samples =
            ((in_sample_rate as u128 * MAX_DRAIN_CHUNK.as_millis()) / 1_000).max(1) as usize;

        Self {
            vad,
            level_cb,
            visualizer,
            resampler,
            max_drain_samples,
            processed: Vec::new(),
            dropped_samples: 0,
        }
    }

    fn begin_recording(&mut self) {
        self.dropped_samples = 0;
        self.processed.clear();
        self.visualizer.reset();
        self.resampler.reset();
        self.vad.lock().unwrap().reset();
    }

    fn drain(&mut self, consumer: &mut Consumer<f32>, disposition: ChunkDisposition) -> usize {
        let available = consumer.slots().min(self.max_drain_samples);
        if available == 0 {
            return 0;
        }
        let chunk = consumer
            .read_chunk(available)
            .expect("reported slots are readable");
        let (first, second) = chunk.as_slices();
        if disposition == ChunkDisposition::Capture {
            for part in [first, second] {
                if !part.is_empty() {
                    self.process_raw(part);
                }
            }
        }
        chunk.commit_all();
        available
    }

    fn process_raw(&mut self, raw: &[f32]) {
        if let Some(levels) = self.visualizer.feed(raw) {
            if let Some(cb) = &self.level_cb {
                cb(levels);
            }
        }
        let vad = &self.vad;
        let processed = &mut self.processed;
        self.resampler
            .push(raw, |frame| handle_frame(frame, vad, processed));
    }

    fn finish_recording(&mut self) -> Vec<f32> {
        let vad = &self.vad;
        let processed = &mut self.processed;
        self.resampler
            .finish(|frame| handle_frame(frame, vad, processed));
        if self.dropped_samples > 0 {
            log::warn!(
                "Recording completed after dropping {} microphone samples",
                self.dropped_samples
            );
        }
        std::mem::take(&mut self.processed)
    }
}

fn handle_frame(samples: &[f32], vad: &SharedVad, out: &mut Vec<f32>) {
    let mut detector = vad.lock().unwrap();
    match detector
        .push_frame(samples)
        .unwrap_or(VadFrame::Speech(samples))
    {
        VadFrame::Speech(buf) => out.extend_from_slice(buf),
        VadFrame::Noise => {}
    }
}

fn run_consumer(
    mut processor: CaptureProcessor,
    mut consumer: Consumer<f32>,
    cmd_rx: mpsc::Receiver<Cmd>,
    transport: Arc<CaptureTransportState>,
    stream_error: Arc<AtomicBool>,
) {
    let mut recording = false;
    let mut error_logged = false;

    loop {
        let command = if consumer.slots() > 0 {
            match cmd_rx.try_recv() {
                Ok(c) => Some(c),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        } else {
            match cmd_rx.recv_timeout(CONSUMER_POLL_INTERVAL) {
                Ok(c) => Some(c),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        };

        match command {
            Some(Cmd::Start) => {
                transport.overrun_samples.store(0, Ordering::Release);
                processor.begin_recording();
                recording = true;
            }
            Some(Cmd::Stop(reply)) => {
                recording = false;
                // Pause the callback after one boundary block, then drain
                // everything captured before the pause into this recording.
                transport.pause_acknowledged.store(false, Ordering::Relaxed);
                transport.pause_requested.store(true, Ordering::Release);
                let started = Instant::now();
                while !transport.pause_acknowledged.load(Ordering::Acquire)
                    && started.elapsed() < PAUSE_ACK_TIMEOUT
                {
                    if processor.drain(&mut consumer, ChunkDisposition::Capture) == 0 {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
                let timed_out = !transport.pause_acknowledged.load(Ordering::Acquire);
                if timed_out {
                    log::warn!("Timed out waiting for the microphone callback to pause");
                    stream_error.store(true, Ordering::Release);
                }
                while processor.drain(&mut consumer, ChunkDisposition::Capture) > 0 {}
                processor.dropped_samples += transport.overrun_samples.swap(0, Ordering::AcqRel);
                let samples = processor.finish_recording();
                if !timed_out {
                    transport.pause_acknowledged.store(false, Ordering::Relaxed);
                    transport.pause_requested.store(false, Ordering::Release);
                }
                let _ = reply.send(samples);
                if timed_out {
                    return;
                }
                continue;
            }
            Some(Cmd::Shutdown) => {
                transport.pause_requested.store(true, Ordering::Release);
                return;
            }
            None => {}
        }

        let disposition = if recording {
            ChunkDisposition::Capture
        } else {
            ChunkDisposition::Discard
        };
        processor.drain(&mut consumer, disposition);
        let overrun = transport.overrun_samples.swap(0, Ordering::AcqRel);
        if recording {
            processor.dropped_samples += overrun;
        }
        if stream_error.load(Ordering::Acquire) && !error_logged {
            log::error!("Microphone stream error; it will be rebuilt on the next recording");
            error_logged = true;
        }
    }
}
