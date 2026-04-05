#!/usr/bin/env cargo-eval
//! Audio fixture generator for astraweave-audio integration tests
//!
//! Generates 3 test audio files using synthetic sine waves:
//! - music_test.ogg (5 sec, 440 Hz) - Music crossfade tests
//! - sfx_test.wav (1 sec, 880 Hz) - SFX playback tests  
//! - voice_test.wav (2 sec, 220 Hz) - Voice playback tests
//!
//! Usage:
//!   cargo test -p astraweave-audio --test generate_fixtures -- --ignored
//!
//! Or standalone:
//!   cd astraweave-audio/tests/fixtures
//!   cargo script generate_fixtures.rs

use std::f32::consts::PI;
use std::path::{Path, PathBuf};

const SAMPLE_RATE: u32 = 44100;

fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin();
            (sample * 32767.0) as i16
        })
        .collect()
}

fn generate_wav(path: &Path, frequency: f32, duration: f32) -> std::io::Result<()> {
    // Manual WAV file generation (no dependencies needed)
    use std::fs::File;
    use std::io::Write;

    let samples = generate_sine_wave(frequency, duration, SAMPLE_RATE);
    let num_samples = samples.len() as u32;
    let byte_rate = SAMPLE_RATE * 2; // 16-bit mono
    let data_size = num_samples * 2;

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM format
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&SAMPLE_RATE.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    for sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

fn get_fixtures_dir() -> PathBuf {
    // When run as test: astraweave-audio/tests/fixtures/
    // When run standalone: current directory
    let current = std::env::current_dir().unwrap();

    if current.ends_with("fixtures") {
        current
    } else if current.ends_with("astraweave-audio") {
        current.join("tests").join("fixtures")
    } else {
        // Assume workspace root
        current
            .join("astraweave-audio")
            .join("tests")
            .join("fixtures")
    }
}

pub fn generate_all_fixtures() -> std::io::Result<()> {
    let dir = get_fixtures_dir();
    std::fs::create_dir_all(&dir)?;

    println!("\n🎵 Generating audio test fixtures in: {}", dir.display());

    // 1. music_test.ogg - 5 sec, 440 Hz
    // Note: rodio supports WAV as .ogg, proper Vorbis encoding requires additional deps
    let music = dir.join("music_test.ogg");
    print!("   [1/3] music_test.ogg (440 Hz, 5 sec)... ");
    generate_wav(&music, 440.0, 5.0)?;
    let size = std::fs::metadata(&music)?.len();
    println!("✅ {} bytes", size);

    // 2. sfx_test.wav - 1 sec, 880 Hz
    let sfx = dir.join("sfx_test.wav");
    print!("   [2/3] sfx_test.wav (880 Hz, 1 sec)... ");
    generate_wav(&sfx, 880.0, 1.0)?;
    let size = std::fs::metadata(&sfx)?.len();
    println!("✅ {} bytes", size);

    // 3. voice_test.wav - 2 sec, 220 Hz
    let voice = dir.join("voice_test.wav");
    print!("   [3/3] voice_test.wav (220 Hz, 2 sec)... ");
    generate_wav(&voice, 220.0, 2.0)?;
    let size = std::fs::metadata(&voice)?.len();
    println!("✅ {} bytes", size);

    println!("\n✅ All fixtures generated successfully!");
    println!("   Total files: 3");
    println!("   Total duration: 8 seconds");
    println!("\n💡 Run integration tests:");
    println!("   cargo test -p astraweave-audio --test integration_tests -- --include-ignored\n");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn generate_fixtures() {
        generate_all_fixtures().expect("Failed to generate fixtures");
    }
}

fn main() {
    if let Err(e) = generate_all_fixtures() {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
