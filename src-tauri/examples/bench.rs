//! Model bake-off: transcribes every `<name>.wav` in a folder and compares it
//! with `<name>.txt` (the reference transcript).
//!
//! WAV files must be 16 kHz mono 16-bit PCM. Convert a recording with:
//!   afconvert -f WAVE -d LEI16@16000 -c 1 input.m4a clip.wav
//!
//! Run (from src-tauri/):
//!   cargo run --release --example bench -- <model.gguf> <clips-dir> [<model2.gguf> ...]
//!
//! Prints per-clip word error rate (WER), detected language, and timing, then
//! totals per model. Set `BENCH_LANG=vi` (or `en`) to force a language instead
//! of auto-detect.

use std::path::{Path, PathBuf};
use std::time::Instant;
use transcribe_cpp::{Model, ModelOptions, RunOptions};

fn read_wav(path: &Path) -> Vec<f32> {
    let reader = hound::WavReader::open(path).expect("open wav");
    let spec = reader.spec();
    assert!(
        spec.sample_rate == 16_000 && spec.channels == 1 && spec.bits_per_sample == 16,
        "{} must be 16 kHz mono 16-bit PCM",
        path.display()
    );
    reader
        .into_samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect()
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace())
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric() || *c == '@' || *c == '\'')
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

fn word_errors(reference: &[String], hypothesis: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=hypothesis.len()).collect();
    for (i, r) in reference.iter().enumerate() {
        let mut cur = vec![i + 1; hypothesis.len() + 1];
        for (j, h) in hypothesis.iter().enumerate() {
            let sub = prev[j] + usize::from(r != h);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[hypothesis.len()]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: bench <model.gguf> <clips-dir> [<model2.gguf> ...]");
        std::process::exit(2);
    }
    let clips_dir = PathBuf::from(&args[1]);
    let models: Vec<&String> = std::iter::once(&args[0]).chain(args.iter().skip(2)).collect();

    let mut clips: Vec<PathBuf> = std::fs::read_dir(&clips_dir)
        .expect("read clips dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wav"))
        .collect();
    clips.sort();

    transcribe_cpp::disable_logging();
    let _ = transcribe_cpp::init_backends_default();

    for model_path in models {
        println!("\n=== {model_path}");
        let load = Instant::now();
        let model = Model::load_with(model_path, &ModelOptions::default()).expect("load model");
        let mut session = model.session().expect("session");
        println!(
            "loaded in {:.2}s on {} (languages: {:?})",
            load.elapsed().as_secs_f32(),
            model.backend(),
            model.capabilities().languages.len()
        );

        // Warm-up so the first clip's timing excludes shader compilation.
        let _ = session.run(&vec![0.0; 16_000], &RunOptions::default());

        let run_options = RunOptions {
            language: std::env::var("BENCH_LANG").ok(),
            ..Default::default()
        };
        let (mut total_errors, mut total_words) = (0, 0);
        let (mut total_audio, mut total_compute) = (0.0f32, 0.0f32);
        for clip in &clips {
            let audio = read_wav(clip);
            let reference = std::fs::read_to_string(clip.with_extension("txt")).unwrap_or_default();
            let started = Instant::now();
            let result = session.run(&audio, &run_options);
            let compute = started.elapsed().as_secs_f32();
            let audio_secs = audio.len() as f32 / 16_000.0;
            match result {
                Ok(t) => {
                    let ref_words = words(&reference);
                    let errors = word_errors(&ref_words, &words(&t.text));
                    total_errors += errors;
                    total_words += ref_words.len();
                    total_audio += audio_secs;
                    total_compute += compute;
                    println!(
                        "{:<28} lang={:<6} audio={:>5.1}s time={:>5.2}s WER={:>5.1}%  {}",
                        clip.file_name().unwrap().to_string_lossy(),
                        t.language.as_deref().unwrap_or("-"),
                        audio_secs,
                        compute,
                        100.0 * errors as f32 / ref_words.len().max(1) as f32,
                        t.text
                    );
                }
                Err(e) => println!("{}: error {e}", clip.display()),
            }
        }
        println!(
            "TOTAL WER={:.1}%  audio={:.1}s compute={:.2}s  ({:.2}s per 10s of audio)",
            100.0 * total_errors as f32 / total_words.max(1) as f32,
            total_audio,
            total_compute,
            10.0 * total_compute / total_audio.max(0.001)
        );
    }
}
