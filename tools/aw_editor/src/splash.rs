//! Startup splash screen with JPEG logo + cinematic MP4 video playback.
//!
//! Flow:
//! 1. **Logo phase** (~0.8s): Static logo image with animated loading bar
//! 2. **Video phase** (~8s): H.264 video decoded from MP4, displayed frame-by-frame
//! 3. **Done**: Transition to editor
//!
//! Video is decoded on a background thread using `mp4` (container) + `openh264` (H.264 codec).
//! Falls back to logo-only if video decode fails or files are missing.
//!
//! Startup can never block on the decoder: decode runs entirely off the UI
//! thread, the logo phase waits for the first frame only up to
//! `VIDEO_WAIT_CAP_SECS`, and phase 1 bails if frames stop arriving. Click or
//! press any key to skip at any point.

use anyhow::{Context as AnyhowContext, Result};
use openh264::formats::YUVSource;
use std::sync::mpsc;
use std::time::Instant;

const LOGO_PATH: &str = "assets/Astraweave_logo.jpg";
const VIDEO_PATH: &str = "assets/8-second_Cinematic_logo_opening.mp4";
const LOGO_PHASE_SECS: f32 = 0.8;
/// Hard cap on the video phase. The asset is an ~8s clip; playback normally
/// ends earlier via the stream-end check in `show()`.
const VIDEO_PHASE_SECS: f32 = 8.5;
/// Max time to keep showing the logo while waiting for the first decoded
/// frame (bounds worst-case startup delay when decode is slow or wedged).
const VIDEO_WAIT_CAP_SECS: f32 = LOGO_PHASE_SECS + 2.0;
/// Hold the final frame briefly before transitioning to the editor.
const LAST_FRAME_HOLD_SECS: f32 = 0.25;

/// A decoded video frame sent from the background thread.
struct VideoFrame {
    width: usize,
    height: usize,
    rgb_data: Vec<u8>,
    timestamp_secs: f32,
}

pub struct SplashScreen {
    // Logo
    logo_image: Option<egui::ColorImage>,
    logo_texture: Option<egui::TextureHandle>,

    // Video
    video_rx: Option<mpsc::Receiver<VideoFrame>>,
    video_texture: Option<egui::TextureHandle>,
    current_frame_image: Option<egui::ColorImage>,
    /// A decoded frame whose timestamp is still in the future — held back so
    /// playback is timestamp-paced instead of running at UI repaint rate.
    pending_frame: Option<(f32, egui::ColorImage)>,
    /// Set when `current_frame_image` changed; `render_video` only re-uploads
    /// the texture when this is set (uploading every repaint allocates a new
    /// GPU texture per frame in egui).
    frame_dirty: bool,
    video_available: bool,

    // State
    phase: u8, // 0=logo, 1=video, 2=done
    start_time: Instant,
    video_start_time: Option<Instant>,
    /// Timestamp of the most recently received frame — used to end the video
    /// phase shortly after the stream finishes instead of waiting out the cap.
    last_frame_ts: Option<f32>,

    _decoder_thread: Option<std::thread::JoinHandle<()>>,
}

impl SplashScreen {
    pub fn new() -> Self {
        let logo_image = load_logo_image();

        // Start background video decode immediately so frames buffer during
        // the logo phase. OpenH264 init can be slow on first use, which is
        // why decode runs entirely off the UI thread — the phase-0 gate in
        // show() waits (bounded by VIDEO_WAIT_CAP_SECS) for the first frame
        // and falls back to a logo-only splash on failure.
        let (tx, rx) = mpsc::sync_channel::<VideoFrame>(10);
        let decoder_thread = match std::thread::Builder::new()
            .name("splash-video-decode".into())
            .spawn(move || {
                if let Err(e) = decode_video_frames(tx) {
                    tracing::warn!("Splash video decode failed: {e:#}");
                }
            }) {
            Ok(handle) => Some(handle),
            Err(e) => {
                // tx is dropped with the unspawned closure, so the receiver
                // disconnects and the splash cleanly degrades to logo-only.
                tracing::warn!("Failed to spawn splash video decode thread: {e}");
                None
            }
        };

        SplashScreen {
            logo_image,
            logo_texture: None,
            video_rx: Some(rx),
            video_texture: None,
            current_frame_image: None,
            pending_frame: None,
            frame_dirty: false,
            video_available: decoder_thread.is_some(),
            phase: 0,
            start_time: Instant::now(),
            video_start_time: None,
            last_frame_ts: None,
            _decoder_thread: decoder_thread,
        }
    }

    /// Render the splash screen. Returns `true` while active, `false` when done.
    /// Click anywhere or press any key to skip.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        // Skip on click or key press
        let skip = ctx.input(|i| i.pointer.any_pressed() || i.keys_down.iter().next().is_some());
        if skip {
            self.phase = 2;
            self.cleanup();
            return false;
        }

        match self.phase {
            0 => {
                let elapsed = self.start_time.elapsed().as_secs_f32();
                self.render_logo(ctx, elapsed);
                if elapsed >= LOGO_PHASE_SECS {
                    if !self.video_available {
                        // Decode failed or never started — logo-only splash.
                        self.phase = 2;
                        self.cleanup();
                        return false;
                    }
                    if self.current_frame_image.is_some() {
                        // First video frame is buffered — start playback.
                        self.phase = 1;
                        self.video_start_time = Some(Instant::now());
                    } else if elapsed >= VIDEO_WAIT_CAP_SECS {
                        // Decoder produced nothing within the wait cap —
                        // don't hold the editor hostage, skip the video.
                        tracing::debug!(
                            "Splash video produced no frame within {VIDEO_WAIT_CAP_SECS}s; skipping video phase"
                        );
                        self.phase = 2;
                        self.cleanup();
                        return false;
                    }
                    // else: keep showing the logo while the decoder warms up.
                }
                ctx.request_repaint();
                true
            }
            1 => {
                let video_elapsed = self
                    .video_start_time
                    .map_or(0.0, |t| t.elapsed().as_secs_f32());

                self.advance_video_frame(video_elapsed);

                // Backstop: if frames stopped arriving before anything was
                // buffered (normally impossible — the phase-0 gate requires a
                // buffered frame), fall back to the editor.
                if video_elapsed > 0.5 && self.current_frame_image.is_none() {
                    self.phase = 2;
                    self.cleanup();
                    return false;
                }

                self.render_video(ctx, video_elapsed);

                // End when the decoded stream is exhausted (decoder thread
                // finished, no held-back frame remains, and the last frame
                // has been shown), or at the cap.
                let stream_ended = self.video_rx.is_none()
                    && self.pending_frame.is_none()
                    && video_elapsed >= self.last_frame_ts.unwrap_or(0.0) + LAST_FRAME_HOLD_SECS;
                if stream_ended || video_elapsed >= VIDEO_PHASE_SECS {
                    self.phase = 2;
                    self.cleanup();
                    return false;
                }
                ctx.request_repaint();
                true
            }
            _ => false,
        }
    }

    fn render_logo(&mut self, ctx: &egui::Context, elapsed: f32) {
        let total_duration = LOGO_PHASE_SECS + VIDEO_PHASE_SECS;
        let progress = (elapsed / total_duration).min(0.25);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(10, 10, 16)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let avail = ui.available_size();

                    if let Some(logo) = &self.logo_image {
                        if self.logo_texture.is_none() {
                            self.logo_texture = Some(ctx.load_texture(
                                "splash_logo",
                                logo.clone(),
                                egui::TextureOptions::LINEAR,
                            ));
                        }

                        if let Some(tex) = &self.logo_texture {
                            let img_w = tex.size()[0] as f32;
                            let img_h = tex.size()[1] as f32;
                            let aspect = img_w / img_h;

                            let max_w = avail.x * 0.6;
                            let max_h = avail.y * 0.55;
                            let (w, h) = fit_preserve_aspect(aspect, max_w, max_h);

                            let pad_top = (avail.y - h) / 2.0 - 40.0;
                            ui.add_space(pad_top.max(20.0));

                            // Fade-in
                            let alpha = (elapsed * 1.8).min(1.0);
                            let tint = egui::Color32::from_rgba_unmultiplied(
                                255,
                                255,
                                255,
                                (alpha * 255.0) as u8,
                            );
                            ui.add(
                                egui::Image::new(egui::load::SizedTexture::new(tex.id(), [w, h]))
                                    .tint(tint),
                            );
                        }
                    } else {
                        // Text fallback
                        ui.add_space(avail.y * 0.35);
                        ui.heading(
                            egui::RichText::new("AstraWeave Engine")
                                .size(36.0)
                                .color(egui::Color32::from_rgb(140, 170, 255))
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("AI-Native Game Editor")
                                .size(16.0)
                                .color(egui::Color32::from_rgb(160, 160, 180)),
                        );
                    }

                    // Loading bar
                    ui.add_space(35.0);
                    let bar_w = (avail.x * 0.35).min(380.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_w, 3.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 1.5, egui::Color32::from_rgb(30, 30, 44));
                    let fill =
                        egui::Rect::from_min_size(rect.min, egui::vec2(bar_w * progress, 3.0));
                    ui.painter()
                        .rect_filled(fill, 1.5, egui::Color32::from_rgb(80, 120, 255));

                    // Subtle animated text
                    ui.add_space(14.0);
                    let dot_n = ((elapsed * 2.5) as usize) % 4;
                    let dots = ".".repeat(dot_n);
                    let pad = " ".repeat(3 - dot_n);
                    ui.label(
                        egui::RichText::new(format!("Initializing{dots}{pad}"))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(80, 80, 100)),
                    );

                    // Skip hint
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Click or press any key to skip")
                            .size(10.0)
                            .color(egui::Color32::from_rgb(60, 60, 75)),
                    );
                });
            });

        // Buffer the FIRST decoded frame (also detects early decode failure
        // via channel disconnect). Only pull one frame — draining here would
        // consume the opening seconds of the video before playback starts.
        if self.current_frame_image.is_none() {
            if let Some(rx) = &self.video_rx {
                match rx.try_recv() {
                    Ok(frame) => {
                        self.last_frame_ts = Some(frame.timestamp_secs);
                        self.current_frame_image = Some(rgb8_to_color_image(
                            &frame.rgb_data,
                            frame.width,
                            frame.height,
                        ));
                        self.frame_dirty = true;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.video_available = false;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
        }
    }

    fn advance_video_frame(&mut self, video_elapsed: f32) {
        // Promote the held-back frame once its timestamp is due; while it is
        // still in the future, playback shows the current frame (this is what
        // paces playback to timestamps instead of the UI repaint rate).
        if let Some((ts, _)) = self.pending_frame {
            if ts > video_elapsed {
                return;
            }
            if let Some((ts, img)) = self.pending_frame.take() {
                self.last_frame_ts = Some(ts);
                self.current_frame_image = Some(img);
                self.frame_dirty = true;
            }
        }

        let rx = match &self.video_rx {
            Some(rx) => rx,
            None => return,
        };

        // Consume frames up to the current playback time
        loop {
            match rx.try_recv() {
                Ok(frame) => {
                    let img = rgb8_to_color_image(&frame.rgb_data, frame.width, frame.height);
                    if frame.timestamp_secs > video_elapsed {
                        // Ahead of schedule — hold it until its timestamp.
                        self.pending_frame = Some((frame.timestamp_secs, img));
                        break;
                    }
                    self.last_frame_ts = Some(frame.timestamp_secs);
                    self.current_frame_image = Some(img);
                    self.frame_dirty = true;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.video_rx = None;
                    break;
                }
            }
        }
    }

    fn render_video(&mut self, ctx: &egui::Context, _video_elapsed: f32) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                if let Some(image) = &self.current_frame_image {
                    // Upload only when the frame changed — load_texture
                    // allocates a NEW texture on every call.
                    let size = image.size;
                    if self.video_texture.is_none() || self.frame_dirty {
                        self.video_texture = Some(ctx.load_texture(
                            "splash_video_frame",
                            image.clone(),
                            egui::TextureOptions::LINEAR,
                        ));
                        self.frame_dirty = false;
                    }

                    if let Some(tex) = &self.video_texture {
                        let avail = ui.available_size();
                        let img_w = size[0] as f32;
                        let img_h = size[1] as f32;

                        if img_w > 0.0 && img_h > 0.0 {
                            let aspect = img_w / img_h;
                            // Scale to fill window
                            let (w, h) = if avail.x / aspect <= avail.y {
                                (avail.x, avail.x / aspect)
                            } else {
                                (avail.y * aspect, avail.y)
                            };

                            let pad_y = (avail.y - h) / 2.0;
                            ui.add_space(pad_y.max(0.0));
                            ui.vertical_centered(|ui| {
                                ui.image(egui::load::SizedTexture::new(tex.id(), [w, h]));
                            });
                        }
                    }
                }
            });
    }

    fn cleanup(&mut self) {
        self.video_rx = None;
        self.video_texture = None;
        self.current_frame_image = None;
        self.pending_frame = None;
        self.logo_texture = None;
        self.logo_image = None;
    }
}

fn fit_preserve_aspect(aspect: f32, max_w: f32, max_h: f32) -> (f32, f32) {
    if max_w / aspect <= max_h {
        (max_w, max_w / aspect)
    } else {
        (max_h * aspect, max_h)
    }
}

fn load_logo_image() -> Option<egui::ColorImage> {
    let data = std::fs::read(LOGO_PATH).ok()?;
    let img = astraweave_asset::decode_limited_image(&data).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let pixels = rgba
        .pixels()
        .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(egui::ColorImage::new([w, h], pixels))
}

fn rgb8_to_color_image(rgb: &[u8], width: usize, height: usize) -> egui::ColorImage {
    let pixels = (0..width * height)
        .map(|i| {
            let off = i * 3;
            if off + 2 < rgb.len() {
                egui::Color32::from_rgb(rgb[off], rgb[off + 1], rgb[off + 2])
            } else {
                egui::Color32::BLACK
            }
        })
        .collect();
    egui::ColorImage::new([width, height], pixels)
}

// ---------------------------------------------------------------------------
// Background video decoder: MP4 container + H.264 via OpenH264
// ---------------------------------------------------------------------------

fn decode_video_frames(tx: mpsc::SyncSender<VideoFrame>) -> Result<()> {
    use std::io::BufReader;

    let file = std::fs::File::open(VIDEO_PATH).context("Failed to open splash video")?;
    let size = file.metadata()?.len();
    let buf = BufReader::new(file);
    let mut reader = mp4::Mp4Reader::read_header(buf, size).context("Failed to parse MP4")?;

    // --- Extract track metadata (immutable borrow) ---
    let (track_id, sample_count, nal_length_size, sps_pps, timescale) = {
        let track = reader
            .tracks()
            .values()
            .find(|t| t.media_type().ok() == Some(mp4::MediaType::H264))
            .ok_or_else(|| anyhow::anyhow!("No H.264 video track in MP4"))?;

        let tid = track.track_id();
        let sc = track.sample_count();
        let ts = track.timescale();

        // Access AVCC decoder configuration for SPS/PPS
        let stsd = &track.trak.mdia.minf.stbl.stsd;
        let avc1 = stsd
            .avc1
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No AVC1 sample entry in track"))?;
        let avcc = &avc1.avcc;
        let nls = (avcc.length_size_minus_one + 1) as usize;

        // Build Annex B init data from SPS + PPS
        let mut init = Vec::new();
        for sps in &avcc.sequence_parameter_sets {
            init.extend_from_slice(&[0, 0, 0, 1]);
            init.extend_from_slice(&sps.bytes);
        }
        for pps in &avcc.picture_parameter_sets {
            init.extend_from_slice(&[0, 0, 0, 1]);
            init.extend_from_slice(&pps.bytes);
        }

        (tid, sc, nls, init, ts)
    };

    // --- Create H.264 decoder and feed SPS/PPS ---
    let mut decoder =
        openh264::decoder::Decoder::new().map_err(|e| anyhow::anyhow!("OpenH264 init: {e:?}"))?;
    let _ = decoder.decode(&sps_pps); // SPS/PPS don't produce frames

    // --- Decode each sample ---
    for sid in 1..=sample_count {
        let sample = reader.read_sample(track_id, sid).context("Read sample")?;
        let sample = match sample {
            Some(s) => s,
            None => continue,
        };

        let timestamp_secs = sample.start_time as f32 / timescale as f32;
        let annex_b = avcc_to_annex_b(&sample.bytes, nal_length_size);

        match decoder.decode(&annex_b) {
            Ok(Some(yuv)) => {
                let (width, height) = yuv.dimensions();
                let mut rgb = vec![0u8; width * height * 3];
                yuv.write_rgb8(&mut rgb);
                if tx
                    .send(VideoFrame {
                        width,
                        height,
                        rgb_data: rgb,
                        timestamp_secs,
                    })
                    .is_err()
                {
                    return Ok(()); // Receiver dropped, splash ended
                }
            }
            Ok(None) => {} // Decoder buffering
            Err(e) => {
                tracing::trace!("H.264 decode error on sample {sid}: {e:?}");
            }
        }
    }

    Ok(())
}

/// Convert AVCC-format NAL units (length-prefixed) to Annex B (start-code-prefixed).
fn avcc_to_annex_b(data: &[u8], nal_length_size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 128);
    let mut offset = 0;

    while offset + nal_length_size <= data.len() {
        let mut nal_len = 0usize;
        for i in 0..nal_length_size {
            nal_len = (nal_len << 8) | data[offset + i] as usize;
        }
        offset += nal_length_size;

        if nal_len == 0 || offset + nal_len > data.len() {
            break;
        }

        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(&data[offset..offset + nal_len]);
        offset += nal_len;
    }

    out
}
