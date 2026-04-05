use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
/// Material Atlas Packer - Phase 3.2
/// Packs multiple material textures into a single GPU texture atlas
/// for reduced bind group overhead and better cache locality
use wgpu;

/// Represents a material's position in the texture atlas
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    /// Material index (0-7 for 2x4 grid)
    pub index: u32,
    /// UV offset (min corner)
    pub uv_offset: [f32; 2],
    /// UV scale (width, height in 0-1 space)
    pub uv_scale: [f32; 2],
    /// Pixel position in atlas (for debugging)
    pub pixel_offset: [u32; 2],
    /// Pixel dimensions in atlas
    pub pixel_size: [u32; 2],
}

/// Material atlas configuration
#[derive(Debug, Clone, Copy)]
pub struct AtlasConfig {
    /// Total atlas size (must be power of 2)
    pub atlas_size: u32,
    /// Grid dimensions (columns × rows)
    pub grid_cols: u32,
    pub grid_rows: u32,
    /// Individual material slot size
    pub slot_size: u32,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            atlas_size: 4096, // 4K atlas
            grid_cols: 4,     // 4 columns
            grid_rows: 2,     // 2 rows = 8 total slots
            slot_size: 1024,  // 1K per material
        }
    }
}

impl AtlasConfig {
    /// Calculate region for material at given index
    pub fn region(&self, index: u32) -> AtlasRegion {
        let col = index % self.grid_cols;
        let row = index / self.grid_cols;

        let pixel_offset = [col * self.slot_size, row * self.slot_size];

        let pixel_size = [self.slot_size, self.slot_size];

        let uv_offset = [
            pixel_offset[0] as f32 / self.atlas_size as f32,
            pixel_offset[1] as f32 / self.atlas_size as f32,
        ];

        let uv_scale = [
            self.slot_size as f32 / self.atlas_size as f32,
            self.slot_size as f32 / self.atlas_size as f32,
        ];

        AtlasRegion {
            index,
            uv_offset,
            uv_scale,
            pixel_offset,
            pixel_size,
        }
    }

    /// Total number of available slots
    pub fn total_slots(&self) -> u32 {
        self.grid_cols * self.grid_rows
    }
}

/// Material atlas builder
pub struct AtlasBuilder {
    config: AtlasConfig,
    atlas_data: Vec<u8>,
}

impl AtlasBuilder {
    /// Create new atlas builder with configuration
    pub fn new(config: AtlasConfig) -> Self {
        let total_pixels = (config.atlas_size * config.atlas_size) as usize;
        let atlas_data = vec![0u8; total_pixels * 4]; // RGBA

        println!(
            "📦 Creating material atlas: {}×{} ({} slots)",
            config.atlas_size,
            config.atlas_size,
            config.total_slots()
        );

        Self { config, atlas_data }
    }

    /// Add a material texture to the atlas at given index
    pub fn add_material(
        &mut self,
        index: u32,
        texture_data: &[u8],
        width: u32,
        height: u32,
    ) -> AtlasRegion {
        if index >= self.config.total_slots() {
            panic!(
                "Material index {} exceeds atlas capacity {}",
                index,
                self.config.total_slots()
            );
        }

        let region = self.config.region(index);

        println!(
            "  📌 Packing material {} at ({}, {}) -> UV ({:.3}, {:.3}) scale ({:.3}, {:.3})",
            index,
            region.pixel_offset[0],
            region.pixel_offset[1],
            region.uv_offset[0],
            region.uv_offset[1],
            region.uv_scale[0],
            region.uv_scale[1]
        );

        // Resize texture to slot size if needed
        let resized = if width != self.config.slot_size || height != self.config.slot_size {
            let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, texture_data.to_vec())
                .expect("Failed to create image from texture data");
            let resized = image::imageops::resize(
                &img,
                self.config.slot_size,
                self.config.slot_size,
                image::imageops::FilterType::Lanczos3,
            );
            resized.into_raw()
        } else {
            texture_data.to_vec()
        };

        // Copy texture data into atlas
        for y in 0..self.config.slot_size {
            for x in 0..self.config.slot_size {
                let src_idx = ((y * self.config.slot_size + x) * 4) as usize;
                let dst_x = region.pixel_offset[0] + x;
                let dst_y = region.pixel_offset[1] + y;
                let dst_idx = ((dst_y * self.config.atlas_size + dst_x) * 4) as usize;

                if src_idx + 4 <= resized.len() && dst_idx + 4 <= self.atlas_data.len() {
                    self.atlas_data[dst_idx..dst_idx + 4]
                        .copy_from_slice(&resized[src_idx..src_idx + 4]);
                }
            }
        }

        region
    }

    /// Finalize atlas and create GPU texture
    pub fn build(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> (wgpu::Texture, Vec<AtlasRegion>) {
        println!("✅ Finalizing atlas '{}'", label);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.config.atlas_size,
                height: self.config.atlas_size,
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
            &self.atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.config.atlas_size),
                rows_per_image: Some(self.config.atlas_size),
            },
            wgpu::Extent3d {
                width: self.config.atlas_size,
                height: self.config.atlas_size,
                depth_or_array_layers: 1,
            },
        );

        // Generate region info for all slots
        let regions: Vec<AtlasRegion> = (0..self.config.total_slots())
            .map(|i| self.config.region(i))
            .collect();

        (texture, regions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_config_default() {
        let config = AtlasConfig::default();
        assert_eq!(config.atlas_size, 4096);
        assert_eq!(config.grid_cols, 2);
        assert_eq!(config.grid_rows, 4);
        assert_eq!(config.slot_size, 2048);
        assert_eq!(config.total_slots(), 8);
    }

    #[test]
    fn test_region_calculation() {
        let config = AtlasConfig::default();

        // Top-left slot (index 0)
        let r0 = config.region(0);
        assert_eq!(r0.pixel_offset, [0, 0]);
        assert_eq!(r0.uv_offset, [0.0, 0.0]);
        assert_eq!(r0.uv_scale, [0.5, 0.5]);

        // Top-right slot (index 1)
        let r1 = config.region(1);
        assert_eq!(r1.pixel_offset, [2048, 0]);
        assert_eq!(r1.uv_offset, [0.5, 0.0]);

        // Second row, left (index 2)
        let r2 = config.region(2);
        assert_eq!(r2.pixel_offset, [0, 2048]);
        assert_eq!(r2.uv_offset, [0.0, 0.5]);
    }
}
