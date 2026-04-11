//! Render-side implementation of [`TerrainGpuAccelerator`].
//!
//! Wraps [`GpuNoisePipeline`] and [`GpuErosionPipeline`] behind the trait
//! defined in `astraweave-terrain`, eliminating the terrain crate's need to
//! depend on `wgpu` directly.

use std::sync::Arc;

use anyhow::{Context, Result};

use astraweave_terrain::gpu_bridge::{
    GpuErosionRequest, GpuHeightmapRequest, GpuHeightmapResult, GpuNoiseRequest,
    TerrainGpuAccelerator,
};

use crate::compute_noise::{GpuNoiseConfig, GpuNoisePipeline, GpuNoiseType};
use crate::gpu_erosion::{GpuErosionConfig, GpuErosionPipeline};

/// Render-side GPU accelerator for terrain generation.
///
/// Holds a reference to the wgpu device/queue and lazily-created compute
/// pipelines. All trait methods perform synchronous GPU dispatch + readback.
pub struct WgpuTerrainAccelerator {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    noise_pipeline: GpuNoisePipeline,
    adapter_name: String,
}

impl WgpuTerrainAccelerator {
    /// Create a new accelerator from an existing wgpu device and queue.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, adapter_name: String) -> Self {
        let noise_pipeline = GpuNoisePipeline::new(&device);
        Self {
            device,
            queue,
            noise_pipeline,
            adapter_name,
        }
    }

    /// Synchronously read back `f32` data from a mapped buffer.
    fn readback_f32(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f32>> {
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = device.poll(wgpu::MaintainBase::Wait);
        rx.recv()
            .context("GPU readback channel closed")?
            .context("GPU buffer mapping failed")?;

        let data = slice.get_mapped_range();
        let floats: &[f32] = bytemuck::cast_slice(&data);
        let result = floats[..count].to_vec();
        drop(data);
        buffer.unmap();
        Ok(result)
    }
}

impl TerrainGpuAccelerator for WgpuTerrainAccelerator {
    fn generate_noise(
        &self,
        request: &GpuHeightmapRequest,
        params: &GpuNoiseRequest,
    ) -> Result<GpuHeightmapResult> {
        let w = request.width;
        let h = request.height;
        let cell_count = (w * h) as usize;

        let config = GpuNoiseConfig {
            resolution: [w, h],
            frequency: params.frequency,
            amplitude: params.amplitude,
            lacunarity: params.lacunarity,
            persistence: params.persistence,
            octaves: params.octaves,
            noise_type: GpuNoiseType::Fbm,
            seed: params.seed,
            world_offset: request.world_origin,
            world_scale: request.cell_size,
            ..Default::default()
        };

        // Create uniform buffer
        let gpu_params = config.to_params();
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain_noise_uniform"),
            size: std::mem::size_of_val(&gpu_params) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck::bytes_of(&gpu_params));

        // Create output storage texture (R32Float)
        let output_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain_noise_output"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output_tex.create_view(&Default::default());

        // Readback buffer (texture → buffer → CPU)
        let bytes_per_row = (w * 4).div_ceil(256) * 256; // wgpu alignment
        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain_noise_readback"),
            size: (bytes_per_row * h) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Encode compute pass + copy
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("terrain_noise_encoder"),
            });

        let dispatch = config.dispatch_size();
        self.noise_pipeline.encode(
            &mut encoder,
            &self.device,
            &uniform_buf,
            &output_view,
            dispatch,
        );

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback — handle padded rows
        let slice = readback_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        rx.recv()
            .context("GPU noise readback channel closed")?
            .context("GPU noise buffer mapping failed")?;

        let data = slice.get_mapped_range();
        let row_stride = (bytes_per_row / 4) as usize;
        let mut heights = Vec::with_capacity(cell_count);
        for row in 0..h as usize {
            let start = row * row_stride;
            let row_floats: &[f32] =
                bytemuck::cast_slice(&data[start * 4..(start + w as usize) * 4]);
            heights.extend_from_slice(row_floats);
        }
        drop(data);
        readback_buf.unmap();

        Ok(GpuHeightmapResult {
            heights,
            width: w,
            height: h,
        })
    }

    fn erode_heightmap(
        &self,
        request: &GpuHeightmapRequest,
        params: &GpuErosionRequest,
        input_heights: &[f32],
    ) -> Result<GpuHeightmapResult> {
        let w = request.width;
        let h = request.height;
        let cell_count = (w * h) as usize;

        anyhow::ensure!(
            input_heights.len() == cell_count,
            "input_heights length {} != expected {}",
            input_heights.len(),
            cell_count
        );

        let config = GpuErosionConfig {
            width: w,
            height: h,
            dt: params.dt,
            rain_rate: params.rain_rate,
            sediment_capacity: params.sediment_capacity,
            dissolution_rate: params.dissolution_rate,
            deposition_rate: params.deposition_rate,
            evaporation_rate: params.evaporation_rate,
            ..Default::default()
        };

        let pipeline = GpuErosionPipeline::new(&self.device, config);
        pipeline.upload_terrain(&self.queue, input_heights);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("terrain_erosion_encoder"),
            });

        pipeline.encode_steps(&mut encoder, &self.device, params.iterations);
        pipeline.encode_readback(&mut encoder);

        self.queue.submit(std::iter::once(encoder.finish()));

        Self::readback_f32(&self.device, pipeline.readback_buffer(), cell_count).map(|heights| {
            GpuHeightmapResult {
                heights,
                width: w,
                height: h,
            }
        })
    }

    fn is_available(&self) -> bool {
        true
    }

    fn backend_name(&self) -> &str {
        &self.adapter_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerator_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WgpuTerrainAccelerator>();
    }
}
