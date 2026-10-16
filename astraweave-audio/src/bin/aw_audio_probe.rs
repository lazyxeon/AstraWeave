//! Opens the default output device and exits.
//!
//! `AudioEngine::new` runs this binary in a child process. An access
//! violation inside WASAPI kills the child and the parent falls back to the
//! null backend instead of taking the game down with it.
//!
//! Exit 0: the device opened and dropped cleanly.
//! Exit 2: `OutputStream::try_default` returned an error.

fn main() -> std::process::ExitCode {
    match rodio::OutputStream::try_default() {
        Ok((stream, handle)) => {
            drop(handle);
            drop(stream);
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("aw_audio_probe: default output device unavailable: {err}");
            std::process::ExitCode::from(2)
        }
    }
}
