use std::{borrow::Cow, collections::HashMap, path::Path};

use crate::material::{
    ArrayLayout, MaterialGpu, MaterialGpuArrays, MaterialLayerDesc, MaterialLoadStats,
};
use anyhow::{anyhow, Context, Result};
use aw_asset_cli::{ColorSpace, TextureMetadata};

pub(crate) mod material_loader_impl {
    use super::*;
    use bytemuck::cast_slice;
    use wgpu::util::DeviceExt;

    fn mip_level_count_for(size: wgpu::Extent3d) -> u32 {
        let max_dim = size.width.max(size.height).max(1);
        32 - max_dim.leading_zeros()
    }

    /// Helper to determine texture format from metadata
    /// Returns (format, is_srgb, channel_count) for proper texture creation
    fn format_from_metadata(
        meta: Option<&TextureMetadata>,
        default_format: wgpu::TextureFormat,
        texture_type: &str, // "albedo", "normal", or "mra"
    ) -> wgpu::TextureFormat {
        if let Some(meta) = meta {
            // Use metadata color_space to determine format
            match texture_type {
                "albedo" => match meta.color_space {
                    ColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
                    ColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
                    _ => default_format,
                },
                "normal" => {
                    // Normal maps are always linear, use RG for BC5-compressed normals
                    wgpu::TextureFormat::Rg8Unorm
                }
                "mra" => {
                    // MRA maps are always linear
                    wgpu::TextureFormat::Rgba8Unorm
                }
                _ => default_format,
            }
        } else {
            // No metadata, use default format
            default_format
        }
    }

    /// Try to load texture metadata for a given texture path
    /// Looks for `.meta.json` file alongside the texture
    fn try_load_metadata(texture_path: &Path) -> Option<TextureMetadata> {
        TextureMetadata::load_for_texture(texture_path).ok()
    }

    /// Validate that texture metadata meets requirements for production rendering
    /// Returns error message if validation fails
    fn validate_texture_metadata(
        meta: Option<&TextureMetadata>,
        texture_type: &str, // "albedo", "normal", or "mra"
        material_key: &str,
        biome_name: &str,
    ) -> Result<()> {
        let meta = meta.ok_or_else(|| {
            anyhow::anyhow!(
                "Missing metadata for {}/{} {} texture - all textures should have .meta.json",
                biome_name,
                material_key,
                texture_type
            )
        })?;

        // Check mip levels requirement
        if meta.mip_levels <= 1 {
            anyhow::bail!(
                "Texture {}/{} {} has only {} mip level(s) - requires full mipmap chain (>1 mips)",
                biome_name,
                material_key,
                texture_type,
                meta.mip_levels
            );
        }

        // Check color-space expectations
        match texture_type {
            "albedo" => {
                if meta.color_space != ColorSpace::Srgb {
                    log::warn!(
                        "[materials] WARN {}/{} albedo has {:?} color-space, expected Srgb",
                        biome_name,
                        material_key,
                        meta.color_space
                    );
                }
            }
            "normal" | "mra" => {
                if meta.color_space != ColorSpace::Linear {
                    log::warn!(
                        "[materials] WARN {}/{} {} has {:?} color-space, expected Linear",
                        biome_name,
                        material_key,
                        texture_type,
                        meta.color_space
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn generate_mipmaps(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        mip_levels: u32,
        base_layer: u32,
        layer_count: u32,
    ) {
        if mip_levels <= 1 {
            return;
        }

        const MIPMAP_SHADER: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
  var out: VsOut;
  let xy = vec2<f32>(f32(i32(vi) - 1), f32((i32(vi) & 1) * 2 - 1));
  out.pos = vec4<f32>(xy, 0.0, 1.0);
  out.uv = vec2<f32>( (xy.x+1.0)*0.5, 1.0 - (xy.y+1.0)*0.5 );
  return out;
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_smp: sampler;

@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(src_tex, src_smp, in.uv);
    return c;
}
"#;

        let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mipmap-gen-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(MIPMAP_SHADER)),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mipmap-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mipmap-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let rp = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mipmap-pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmap-linear-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mipmap-encoder"),
        });

        for layer in base_layer..(base_layer + layer_count) {
            for level in 1..mip_levels {
                let src_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    usage: None,
                    label: Some("mip-src-view"),
                    format: Some(format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                });

                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("mipmap-bg"),
                    layout: &bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&src_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });

                let dst_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    usage: None,
                    label: Some("mip-dst-view"),
                    format: Some(format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                });

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("mipmap-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &dst_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rpass.set_pipeline(&rp);
                rpass.set_bind_group(0, &bind, &[]);
                rpass.draw(0..3, 0..1);
                drop(rpass);
            }
        }

        queue.submit(Some(encoder.finish()));
    }

    fn load_rgba(path: &Path) -> Result<image::RgbaImage> {
        // Check if it's a KTX2 file
        if path.extension().and_then(|s| s.to_str()) == Some("ktx2") {
            return load_ktx2_to_rgba(path);
        }

        let img = astraweave_asset::open_limited_image(path)
            .with_context(|| format!("open {}", path.display()))?;
        Ok(img.to_rgba8())
    }

    fn load_gray(path: &Path) -> Result<image::GrayImage> {
        // Note: KTX2 normal maps (BC5) need special handling
        // For now, we only support PNG/JPEG normals via load_gray
        let img = astraweave_asset::open_limited_image(path)
            .with_context(|| format!("open {}", path.display()))?;
        Ok(img.to_luma8())
    }

    pub(crate) fn reject_oversized_ktx2(width: u32, height: u32) -> Result<()> {
        let edge = astraweave_asset::image_limits::MAX_IMAGE_EDGE;
        let cap = astraweave_asset::image_limits::MAX_IMAGE_ALLOC;
        if width > edge || height > edge {
            return Err(anyhow!("ktx2 edge {width}x{height} exceeds {edge}"));
        }
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .context("ktx2 pixel count")?;
        let rgba = pixels.checked_mul(4).context("ktx2 rgba bytes")?;
        if rgba > cap {
            return Err(anyhow!("ktx2 rgba is {rgba} bytes, cap is {cap}"));
        }
        Ok(())
    }

    /// Load a KTX2 file and decompress to RGBA8 using basis_universal transcoder
    pub(crate) fn load_ktx2_to_rgba(path: &Path) -> Result<image::RgbaImage> {
        use basis_universal::*;

        let data = astraweave_asset::read_limited_image_bytes(path)
            .with_context(|| format!("read ktx2 file {}", path.display()))?;

        let reader = ktx2::Reader::new(&data).context("failed to parse KTX2 header")?;

        let level0 = reader
            .levels()
            .next()
            .ok_or_else(|| anyhow!("KTX2 file has no mip levels"))?;

        let width = reader.header().pixel_width;
        let height = reader.header().pixel_height;
        reject_oversized_ktx2(width, height)?;

        // Check if this is a Basis Universal compressed texture
        // In ktx2 0.4+, check supercompression_scheme instead of data_format_descriptors
        let has_basis_data = reader.header().supercompression_scheme.is_some();

        log::debug!(
            "[ktx2] Loading texture: {} ({}x{}, basis={:?})",
            path.display(),
            width,
            height,
            has_basis_data
        );

        if has_basis_data {
            // Basis Universal compressed texture - use transcoder
            let mut transcoder = Transcoder::new();

            // Initialize transcoder (must be called once per transcoder instance)
            transcoder
                .prepare_transcoding(&data)
                .map_err(|e| anyhow!("Failed to prepare basis transcoding: {:?}", e))?;

            let image_count = transcoder.image_count(&data);
            if image_count == 0 {
                return Err(anyhow!("KTX2 file has no basis images"));
            }

            // Transcode to RGBA8
            let image_index = 0;
            let level_index = 0;

            let transcoded = transcoder
                .transcode_image_level(
                    &data,
                    TranscoderTextureFormat::RGBA32,
                    TranscodeParameters {
                        image_index,
                        level_index,
                        ..Default::default()
                    },
                )
                .map_err(|e| anyhow!("Failed to transcode basis image: {:?}", e))?;

            let img = image::RgbaImage::from_raw(width, height, transcoded)
                .ok_or_else(|| anyhow!("failed to create RGBA image from transcoded data"))?;

            log::debug!("[ktx2] ✓ Transcoded Basis Universal texture to RGBA");
            Ok(img)
        } else {
            // Raw BC-compressed texture - use texture2ddecoder for decoding
            let format_val = reader.header().format;
            let format_desc = format!("{:?}", format_val);
            let is_bc7 = format_desc.contains("98") || format_desc.contains("BC7");
            let is_bc5 = format_desc.contains("143") || format_desc.contains("BC5");
            let is_bc3 = format_desc.contains("133")
                || format_desc.contains("137")
                || format_desc.contains("BC3");
            let is_bc1 = format_desc.contains("131") || format_desc.contains("BC1");

            log::debug!(
                "[ktx2] Decoding BC format: BC7={}, BC5={}, BC3={}, BC1={}",
                is_bc7,
                is_bc5,
                is_bc3,
                is_bc1
            );

            if is_bc7 {
                // BC7: Full RGBA with perceptual endpoint coding
                let mut pixels_u32 = vec![0u32; (width * height) as usize];
                // Access byte data from Level struct
                texture2ddecoder::decode_bc7(
                    level0.data,
                    width as usize,
                    height as usize,
                    &mut pixels_u32,
                )
                .map_err(|e| anyhow!("BC7 decode failed: {}", e))?;

                // Convert u32 pixels to u8 RGBA
                let mut rgba = vec![0u8; (width * height * 4) as usize];
                for (i, &pixel) in pixels_u32.iter().enumerate() {
                    let bytes = pixel.to_le_bytes();
                    rgba[i * 4] = bytes[0]; // R
                    rgba[i * 4 + 1] = bytes[1]; // G
                    rgba[i * 4 + 2] = bytes[2]; // B
                    rgba[i * 4 + 3] = bytes[3]; // A
                }

                let img = image::RgbaImage::from_raw(width, height, rgba)
                    .ok_or_else(|| anyhow!("failed to create RGBA image from BC7 data"))?;

                log::debug!("[ktx2] ✓ Decoded BC7 texture");
                Ok(img)
            } else if is_bc5 {
                // BC5: 2-channel for normal maps, reconstruct Z component
                let mut pixels_u32 = vec![0u32; (width * height) as usize];
                texture2ddecoder::decode_bc5(
                    level0.data,
                    width as usize,
                    height as usize,
                    &mut pixels_u32,
                )
                .map_err(|e| anyhow!("BC5 decode failed: {}", e))?;

                // Convert u32 to u8 and reconstruct Z
                let mut rgba = vec![0u8; (width * height * 4) as usize];
                for (i, &pixel) in pixels_u32.iter().enumerate() {
                    let bytes = pixel.to_le_bytes();
                    let r = bytes[0];
                    let g = bytes[1];

                    // Reconstruct Z component: Z = sqrt(1 - X² - Y²)
                    let x = (r as f32 / 255.0) * 2.0 - 1.0;
                    let y = (g as f32 / 255.0) * 2.0 - 1.0;
                    let z = (1.0 - x * x - y * y).max(0.0).sqrt();
                    let b = ((z + 1.0) * 0.5 * 255.0) as u8;

                    rgba[i * 4] = r;
                    rgba[i * 4 + 1] = g;
                    rgba[i * 4 + 2] = b;
                    rgba[i * 4 + 3] = 255;
                }

                let img = image::RgbaImage::from_raw(width, height, rgba)
                    .ok_or_else(|| anyhow!("failed to create RGBA image from BC5 data"))?;
                log::debug!("[ktx2] ✓ Decoded BC5 normal map with Z reconstruction");
                Ok(img)
            } else if is_bc3 {
                // BC3 (DXT5): RGBA with interpolated alpha
                let mut pixels_u32 = vec![0u32; (width * height) as usize];
                texture2ddecoder::decode_bc3(
                    level0.data,
                    width as usize,
                    height as usize,
                    &mut pixels_u32,
                )
                .map_err(|e| anyhow!("BC3 decode failed: {}", e))?;

                // Convert u32 to u8 RGBA
                let mut rgba = vec![0u8; (width * height * 4) as usize];
                for (i, &pixel) in pixels_u32.iter().enumerate() {
                    let bytes = pixel.to_le_bytes();
                    rgba[i * 4] = bytes[0];
                    rgba[i * 4 + 1] = bytes[1];
                    rgba[i * 4 + 2] = bytes[2];
                    rgba[i * 4 + 3] = bytes[3];
                }

                let img = image::RgbaImage::from_raw(width, height, rgba)
                    .ok_or_else(|| anyhow!("failed to create RGBA image from BC3 data"))?;
                log::debug!("[ktx2] ✓ Decoded BC3 texture");
                Ok(img)
            } else if is_bc1 {
                // BC1 (DXT1): RGB 565 with 1-bit alpha
                let mut pixels_u32 = vec![0u32; (width * height) as usize];
                texture2ddecoder::decode_bc1(
                    level0.data,
                    width as usize,
                    height as usize,
                    &mut pixels_u32,
                )
                .map_err(|e| anyhow!("BC1 decode failed: {}", e))?;
                // Convert u32 to u8 RGBA
                let mut rgba = vec![0u8; (width * height * 4) as usize];
                for (i, &pixel) in pixels_u32.iter().enumerate() {
                    let bytes = pixel.to_le_bytes();
                    rgba[i * 4] = bytes[0];
                    rgba[i * 4 + 1] = bytes[1];
                    rgba[i * 4 + 2] = bytes[2];
                    rgba[i * 4 + 3] = bytes[3];
                }

                let img = image::RgbaImage::from_raw(width, height, rgba)
                    .ok_or_else(|| anyhow!("failed to create RGBA image from BC1 data"))?;

                log::debug!("[ktx2] ✓ Decoded BC1 texture");
                Ok(img)
            } else {
                Err(anyhow!(
                    "Unsupported BC format: {:?}. Supported: BC1, BC3, BC5, BC7",
                    format_val
                ))
            }
        }
    }

    pub fn build_arrays(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layers: &[(String, MaterialLayerDesc)],
        mapping: &HashMap<String, u32>,
        biome_name: &str,
    ) -> Result<(
        MaterialGpuArrays,
        MaterialLoadStats,
        wgpu::Texture,
        wgpu::Texture,
        wgpu::Texture,
    )> {
        let width = 1024u32;
        let height = 1024u32;
        let layer_count = mapping
            .values()
            .max()
            .map(|v| v + 1)
            .unwrap_or(0)
            .max(layers.len() as u32);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layer_count,
        };
        let mip_level_count = mip_level_count_for(size);

        fn make_array(
            device: &wgpu::Device,
            label: &str,
            size: wgpu::Extent3d,
            mips: u32,
            fmt: wgpu::TextureFormat,
            samp: &wgpu::SamplerDescriptor,
        ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor {
                usage: None,
                label: Some(label),
                format: Some(fmt),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(mips),
                base_array_layer: 0,
                array_layer_count: Some(size.depth_or_array_layers),
            });
            let sampler = device.create_sampler(samp);
            (tex, view, sampler)
        }

        let base_sampler = wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        };

        // Check first layer for metadata to determine format policy
        // In production, all textures should have metadata; we use first layer as representative sample
        let sample_albedo_meta = layers
            .first()
            .and_then(|(_, desc)| desc.albedo.as_ref().and_then(|p| try_load_metadata(p)));
        let sample_normal_meta = layers
            .first()
            .and_then(|(_, desc)| desc.normal.as_ref().and_then(|p| try_load_metadata(p)));
        let sample_mra_meta = layers
            .first()
            .and_then(|(_, desc)| desc.mra.as_ref().and_then(|p| try_load_metadata(p)));

        // Use metadata to determine formats, fallback to defaults
        let alb_fmt = format_from_metadata(
            sample_albedo_meta.as_ref(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "albedo",
        );
        let nrm_fmt = format_from_metadata(
            sample_normal_meta.as_ref(),
            wgpu::TextureFormat::Rg8Unorm,
            "normal",
        );
        let mra_fmt = format_from_metadata(
            sample_mra_meta.as_ref(),
            wgpu::TextureFormat::Rgba8Unorm,
            "mra",
        );
        let (alb_tex, alb_view, samp_alb) = make_array(
            device,
            "mat-albedo",
            size,
            mip_level_count,
            alb_fmt,
            &base_sampler,
        );
        let (nrm_tex, nrm_view, samp_lin) = make_array(
            device,
            "mat-normal",
            size,
            mip_level_count,
            nrm_fmt,
            &base_sampler,
        );
        let (mra_tex, mra_view, _s) = make_array(
            device,
            "mat-mra",
            size,
            mip_level_count,
            mra_fmt,
            &base_sampler,
        );

        // Diagnostic: print chosen formats and mip count so test runs can validate expectations
        log::debug!("[materials-debug] building arrays: layers={} size={}x{} mips={} formats: albedo={:?} normal={:?} mra={:?}",
            layer_count, width, height, mip_level_count, alb_fmt, nrm_fmt, mra_fmt);

        let mut stats = MaterialLoadStats {
            biome: biome_name.to_string(),
            ..Default::default()
        };
        stats.layers_total = layer_count as usize;

        let mut layout = ArrayLayout {
            layer_indices: HashMap::new(),
            count: layer_count,
        };

        let mut material_records = (0..layer_count)
            .map(MaterialGpu::neutral)
            .collect::<Vec<_>>();

        // Neutral patterns used for fallbacks
        let neutral_albedo = vec![255u8; (width * height * 4) as usize];
        let neutral_normal_rg = vec![128u8; (width * height * 2) as usize]; // xy = 0.5
        let neutral_mra = {
            let mut v = vec![0u8; (width * height * 4) as usize];
            for px in v.chunks_mut(4) {
                px[0] = 0;
                px[1] = 128;
                px[2] = 255;
                px[3] = 255;
            }
            v
        };

        // Helper to write a whole layer at mip 0
        let write_layer =
            |tex: &wgpu::Texture, bytes: &[u8], bpr: u32, _fmt: wgpu::TextureFormat, layer: u32| {
                let block = wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bpr),
                        rows_per_image: Some(height),
                    },
                    block,
                );
            };

        // Start with neutral values for every layer so missing content is defined
        for layer in 0..layer_count {
            write_layer(
                &alb_tex,
                &neutral_albedo,
                width * 4,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                layer,
            );
            write_layer(
                &nrm_tex,
                &neutral_normal_rg,
                width * 2,
                wgpu::TextureFormat::Rg8Unorm,
                layer,
            );
            write_layer(
                &mra_tex,
                &neutral_mra,
                width * 4,
                wgpu::TextureFormat::Rgba8Unorm,
                layer,
            );
        }

        // Load, pack, and upload per-layer
        for (key, desc) in layers.iter() {
            if let Some(&idx) = mapping.get(key) {
                layout.layer_indices.insert(key.clone(), idx);

                let record = &mut material_records[idx as usize];
                record.tiling_triplanar =
                    [desc.tiling[0], desc.tiling[1], desc.triplanar_scale, 0.0];
                if desc.triplanar_scale != 0.0 && desc.triplanar_scale != 1.0 {
                    record.flags |= MaterialGpu::FLAG_TRIPLANAR;
                }

                let mut has_albedo = false;
                let mut has_normal = false;
                let mut has_orm = false;

                // Albedo
                if let Some(ref p) = desc.albedo {
                    // Load metadata to check color-space
                    let meta = try_load_metadata(p);
                    if let Some(ref m) = meta {
                        log::info!("[materials] loaded metadata for {}/{} albedo: color_space={:?} mips={} compression={:?}",
                            biome_name, key, m.color_space, m.mip_levels, m.compression);
                    } else {
                        log::info!(
                            "[materials] WARN no metadata for {}/{} albedo → assuming sRGB",
                            biome_name,
                            key
                        );
                    }

                    // Validate metadata (production requirement)
                    // NOTE: This will log warnings for now but won't block loading
                    // Remove the `if let Err(e)` to enforce strict validation
                    if let Err(e) =
                        validate_texture_metadata(meta.as_ref(), "albedo", key, biome_name)
                    {
                        log::warn!(
                            "[materials] VALIDATION WARNING: {} (loading anyway with fallbacks)",
                            e
                        );
                    }

                    match load_rgba(p) {
                        Ok(img) => {
                            let img = image::imageops::resize(
                                &img,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            write_layer(
                                &alb_tex,
                                &img,
                                width * 4,
                                wgpu::TextureFormat::Rgba8UnormSrgb,
                                idx,
                            );
                            stats.albedo_loaded += 1;
                            has_albedo = true;
                        }
                        Err(e) => {
                            log::warn!("[materials] WARN missing/bad albedo for {}/{}: {} → substituting neutral", biome_name, key, e);
                            stats.albedo_substituted += 1;
                        }
                    }
                } else {
                    // 1x1 policy mentioned → we log and keep the neutral (already written full-res neutral)
                    log::warn!(
                        "[materials] WARN albedo not provided for {}/{} → substituting 1×1 neutral",
                        biome_name,
                        key
                    );
                    stats.albedo_substituted += 1;
                }

                // Normal (RG from XY)
                if let Some(ref p) = desc.normal {
                    // Load metadata to check color-space
                    let meta = try_load_metadata(p);
                    if let Some(ref m) = meta {
                        log::info!("[materials] loaded metadata for {}/{} normal: color_space={:?} mips={} compression={:?}",
                            biome_name, key, m.color_space, m.mip_levels, m.compression);
                    } else {
                        log::info!(
                            "[materials] WARN no metadata for {}/{} normal → assuming Linear RG",
                            biome_name,
                            key
                        );
                    }

                    // Validate metadata (production requirement)
                    if let Err(e) =
                        validate_texture_metadata(meta.as_ref(), "normal", key, biome_name)
                    {
                        log::warn!(
                            "[materials] VALIDATION WARNING: {} (loading anyway with fallbacks)",
                            e
                        );
                    }

                    match load_rgba(p) {
                        Ok(img_rgba) => {
                            // Extract RG channels
                            let img = image::imageops::resize(
                                &img_rgba,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            let mut rg = vec![0u8; (width * height * 2) as usize];
                            let mut o = 0usize;
                            for px in img.pixels() {
                                rg[o] = px[0];
                                rg[o + 1] = px[1];
                                o += 2;
                            }
                            write_layer(
                                &nrm_tex,
                                &rg,
                                width * 2,
                                wgpu::TextureFormat::Rg8Unorm,
                                idx,
                            );
                            stats.normal_loaded += 1;
                            has_normal = true;
                        }
                        Err(e) => {
                            log::warn!("[materials] WARN missing/bad normal for {}/{}: {} → substituting neutral", biome_name, key, e);
                            stats.normal_substituted += 1;
                        }
                    }
                } else {
                    log::warn!(
                        "[materials] WARN normal not provided for {}/{} → substituting neutral",
                        biome_name,
                        key
                    );
                    stats.normal_substituted += 1;
                }

                // MRA
                if let Some(ref p) = desc.mra {
                    // Load metadata to check color-space
                    let meta = try_load_metadata(p);
                    if let Some(ref m) = meta {
                        log::info!("[materials] loaded metadata for {}/{} mra: color_space={:?} mips={} compression={:?}",
                            biome_name, key, m.color_space, m.mip_levels, m.compression);
                    } else {
                        log::info!(
                            "[materials] WARN no metadata for {}/{} mra → assuming Linear RGBA",
                            biome_name,
                            key
                        );
                    }

                    // Validate metadata (production requirement)
                    if let Err(e) = validate_texture_metadata(meta.as_ref(), "mra", key, biome_name)
                    {
                        log::warn!(
                            "[materials] VALIDATION WARNING: {} (loading anyway with fallbacks)",
                            e
                        );
                    }

                    match load_rgba(p) {
                        Ok(img) => {
                            let img = image::imageops::resize(
                                &img,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            write_layer(
                                &mra_tex,
                                &img,
                                width * 4,
                                wgpu::TextureFormat::Rgba8Unorm,
                                idx,
                            );
                            stats.mra_loaded += 1;
                            has_orm = true;
                        }
                        Err(e) => {
                            log::warn!(
                                "[materials] WARN missing/bad MRA for {}/{}: {}",
                                biome_name,
                                key,
                                e
                            );
                            // Try packing from separate channels
                            if let (Some(m), Some(r), Some(a)) =
                                (&desc.metallic, &desc.roughness, &desc.ao)
                            {
                                match (load_gray(m), load_gray(r), load_gray(a)) {
                                    (Ok(m), Ok(r), Ok(a)) => {
                                        let m = image::imageops::resize(
                                            &m,
                                            width,
                                            height,
                                            image::imageops::FilterType::Lanczos3,
                                        );
                                        let r = image::imageops::resize(
                                            &r,
                                            width,
                                            height,
                                            image::imageops::FilterType::Lanczos3,
                                        );
                                        let a = image::imageops::resize(
                                            &a,
                                            width,
                                            height,
                                            image::imageops::FilterType::Lanczos3,
                                        );
                                        let mut out = vec![0u8; (width * height * 4) as usize];
                                        let mut o = 0usize;
                                        for y in 0..height {
                                            for x in 0..width {
                                                let mi = m.get_pixel(x, y)[0];
                                                let ri = r.get_pixel(x, y)[0];
                                                let ai = a.get_pixel(x, y)[0];
                                                out[o] = mi;
                                                out[o + 1] = ri;
                                                out[o + 2] = ai;
                                                out[o + 3] = 255;
                                                o += 4;
                                            }
                                        }
                                        write_layer(
                                            &mra_tex,
                                            &out,
                                            width * 4,
                                            wgpu::TextureFormat::Rgba8Unorm,
                                            idx,
                                        );
                                        stats.mra_loaded += 1;
                                        stats.mra_packed += 1;
                                        has_orm = true;
                                    }
                                    _ => {
                                        log::warn!("[materials] WARN cannot pack MRA for {}/{} → substituting neutral", biome_name, key);
                                        stats.mra_substituted += 1;
                                    }
                                }
                            } else {
                                stats.mra_substituted += 1;
                            }
                        }
                    }
                } else if let (Some(m), Some(r), Some(a)) =
                    (&desc.metallic, &desc.roughness, &desc.ao)
                {
                    match (load_gray(m), load_gray(r), load_gray(a)) {
                        (Ok(m), Ok(r), Ok(a)) => {
                            let m = image::imageops::resize(
                                &m,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            let r = image::imageops::resize(
                                &r,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            let a = image::imageops::resize(
                                &a,
                                width,
                                height,
                                image::imageops::FilterType::Lanczos3,
                            );
                            let mut out = vec![0u8; (width * height * 4) as usize];
                            let mut o = 0usize;
                            for y in 0..height {
                                for x in 0..width {
                                    let mi = m.get_pixel(x, y)[0];
                                    let ri = r.get_pixel(x, y)[0];
                                    let ai = a.get_pixel(x, y)[0];
                                    out[o] = mi;
                                    out[o + 1] = ri;
                                    out[o + 2] = ai;
                                    out[o + 3] = 255;
                                    o += 4;
                                }
                            }
                            write_layer(
                                &mra_tex,
                                &out,
                                width * 4,
                                wgpu::TextureFormat::Rgba8Unorm,
                                idx,
                            );
                            stats.mra_loaded += 1;
                            stats.mra_packed += 1;
                            has_orm = true;
                        }
                        _ => {
                            log::warn!(
                                "[materials] WARN cannot pack MRA for {}/{} → substituting neutral",
                                biome_name,
                                key
                            );
                            stats.mra_substituted += 1;
                        }
                    }
                } else {
                    log::warn!(
                        "[materials] WARN MRA not provided for {}/{} → substituting neutral",
                        biome_name,
                        key
                    );
                    stats.mra_substituted += 1;
                }

                if has_albedo {
                    record.flags |= MaterialGpu::FLAG_HAS_ALBEDO;
                }
                if has_normal {
                    record.flags |= MaterialGpu::FLAG_HAS_NORMAL;
                }
                if has_orm {
                    record.flags |= MaterialGpu::FLAG_HAS_ORM;
                }
            }
        }

        // Generate mips for all layers
        generate_mipmaps(
            device,
            queue,
            &alb_tex,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            mip_level_count,
            0,
            layer_count,
        );
        generate_mipmaps(
            device,
            queue,
            &nrm_tex,
            wgpu::TextureFormat::Rg8Unorm,
            mip_level_count,
            0,
            layer_count,
        );
        generate_mipmaps(
            device,
            queue,
            &mra_tex,
            wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count,
            0,
            layer_count,
        );

        // GPU memory estimate (sum of mips for each texture)
        let sum_mips = |bpp: u32| -> u64 {
            let mut total: u64 = 0;
            let mut w = width as u64;
            let mut h = height as u64;
            for _ in 0..mip_level_count {
                total += w.max(1) * h.max(1) * bpp as u64 * layer_count as u64;
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }
            total
        };
        stats.gpu_memory_bytes = sum_mips(4) + sum_mips(2) + sum_mips(4);

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("material-gpu-records"),
            contents: cast_slice(&material_records),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let gpu = MaterialGpuArrays {
            albedo: alb_view,
            normal: nrm_view,
            mra: mra_view,
            sampler_albedo: samp_alb,
            sampler_linear: samp_lin,
            layout,
            materials: material_records,
            material_buffer,
        };

        Ok((gpu, stats, alb_tex, nrm_tex, mra_tex))
    }
}

#[cfg(test)]
mod ktx2_limit_tests {
    use super::material_loader_impl::reject_oversized_ktx2;

    #[test]
    fn ktx2_edge_above_4096_is_rejected() {
        let err = reject_oversized_ktx2(8192, 8).expect_err("edge");
        assert!(format!("{err:#}").contains("8192"), "{err:#}");
        reject_oversized_ktx2(4096, 4096).expect("4096 square is the cap");
    }
}
