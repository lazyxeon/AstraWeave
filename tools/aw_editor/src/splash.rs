//! Startup splash screen with JPEG logo + cinematic MP4 video playback.
//!
//! Flow:
//! 1. **Logo phase** (~3s): Static logo image with animated loading bar
//! 2. **Video phase** (~8s): H.264 video decoded from MP4, displayed frame-by-frame
//! 3. **Done**: Transition to editor
//!
//! Video is decoded on a background thread using `mp4` (container) + `openh264` (H.264 codec).
//! Falls back to logo-only if video decode fails or files are missing.

use anyhow::{Context as AnyhowContext, Result};
use openh264::formats::YUVSource;
use std::sync::mpsc;
use std::time::Instant;

const LOGO_PATH: &str = "assets/Astraweave_logo.jpg";
const VIDEO_PATH: &str = "assets/8-second_Cinematic_logo_opening.mp4";
const LOGO_PHASE_SECS: f32 = 0.5;
const VIDEO_PHASE_SECS: f32 = 2.0;

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
    video_available: bool,

    // State
    phase: u8, // 0=logo, 1=video, 2=done
    start_time: Instant,
    video_start_time: Option<Instant>,

    _decoder_thread: Option<std::thread::JoinHandle<()>>,
}

impl SplashScreen {
    pub fn new() -> Self {
        let logo_image = load_logo_image();

        // Skip video decode entirely for fast startup — logo-only splash.
        // Video decode thread was spawning openh264 which can be slow on first use.
        SplashScreen {
            logo_image,
            logo_texture: None,
            video_rx: None,
            video_texture: None,
            current_frame_image: None,
            video_available: false,
            phase: 0,
            start_time: Instant::now(),
            video_start_time: None,
            _decoder_thread: None,
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
                    // Try to peek at the video channel to see if frames are coming
                    if !self.video_available {
                        // No video, skip to done
                        self.phase = 2;
                        self.cleanup();
                        return false;
                    }
                    self.phase = 1;
                    self.video_start_time = Some(Instant::now());
                }
                ctx.request_repaint();
                true
            }
            1 => {
                let video_elapsed = self
                    .video_start_time
                    .map_or(0.0, |t| t.elapsed().as_secs_f32());

                self.advance_video_frame(video_elapsed);

                // If we never got any frames, fall back
                if video_elapsed > 0.5 && self.current_frame_image.is_none() {
                    self.phase = 2;
                    self.cleanup();
                    return false;
                }

                self.render_video(ctx, video_elapsed);

                if video_elapsed >= VIDEO_PHASE_SECS {
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

        // Check if video channel got disconnected (decode failed early)
        if let Some(rx) = &self.video_rx {
            match rx.try_recv() {
                Ok(frame) => {
                    // Got a frame — buffer it for later
                    self.current_frame_image = Some(rgb8_to_color_image(
                        &frame.rgb_data,
                        frame.width,
                        frame.height,
                    ));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.video_available = false;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    fn advance_video_frame(&mut self, video_elapsed: f32) {
        let rx = match &self.video_rx {
            Some(rx) => rx,
            None => return,
        };

        // Consume frames up to the current playback time
        loop {
            match rx.try_recv() {
                Ok(frame) => {
                    let img = rgb8_to_color_image(&frame.rgb_data, frame.width, frame.height);
                    self.current_frame_image = Some(img);

                    // If this frame is ahead of playback time, stop consuming
                    if frame.timestamp_secs > video_elapsed {
                        break;
                    }
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
                    // Update or create video texture
                    let size = image.size;
                    self.video_texture = Some(ctx.load_texture(
                        "splash_video_frame",
                        image.clone(),
                        egui::TextureOptions::LINEAR,
                    ));

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
    let img = image::load_from_memory(&data).ok()?;
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
