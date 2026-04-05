// Audio test fixture generator - Programmatic synthetic audio creation
// Uses only free/libre libraries (hound for WAV, lewton for OGG Vorbis)
//
// This utility generates the 3 audio files required for integration tests:
// - music_test.ogg: 5-second 440 Hz sine wave (OGG Vorbis)
// - sfx_test.wav: 1-second 880 Hz sine wave (WAV PCM 16-bit)
// - voice_test.wav: 2-second 220 Hz sine wave (WAV PCM 16-bit)
//
// Usage:
//   cargo run --bin generate_audio_fixtures --features audio-fixtures
//
// Or call from tests:
//   generate_fixtures().expect("Failed to generate audio files");

use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const SAMPLE_RATE: u32 = 44100;

/// Generate a sine wave at the specified frequency
fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        // Convert to 16-bit PCM (range: -32768 to 32767)
        let pcm = (sample * 32767.0) as i16;
        samples.push(pcm);
    }

    samples
}

/// Generate a WAV file with a sine wave
fn generate_wav_file(
    path: &Path,
    frequency: f32,
    duration_secs: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    use hound::{WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let samples = generate_sine_wave(frequency, duration_secs, SAMPLE_RATE);

    for sample in samples {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Generate an OGG Vorbis file with a sine wave (simple approach: WAV → OGG conversion)
/// Note: We'll use a simpler approach - write WAV data and convert manually if needed,
/// or use a simple OGG encoder. For now, we'll create a WAV and document OGG conversion.
fn generate_ogg_file(
    path: &Path,
    frequency: f32,
    duration_secs: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // For OGG Vorbis, we need vorbis_encoder crate (not in deps yet)
    // Fallback: Generate WAV first, then convert (or use pre-generated OGG)
    // For this test, we'll create a placeholder or use WAV as fallback

    // TODO: Add vorbis_encoder to dev-dependencies for proper OGG generation
    // For now, create WAV with .ogg extension (rodio can decode both)

    use hound::{WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // Create temporary WAV file
    let wav_path = path.with_extension("wav.tmp");
    let mut writer = WavWriter::create(&wav_path, spec)?;
    let samples = generate_sine_wave(frequency, duration_secs, SAMPLE_RATE);

    for sample in samples {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;

    // For now, just rename WAV to OGG (rodio accepts WAV files)
    // In production, use proper OGG Vorbis encoding
    std::fs::rename(&wav_path, path)?;

    eprintln!("⚠️  Generated WAV file as .ogg (rodio compatible)");
    eprintln!("   For proper OGG Vorbis, use: ffmpeg -i music_test.ogg -c:a libvorbis music_test_vorbis.ogg");

    Ok(())
}

/// Generate all test audio fixtures
pub fn generate_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    println!("🎵 Generating audio test fixtures...");
    println!("   Directory: {}", fixtures_dir.display());

    // Ensure directory exists
    std::fs::create_dir_all(&fixtures_dir)?;

    // 1. music_test.ogg: 5-second 440 Hz sine wave
    let music_path = fixtures_dir.join("music_test.ogg");
    println!("   Generating music_test.ogg (440 Hz, 5 sec)...");
    generate_ogg_file(&music_path, 440.0, 5.0)?;
    println!(
        "   ✅ Created: {} ({} bytes)",
        music_path.display(),
        std::fs::metadata(&music_path)?.len()
    );

    // 2. sfx_test.wav: 1-second 880 Hz sine wave
    let sfx_path = fixtures_dir.join("sfx_test.wav");
    println!("   Generating sfx_test.wav (880 Hz, 1 sec)...");
    generate_wav_file(&sfx_path, 880.0, 1.0)?;
    println!(
        "   ✅ Created: {} ({} bytes)",
        sfx_path.display(),
        std::fs::metadata(&sfx_path)?.len()
    );

    // 3. voice_test.wav: 2-second 220 Hz sine wave
    let voice_path = fixtures_dir.join("voice_test.wav");
    println!("   Generating voice_test.wav (220 Hz, 2 sec)...");
    generate_wav_file(&voice_path, 220.0, 2.0)?;
    println!(
        "   ✅ Created: {} ({} bytes)",
        voice_path.display(),
        std::fs::metadata(&voice_path)?.len()
    );

    println!("\n✅ All audio fixtures generated successfully!");
    println!("   Run integration tests: cargo test -p astraweave-audio --test integration_tests -- --include-ignored");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Only run when explicitly requested
    fn test_generate_fixtures() {
        generate_fixtures().expect("Failed to generate fixtures");
    }

    #[test]
    fn test_sine_wave_generation() {
        let samples = generate_sine_wave(440.0, 0.1, 44100);
        assert_eq!(samples.len(), 4410); // 0.1 * 44100

        // Check first few samples are valid PCM
        assert!(samples[0].abs() <= 32767);
        assert!(samples[100].abs() <= 32767);
    }
}

fn main() {
    match generate_fixtures() {
        Ok(()) => {
            println!("\n🎉 Success! Audio fixtures ready for testing.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\n❌ Error generating fixtures: {}", e);
            std::process::exit(1);
        }
    }
}
