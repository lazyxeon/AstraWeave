use anyhow::Result;
#[cfg(feature = "textures")]
use image::{DynamicImage, GenericImageView};
// #[cfg(feature = "textures")]
#[cfg(feature = "textures")]
use std::path::Path;

/// Read an image file without exceeding [`astraweave_asset::image_limits::MAX_IMAGE_ALLOC`].
///
/// A regular file longer than the cap is rejected from its metadata, before
/// any byte is read. A regular file is then read only up to that length.
/// A fifo or other non-regular file is read with `take(cap)` and stopped
/// there: probing one byte past the cap would block if the writer stalled
/// after exactly that many bytes.
pub(crate) async fn read_capped_image(path: &str) -> Result<Vec<u8>, String> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || {
        astraweave_asset::read_limited_image_bytes(std::path::Path::new(&path))
            .map_err(|err| format!("{err:#}"))
    })
    .await
    .map_err(|err| format!("read failed: {err}"))?
}

/// Defines how a texture should be interpreted for correct color space handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureUsage {
    /// Color/albedo textures - stored in sRGB, GPU converts to linear
    Albedo,
    /// Normal maps - stored in linear space, no gamma correction
    Normal,
    /// Metallic/Roughness/AO packed texture - linear space
    MRA,
    /// Emissive/glow textures - sRGB space
    Emissive,
    /// Height/displacement maps - linear space
    Height,
}

impl TextureUsage {
    /// Returns the correct wgpu texture format for this usage
    pub fn format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Albedo | Self::Emissive => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Normal | Self::MRA | Self::Height => wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    /// Returns whether this texture type should have mipmaps generated
    pub fn needs_mipmaps(&self) -> bool {
        match self {
            Self::Albedo | Self::Emissive | Self::MRA => true,
            Self::Normal | Self::Height => false, // Normal maps can have artifacts with mip blending
        }
    }

    /// Returns a human-readable description of this usage
    pub fn description(&self) -> &'static str {
        match self {
            Self::Albedo => "Albedo (sRGB color)",
            Self::Normal => "Normal Map (linear RGB)",
            Self::MRA => "Metallic/Roughness/AO (linear)",
            Self::Emissive => "Emissive (sRGB color)",
            Self::Height => "Height/Displacement (linear)",
        }
    }
}

/// A loaded texture with its GPU resources
#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    /// Create a 1x1 white texture as a default/fallback
    pub fn create_default_white(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> Result<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255], // RGBA white
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    /// Create a 1x1 normal map texture pointing upward (0, 0, 1)
    pub fn create_default_normal(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> Result<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Normal maps are linear, not sRGB
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[128, 128, 255, 255], // Normal pointing up: (0, 0, 1) in normal map encoding
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    /// Load a texture from a file (requires "textures" feature)
    #[cfg(feature = "textures")]
    pub fn from_file(device: &wgpu::Device, queue: &wgpu::Queue, path: &Path) -> Result<Self> {
        Self::from_file_with_usage(device, queue, path, TextureUsage::Albedo)
    }

    /// Load a texture from a file with specific usage (requires "textures" feature)
    #[cfg(feature = "textures")]
    pub fn from_file_with_usage(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        usage: TextureUsage,
    ) -> Result<Self> {
        log::info!("Loading texture from: {}", path.display());

        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Texture file not found: {}",
                path.display()
            ));
        }

        let bytes = astraweave_asset::read_limited_image_bytes(path)?;
        Self::from_bytes_with_usage(device, queue, &bytes, &path.to_string_lossy(), usage)
    }

    /// Load a texture from byte data (requires "textures" feature)
    /// Uses Albedo (sRGB) format by default
    #[cfg(feature = "textures")]
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        Self::from_bytes_with_usage(device, queue, bytes, label, TextureUsage::Albedo)
    }

    /// Load a texture from byte data with specific usage (requires "textures" feature)
    #[cfg(feature = "textures")]
    pub fn from_bytes_with_usage(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        usage: TextureUsage,
    ) -> Result<Self> {
        let img = astraweave_asset::decode_limited_image(bytes)?;
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();

        // Calculate mip levels
        let mip_levels = if usage.needs_mipmaps() {
            calculate_mip_levels(width, height)
        } else {
            1
        };

        log::info!(
            "Loaded texture '{}': {}x{} pixels, {} as {}, {} mip levels",
            label,
            width,
            height,
            usage.description(),
            if usage.format() == wgpu::TextureFormat::Rgba8UnormSrgb {
                "sRGB"
            } else {
                "Linear"
            },
            mip_levels
        );

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: usage.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload base mip level (level 0)
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        // Generate and upload mipmaps
        if mip_levels > 1 {
            generate_and_upload_mipmaps(device, queue, &texture, &img, mip_levels)?;
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: if mip_levels > 1 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    /// Load a texture asynchronously from a file path
    ///
    /// This method performs I/O and image decoding off the main thread,
    /// only touching the GPU on the final upload step.
    ///
    /// # Arguments
    /// * `device` - WGPU device
    /// * `queue` - WGPU queue
    /// * `path` - Path to texture file
    /// * `usage` - Texture usage (Albedo, Normal, etc.)
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_render::texture::{Texture, TextureUsage};
    /// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<()> {
    /// let texture = Texture::load_texture_async(
    ///     device,
    ///     queue,
    ///     "assets/albedo.png",
    ///     TextureUsage::Albedo
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "textures")]
    pub async fn load_texture_async(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        usage: TextureUsage,
    ) -> Result<Self> {
        // Phase 1: Async file I/O (off main thread). Same 64 MiB cap as
        // `texture_streaming::decode_texture`: length is checked before the
        // read, and a non-regular file is not probed one byte past the cap.
        let bytes = read_capped_image(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read texture file {path}: {e}"))?;

        // Phase 2: Async image decoding (on thread pool)
        let label = path.to_string();
        let img = tokio::task::spawn_blocking(move || {
            astraweave_asset::decode_limited_image(&bytes)
                .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Image decode task panicked: {}", e))??;

        // Phase 3: Synchronous GPU upload (main thread)
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();

        let mip_levels = if usage.needs_mipmaps() {
            calculate_mip_levels(width, height)
        } else {
            1
        };

        log::debug!(
            "Async loaded texture '{}': {}x{} pixels, {} as {}, {} mip levels",
            label,
            width,
            height,
            usage.description(),
            if usage.format() == wgpu::TextureFormat::Rgba8UnormSrgb {
                "sRGB"
            } else {
                "Linear"
            },
            mip_levels
        );

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label),
            size,
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: usage.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload base mip level
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        // Generate and upload mipmaps
        if mip_levels > 1 {
            generate_and_upload_mipmaps(device, queue, &texture, &img, mip_levels)?;
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: if mip_levels > 1 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    /// Load a texture from already-decoded image data with specific usage
    ///
    /// This is useful when you have image data from another source (e.g., procedural generation).
    #[cfg(feature = "textures")]
    pub fn from_image_with_usage(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &DynamicImage,
        usage: TextureUsage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();

        let mip_levels = if usage.needs_mipmaps() {
            calculate_mip_levels(width, height)
        } else {
            1
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: usage.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        if mip_levels > 1 {
            generate_and_upload_mipmaps(device, queue, &texture, img, mip_levels)?;
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: if mip_levels > 1 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

/// Calculate the number of mip levels for a texture
#[cfg(feature = "textures")]
fn calculate_mip_levels(width: u32, height: u32) -> u32 {
    let max_dimension = width.max(height) as f32;
    (max_dimension.log2().floor() as u32 + 1).max(1)
}

/// Generate mipmaps using CPU downsampling and upload to GPU
#[cfg(feature = "textures")]
fn generate_and_upload_mipmaps(
    _device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    base_image: &DynamicImage,
    mip_levels: u32,
) -> Result<()> {
    let mut current_image = base_image.clone();
    let (base_width, base_height) = base_image.dimensions();

    for level in 1..mip_levels {
        // Calculate mip dimensions
        let mip_width = (base_width >> level).max(1);
        let mip_height = (base_height >> level).max(1);

        // Downsample using high-quality filter
        current_image =
            current_image.resize(mip_width, mip_height, image::imageops::FilterType::Lanczos3);

        let rgba_mip = current_image.to_rgba8();

        // Upload mip level
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_mip,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * mip_width),
                rows_per_image: Some(mip_height),
            },
            wgpu::Extent3d {
                width: mip_width,
                height: mip_height,
                depth_or_array_layers: 1,
            },
        );
    }

    Ok(())
}

/// Validate that texture files exist and can be loaded
#[cfg(feature = "textures")]
pub fn validate_texture_assets(asset_paths: &[&str]) -> Result<()> {
    log::info!("🎨 Validating texture assets...");

    let mut valid_count = 0;

    for texture_path in asset_paths {
        if std::path::Path::new(texture_path).exists() {
            match astraweave_asset::open_limited_image(texture_path) {
                Ok(img) => {
                    let (w, h) = img.dimensions();
                    log::info!("  ✅ {}: {}x{} pixels", texture_path, w, h);
                    valid_count += 1;
                }
                Err(e) => {
                    log::info!("  ❌ {}: Failed to load - {}", texture_path, e);
                }
            }
        } else {
            log::info!("  ❌ {}: File not found", texture_path);
        }
    }

    log::info!(
        "📊 Texture validation: {}/{} textures valid",
        valid_count,
        asset_paths.len()
    );

    if valid_count > 0 {
        log::info!("✅ Found valid textures for rendering!");
        Ok(())
    } else {
        Err(anyhow::anyhow!("No valid textures found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        crate::gpu_capability::request_headless(
            "test_device",
            wgpu::Features::empty(),
            wgpu::Limits::downlevel_defaults(),
            wgpu::PowerPreference::default(),
            true,
        )
        .await
        .map(|gpu| (gpu.device, gpu.queue))
    }

    #[test]
    fn test_create_default_white() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };
            let result = Texture::create_default_white(&device, &queue, "test_white");

            assert!(result.is_ok(), "Should create white texture successfully");
            let texture = result.unwrap();

            // Verify texture properties
            assert_eq!(texture.texture.size().width, 1);
            assert_eq!(texture.texture.size().height, 1);
            assert_eq!(
                texture.texture.format(),
                wgpu::TextureFormat::Rgba8UnormSrgb
            );
        });
    }

    #[test]
    fn test_create_default_normal() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };
            let result = Texture::create_default_normal(&device, &queue, "test_normal");

            assert!(result.is_ok(), "Should create normal texture successfully");
            let texture = result.unwrap();

            // Verify texture properties
            assert_eq!(texture.texture.size().width, 1);
            assert_eq!(texture.texture.size().height, 1);
            assert_eq!(
                texture.texture.format(),
                wgpu::TextureFormat::Rgba8Unorm // Normal maps use linear format
            );
        });
    }

    #[test]
    fn test_white_and_normal_different_labels() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            let white = Texture::create_default_white(&device, &queue, "white").unwrap();
            let normal = Texture::create_default_normal(&device, &queue, "normal").unwrap();

            // Both should be 1x1 textures
            assert_eq!(white.texture.size().width, 1);
            assert_eq!(normal.texture.size().width, 1);
        });
    }

    #[cfg(feature = "textures")]
    #[test]
    fn test_from_bytes_valid_png() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };

            // Create a simple 2x2 PNG in memory using image crate
            let mut img = image::RgbaImage::new(2, 2);
            img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
            let mut png_data = Vec::new();
            img.write_to(
                &mut std::io::Cursor::new(&mut png_data),
                image::ImageFormat::Png,
            )
            .unwrap();

            let result = Texture::from_bytes(&device, &queue, &png_data, "test_png");
            assert!(result.is_ok(), "Should load PNG from bytes");
        });
    }

    #[cfg(feature = "textures")]
    #[test]
    fn test_from_bytes_invalid_data() {
        pollster::block_on(async {
            let Some((device, queue)) = create_test_device().await else {
                return;
            };
            let invalid_data = vec![0, 1, 2, 3, 4, 5]; // Not a valid image

            let result = Texture::from_bytes(&device, &queue, &invalid_data, "invalid");
            assert!(result.is_err(), "Should fail on invalid image data");
        });
    }

    #[cfg(feature = "textures")]
    #[test]
    fn test_validate_texture_assets_empty() {
        let result = validate_texture_assets(&[]);
        assert!(result.is_err(), "Empty asset list should fail validation");
    }

    #[cfg(feature = "textures")]
    #[test]
    fn test_validate_texture_assets_nonexistent() {
        let result = validate_texture_assets(&["nonexistent_file.png"]);
        assert!(result.is_err(), "Nonexistent file should fail validation");
    }
}
