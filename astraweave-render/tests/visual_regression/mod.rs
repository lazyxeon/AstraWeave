//! Visual Regression Test Framework
//!
//! Provides utilities for rendering to buffers and comparing against golden images.

#[path = "../support/gpu_capability.rs"]
mod gpu_capability;

pub struct VisualTestContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub width: u32,
    pub height: u32,
}

impl VisualTestContext {
    /// Create a new visual test context with specified dimensions
    pub async fn new(width: u32, height: u32) -> Option<Self> {
        let gpu = gpu_capability::request_headless(
            "Visual Test Device",
            wgpu::Features::empty(),
            wgpu::Limits::default(),
            wgpu::PowerPreference::default(),
            false,
        )
        .await?;

        Some(Self {
            device: gpu.device,
            queue: gpu.queue,
            width,
            height,
        })
    }

    /// Render using the provided render function and return the pixel buffer
    pub fn render_to_buffer<F>(&self, render_fn: F) -> Vec<u8>
    where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView, u32, u32),
    {
        // Create a texture to render into
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Visual Test Render Target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Execute the render function
        render_fn(&self.device, &self.queue, &view, self.width, self.height);

        // Read back the pixels
        let buffer_size = (self.width * self.height * 4) as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visual Test Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Visual Test Copy Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 4),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        // Map and read the buffer
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        receiver.recv().unwrap().expect("Failed to map buffer");

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        buffer.unmap();

        result
    }

    /// Assert that the rendered image matches the golden image within a tolerance
    pub fn assert_image_matches(&self, actual: &[u8], golden_path: &str, tolerance: u8) {
        // Try to load golden image
        let golden_data = match std::fs::read(golden_path) {
            Ok(data) => {
                // Parse PNG
                let decoder = png::Decoder::new(std::io::Cursor::new(&data));
                let mut reader = decoder.read_info().expect("Failed to read PNG info");
                let mut buf = vec![0; reader.output_buffer_size().unwrap()];
                let info = reader.next_frame(&mut buf).expect("Failed to decode PNG");
                buf.truncate(info.buffer_size());
                buf
            }
            Err(_) => {
                // Golden image doesn't exist - create it
                eprintln!("Golden image not found at {}, creating it...", golden_path);
                self.save_image(actual, golden_path);
                return;
            }
        };

        let (max_delta, avg_delta) = image_delta(actual, &golden_data);

        assert!(
            max_delta <= tolerance,
            "Image mismatch: max delta {} exceeds tolerance {} (avg delta: {:.2})",
            max_delta,
            tolerance,
            avg_delta
        );
    }

    /// Save an image to disk as PNG
    pub fn save_image(&self, data: &[u8], path: &str) {
        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let file = std::fs::File::create(path).expect("Failed to create file");
        let mut encoder = png::Encoder::new(file, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("Failed to write PNG header");
        writer
            .write_image_data(data)
            .expect("Failed to write PNG data");
    }
}

/// Calculate the delta between two images
/// Returns (max_delta, avg_delta)
fn image_delta(a: &[u8], b: &[u8]) -> (u8, f32) {
    assert_eq!(a.len(), b.len(), "Image sizes must match");

    let mut max_delta = 0u8;
    let mut sum_delta = 0u32;
    let mut count = 0u32;

    for (pixel_a, pixel_b) in a.iter().zip(b.iter()) {
        let delta = ((*pixel_a as i16 - *pixel_b as i16).unsigned_abs()) as u8;
        max_delta = max_delta.max(delta);
        sum_delta += delta as u32;
        count += 1;
    }

    let avg_delta = if count > 0 {
        sum_delta as f32 / count as f32
    } else {
        0.0
    };

    (max_delta, avg_delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_delta_identical() {
        let a = vec![255, 128, 64, 32];
        let b = vec![255, 128, 64, 32];
        let (max, avg) = image_delta(&a, &b);
        assert_eq!(max, 0);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_image_delta_different() {
        let a = vec![255, 128, 64, 32];
        let b = vec![250, 130, 60, 30];
        let (max, avg) = image_delta(&a, &b);
        assert_eq!(max, 5);
        assert!(avg > 0.0 && avg <= 5.0);
    }
}
