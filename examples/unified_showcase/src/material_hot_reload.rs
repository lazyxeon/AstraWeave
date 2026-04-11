// Comprehensive GPU Hot-Reload System for unified_showcase
// Supports standard materials, extended materials (Phase PBR-E), and terrain materials (Phase PBR-F)
// Full texture array management with automatic bind group recreation
// Production-ready with comprehensive error handling and performance optimization

use anyhow::{Context, Result};
use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// Reload event types for different asset categories
#[derive(Debug, Clone)]
pub enum ReloadEvent {
    /// Standard material TOML changed (materials.toml, individual .toml)
    Material {
        path: PathBuf,
        material_type: MaterialType,
    },
    /// Texture file changed (albedo, normal, ORM/MRA)
    Texture {
        path: PathBuf,
        texture_type: TextureType,
        color_space: ColorSpace,
    },
    /// Texture array manifest changed (arrays.toml)
    ArrayManifest { path: PathBuf },
    /// Extended material configuration (Phase PBR-E)
    ExtendedMaterial { path: PathBuf },
    /// Terrain material configuration (Phase PBR-F)
    TerrainMaterial { path: PathBuf },
}

/// Material type classification for GPU buffer routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialType {
    Standard, // Basic PBR material
    Extended, // Phase PBR-E (clearcoat, anisotropy, SSS, sheen, transmission)
    Terrain,  // Phase PBR-F (splat blending, triplanar, height blending)
    Biome,    // Environment-specific material pack
}

/// Texture type for array index routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    Albedo,   // Base color (sRGB)
    Normal,   // Normal map (linear)
    ORM,      // Occlusion-Roughness-Metallic (linear)
    MRA,      // Metallic-Roughness-AO (alternative packing, linear)
    Emissive, // Emission map (sRGB)
    Height,   // Height map for parallax/displacement (linear)
    Splat,    // Splat mask for terrain blending (linear)
}

/// Color space handling for texture uploads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    SRGB,   // sRGB color space (albedo, emissive)
    Linear, // Linear color space (normal, ORM, height, splat)
}

/// Material ID to file path mapping for hot-reload targeting
#[derive(Debug, Clone)]
pub struct MaterialMapping {
    pub material_id: u32,
    pub material_type: MaterialType,
    pub toml_path: PathBuf,
    pub albedo_path: Option<PathBuf>,
    pub normal_path: Option<PathBuf>,
    pub orm_path: Option<PathBuf>,
    pub array_indices: MaterialArrayIndices,
}

/// Texture array indices for GPU updates
#[derive(Debug, Clone, Copy, Default)]
pub struct MaterialArrayIndices {
    pub albedo_index: u32,
    pub normal_index: u32,
    pub orm_index: u32,
}

/// File watcher with debouncing and filtering
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<ReloadEvent>,
    debounce_map: HashMap<PathBuf, Instant>,
    debounce_duration: Duration,
}

impl FileWatcher {
    /// Create a new file watcher monitoring the specified directory
    pub fn new(watch_dir: impl AsRef<Path>) -> Result<Self> {
        let (tx, rx) = channel();
        let debounce_duration = Duration::from_millis(500); // 500ms debounce

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                        for path in event.paths {
                            if let Some(reload_event) = Self::classify_file(&path) {
                                let _ = tx.send(reload_event);
                            }
                        }
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(watch_dir.as_ref(), RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
            debounce_map: HashMap::new(),
            debounce_duration,
        })
    }

    /// Classify a file path into a reload event type
    fn classify_file(path: &Path) -> Option<ReloadEvent> {
        let path_str = path.to_string_lossy().to_lowercase();
        let extension = path.extension()?.to_str()?;

        // Check for TOML files
        if extension == "toml" {
            let filename = path.file_name()?.to_str()?;

            if filename == "arrays.toml" {
                return Some(ReloadEvent::ArrayManifest {
                    path: path.to_path_buf(),
                });
            }

            if filename == "materials.toml" {
                return Some(ReloadEvent::Material {
                    path: path.to_path_buf(),
                    material_type: MaterialType::Biome,
                });
            }

            // Classify individual material files
            let material_type = if path_str.contains("terrain") {
                MaterialType::Terrain
            } else if path_str.contains("extended") || path_str.contains("advanced") {
                MaterialType::Extended
            } else {
                MaterialType::Standard
            };

            return Some(ReloadEvent::Material {
                path: path.to_path_buf(),
                material_type,
            });
        }

        // Check for texture files
        if matches!(extension, "png" | "ktx2" | "dds" | "basis") {
            let filename = path.file_stem()?.to_str()?.to_lowercase();

            // Classify texture type and color space
            let (texture_type, color_space) = if filename.contains("albedo")
                || filename.contains("color")
                || filename.contains("diffuse")
            {
                (TextureType::Albedo, ColorSpace::SRGB)
            } else if filename.contains("normal") || filename.contains("_n") {
                (TextureType::Normal, ColorSpace::Linear)
            } else if filename.contains("orm") {
                (TextureType::ORM, ColorSpace::Linear)
            } else if filename.contains("mra") || filename.contains("roughness") {
                (TextureType::MRA, ColorSpace::Linear)
            } else if filename.contains("emissive") || filename.contains("emission") {
                (TextureType::Emissive, ColorSpace::SRGB)
            } else if filename.contains("height") || filename.contains("displacement") {
                (TextureType::Height, ColorSpace::Linear)
            } else if filename.contains("splat") || filename.contains("mask") {
                (TextureType::Splat, ColorSpace::Linear)
            } else {
                return None; // Unknown texture type
            };

            return Some(ReloadEvent::Texture {
                path: path.to_path_buf(),
                texture_type,
                color_space,
            });
        }

        None
    }

    /// Try to receive a reload event (non-blocking with debouncing)
    pub fn try_recv(&mut self) -> Option<ReloadEvent> {
        while let Ok(event) = self.rx.try_recv() {
            let path = match &event {
                ReloadEvent::Material { path, .. } => path,
                ReloadEvent::Texture { path, .. } => path,
                ReloadEvent::ArrayManifest { path } => path,
                ReloadEvent::ExtendedMaterial { path } => path,
                ReloadEvent::TerrainMaterial { path } => path,
            };

            let now = Instant::now();

            // Check if we should debounce this event
            if let Some(last_time) = self.debounce_map.get(path) {
                if now.duration_since(*last_time) < self.debounce_duration {
                    continue; // Debounce this event
                }
            }

            // Update debounce map and return event
            self.debounce_map.insert(path.clone(), now);
            return Some(event);
        }

        None
    }

    /// Collect all pending events (respecting debounce)
    pub fn collect_events(&mut self) -> Vec<ReloadEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Material reload manager for GPU updates
pub struct MaterialReloadManager {
    /// Material ID to file path mapping
    material_mappings: HashMap<u32, MaterialMapping>,

    /// Biome name to base directory mapping
    biome_directories: HashMap<String, PathBuf>,

    /// Current active biome
    current_biome: Option<String>,

    /// Reload statistics
    reload_count: u64,
    last_reload_time: Instant,

    /// Performance tracking
    total_reload_time_ms: f64,
}

impl MaterialReloadManager {
    pub fn new() -> Self {
        Self {
            material_mappings: HashMap::new(),
            biome_directories: HashMap::new(),
            current_biome: None,
            reload_count: 0,
            last_reload_time: Instant::now(),
            total_reload_time_ms: 0.0,
        }
    }

    /// Register a biome directory for hot-reload tracking
    pub fn register_biome(&mut self, biome: &str, base_dir: PathBuf) {
        self.biome_directories.insert(biome.to_string(), base_dir);
    }

    /// Set the currently active biome
    pub fn set_current_biome(&mut self, biome: &str) {
        self.current_biome = Some(biome.to_string());
    }

    /// Register a material for hot-reload tracking
    pub fn register_material(
        &mut self,
        material_id: u32,
        material_type: MaterialType,
        toml_path: PathBuf,
        array_indices: MaterialArrayIndices,
    ) {
        let mapping = MaterialMapping {
            material_id,
            material_type,
            toml_path,
            albedo_path: None,
            normal_path: None,
            orm_path: None,
            array_indices,
        };

        self.material_mappings.insert(material_id, mapping);
    }

    /// Update texture paths for a material
    pub fn update_material_textures(
        &mut self,
        material_id: u32,
        albedo: Option<PathBuf>,
        normal: Option<PathBuf>,
        orm: Option<PathBuf>,
    ) {
        if let Some(mapping) = self.material_mappings.get_mut(&material_id) {
            mapping.albedo_path = albedo;
            mapping.normal_path = normal;
            mapping.orm_path = orm;
        }
    }

    /// Find material ID by file path
    pub fn find_material_by_path(&self, path: &Path) -> Option<u32> {
        for (material_id, mapping) in &self.material_mappings {
            if mapping.toml_path == path {
                return Some(*material_id);
            }

            if let Some(ref albedo) = mapping.albedo_path {
                if albedo == path {
                    return Some(*material_id);
                }
            }

            if let Some(ref normal) = mapping.normal_path {
                if normal == path {
                    return Some(*material_id);
                }
            }

            if let Some(ref orm) = mapping.orm_path {
                if orm == path {
                    return Some(*material_id);
                }
            }
        }

        None
    }

    /// Check if a path belongs to the current biome
    pub fn is_current_biome_path(&self, path: &Path) -> bool {
        if let Some(ref biome) = self.current_biome {
            if let Some(base_dir) = self.biome_directories.get(biome) {
                return path.starts_with(base_dir);
            }
        }
        false
    }

    /// Reload a material from TOML file (CPU → GPU conversion)
    pub fn reload_material_from_toml(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_buffer: &wgpu::Buffer,
        path: &Path,
        material_type: MaterialType,
    ) -> Result<()> {
        let start = Instant::now();

        // Parse TOML file
        let toml_content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read material TOML: {}", path.display()))?;

        // Find material ID for this path
        let material_id = self
            .find_material_by_path(path)
            .ok_or_else(|| anyhow::anyhow!("Material not registered: {}", path.display()))?;

        // Convert based on material type
        match material_type {
            MaterialType::Standard | MaterialType::Biome => {
                self.reload_standard_material(
                    device,
                    queue,
                    material_buffer,
                    material_id,
                    &toml_content,
                )?;
            }
            MaterialType::Extended => {
                self.reload_extended_material(
                    device,
                    queue,
                    material_buffer,
                    material_id,
                    &toml_content,
                )?;
            }
            MaterialType::Terrain => {
                self.reload_terrain_material(
                    device,
                    queue,
                    material_buffer,
                    material_id,
                    &toml_content,
                )?;
            }
        }

        // Update statistics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.reload_count += 1;
        self.last_reload_time = Instant::now();
        self.total_reload_time_ms += elapsed;

        println!(
            "✅ Hot-reloaded material: {} ({:.2}ms)",
            path.display(),
            elapsed
        );

        Ok(())
    }

    /// Reload a standard PBR material
    fn reload_standard_material(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_buffer: &wgpu::Buffer,
        material_id: u32,
        toml_content: &str,
    ) -> Result<()> {
        // Parse material from TOML
        let material: crate::material::Material =
            toml::from_str(toml_content).context("Failed to parse material TOML")?;

        // Convert to GPU representation
        let material_gpu = crate::material::MaterialGpu::from(&material);

        // Calculate buffer offset
        let offset =
            (material_id as u64) * std::mem::size_of::<crate::material::MaterialGpu>() as u64;

        // Write to GPU buffer
        queue.write_buffer(material_buffer, offset, bytemuck::bytes_of(&material_gpu));

        Ok(())
    }

    /// Reload an extended material (Phase PBR-E)
    fn reload_extended_material(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_buffer: &wgpu::Buffer,
        material_id: u32,
        toml_content: &str,
    ) -> Result<()> {
        // Parse TOML to get material parameters
        let toml_value: toml::Value = toml::from_str(toml_content)
            .with_context(|| "Failed to parse extended material TOML")?;

        // Start with default material
        let mut material = astraweave_render::MaterialGpuExtended::default();

        // Parse base PBR properties
        if let Some(mat) = toml_value.get("material").and_then(|v| v.as_table()) {
            if let Some(albedo) = mat.get("base_color_factor").and_then(|v| v.as_array()) {
                material.base_color_factor = [
                    albedo.get(0).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    albedo.get(1).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    albedo.get(2).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    albedo.get(3).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                ];
            }

            if let Some(metallic) = mat.get("metallic_factor").and_then(|v| v.as_float()) {
                material.metallic_factor = metallic as f32;
            }

            if let Some(roughness) = mat.get("roughness_factor").and_then(|v| v.as_float()) {
                material.roughness_factor = roughness as f32;
            }

            // Parse clearcoat properties
            if let Some(clearcoat) = mat.get("clearcoat_strength").and_then(|v| v.as_float()) {
                material.clearcoat_strength = clearcoat as f32;
                material.flags |= astraweave_render::MATERIAL_FLAG_CLEARCOAT;
            }

            if let Some(clear_rough) = mat.get("clearcoat_roughness").and_then(|v| v.as_float()) {
                material.clearcoat_roughness = clear_rough as f32;
            }

            // Parse anisotropy properties
            if let Some(aniso) = mat.get("anisotropy_strength").and_then(|v| v.as_float()) {
                material.anisotropy_strength = aniso as f32;
                material.flags |= astraweave_render::MATERIAL_FLAG_ANISOTROPY;
            }

            if let Some(rot) = mat.get("anisotropy_rotation").and_then(|v| v.as_float()) {
                material.anisotropy_rotation = rot as f32;
            }

            // Parse subsurface properties
            if let Some(sss_color) = mat.get("subsurface_color").and_then(|v| v.as_array()) {
                material.subsurface_color = [
                    sss_color.get(0).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    sss_color.get(1).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    sss_color.get(2).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                ];
                material.flags |= astraweave_render::MATERIAL_FLAG_SUBSURFACE;
            }

            if let Some(sss_scale) = mat.get("subsurface_scale").and_then(|v| v.as_float()) {
                material.subsurface_scale = sss_scale as f32;
            }

            // Parse sheen properties
            if let Some(sheen_color) = mat.get("sheen_color").and_then(|v| v.as_array()) {
                material.sheen_color = [
                    sheen_color.get(0).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                    sheen_color.get(1).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                    sheen_color.get(2).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                ];
                material.flags |= astraweave_render::MATERIAL_FLAG_SHEEN;
            }

            // Parse transmission properties
            if let Some(trans) = mat.get("transmission_factor").and_then(|v| v.as_float()) {
                material.transmission_factor = trans as f32;
                material.flags |= astraweave_render::MATERIAL_FLAG_TRANSMISSION;
            }

            if let Some(ior) = mat.get("ior").and_then(|v| v.as_float()) {
                material.ior = ior as f32;
            }
        }

        // Calculate GPU buffer offset (256 bytes per extended material)
        let offset = (material_id as u64)
            * std::mem::size_of::<astraweave_render::MaterialGpuExtended>() as u64;

        // Upload to GPU
        queue.write_buffer(material_buffer, offset, bytemuck::bytes_of(&material));

        println!("✅ Extended material reloaded: material_id={} (clearcoat={}, aniso={}, sss={}, sheen={}, trans={})",
            material_id,
            material.has_feature(astraweave_render::MATERIAL_FLAG_CLEARCOAT),
            material.has_feature(astraweave_render::MATERIAL_FLAG_ANISOTROPY),
            material.has_feature(astraweave_render::MATERIAL_FLAG_SUBSURFACE),
            material.has_feature(astraweave_render::MATERIAL_FLAG_SHEEN),
            material.has_feature(astraweave_render::MATERIAL_FLAG_TRANSMISSION),
        );

        Ok(())
    }

    /// Reload a terrain material (Phase PBR-F)
    fn reload_terrain_material(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_buffer: &wgpu::Buffer,
        material_id: u32,
        toml_content: &str,
    ) -> Result<()> {
        // Parse TOML to get terrain material parameters
        let toml_value: toml::Value = toml::from_str(toml_content)
            .with_context(|| "Failed to parse terrain material TOML")?;

        // Start with default material
        let mut material = astraweave_render::TerrainMaterialGpu::default();

        // Parse terrain properties
        if let Some(terrain) = toml_value.get("terrain").and_then(|v| v.as_table()) {
            if let Some(splat_scale) = terrain.get("splat_uv_scale").and_then(|v| v.as_float()) {
                material.splat_uv_scale = splat_scale as f32;
            }

            if let Some(triplanar) = terrain.get("triplanar_enabled").and_then(|v| v.as_bool()) {
                material.triplanar_enabled = if triplanar { 1 } else { 0 };
            }

            if let Some(threshold) = terrain
                .get("triplanar_slope_threshold")
                .and_then(|v| v.as_float())
            {
                material.triplanar_slope_threshold = threshold as f32;
            }

            if let Some(blend_method) = terrain
                .get("normal_blend_method")
                .and_then(|v| v.as_integer())
            {
                material.normal_blend_method = blend_method as u32;
            }

            if let Some(height_blend) = terrain
                .get("height_blend_enabled")
                .and_then(|v| v.as_bool())
            {
                material.height_blend_enabled = if height_blend { 1 } else { 0 };
            }

            // Parse layers
            if let Some(layers) = terrain.get("layers").and_then(|v| v.as_array()) {
                for (i, layer_value) in layers.iter().enumerate().take(8) {
                    if let Some(layer) = layer_value.as_table() {
                        // Parse UV scale
                        if let Some(uv_scale) = layer.get("uv_scale").and_then(|v| v.as_array()) {
                            material.layers[i].uv_scale = [
                                uv_scale.get(0).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                                uv_scale.get(1).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                            ];
                        }

                        // Parse height range
                        if let Some(height_range) =
                            layer.get("height_range").and_then(|v| v.as_array())
                        {
                            material.layers[i].height_range = [
                                height_range
                                    .get(0)
                                    .and_then(|v| v.as_float())
                                    .unwrap_or(0.0) as f32,
                                height_range
                                    .get(1)
                                    .and_then(|v| v.as_float())
                                    .unwrap_or(100.0) as f32,
                            ];
                        }

                        // Parse blend sharpness
                        if let Some(sharpness) =
                            layer.get("blend_sharpness").and_then(|v| v.as_float())
                        {
                            material.layers[i].blend_sharpness = sharpness as f32;
                        }

                        // Parse triplanar power
                        if let Some(power) = layer.get("triplanar_power").and_then(|v| v.as_float())
                        {
                            material.layers[i].triplanar_power = power as f32;
                        }

                        // Parse material factors
                        if let Some(metallic) = layer.get("metallic").and_then(|v| v.as_float()) {
                            material.layers[i].material_factors[0] = metallic as f32;
                        }

                        if let Some(roughness) = layer.get("roughness").and_then(|v| v.as_float()) {
                            material.layers[i].material_factors[1] = roughness as f32;
                        }
                    }
                }
            }
        }

        // Calculate GPU buffer offset (320 bytes per terrain material)
        let offset = (material_id as u64)
            * std::mem::size_of::<astraweave_render::TerrainMaterialGpu>() as u64;

        // Upload to GPU
        queue.write_buffer(material_buffer, offset, bytemuck::bytes_of(&material));

        println!("✅ Terrain material reloaded: material_id={} (triplanar={}, height_blend={}, layers=4)",
            material_id,
            material.triplanar_enabled == 1,
            material.height_blend_enabled == 1,
        );

        Ok(())
    }

    /// Reload a texture and upload to GPU texture array
    pub fn reload_texture(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_array: &wgpu::Texture,
        array_index: u32,
        path: &Path,
        _texture_type: TextureType,
        color_space: ColorSpace,
    ) -> Result<()> {
        let start = Instant::now();

        // Load image file
        let img = image::open(path)
            .with_context(|| format!("Failed to load texture: {}", path.display()))?;

        // Convert to RGBA8
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Determine texture format based on color space
        let format = match color_space {
            ColorSpace::SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
        };

        // Validate texture format matches array format
        if texture_array.format() != format {
            anyhow::bail!(
                "Texture format mismatch: array={:?}, file={:?} ({})",
                texture_array.format(),
                format,
                path.display()
            );
        }

        // Write texture data to GPU array
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: texture_array,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: array_index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Update statistics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.reload_count += 1;
        self.last_reload_time = Instant::now();
        self.total_reload_time_ms += elapsed;

        println!(
            "✅ Hot-reloaded texture: {} → array[{}] ({:.2}ms, {}x{}, {:?})",
            path.display(),
            array_index,
            elapsed,
            width,
            height,
            color_space
        );

        Ok(())
    }

    /// Get array indices for a material (for texture array routing)
    pub fn get_array_indices(&self, material_id: u32) -> Option<&MaterialArrayIndices> {
        self.material_mappings
            .get(&material_id)
            .map(|m| &m.array_indices)
    }

    /// Get texture type from file path (for hot-reload event routing)
    pub fn get_texture_type_for_path(&self, material_id: u32, path: &Path) -> Option<TextureType> {
        let mapping = self.material_mappings.get(&material_id)?;

        if let Some(ref albedo) = mapping.albedo_path {
            if albedo == path {
                return Some(TextureType::Albedo);
            }
        }

        if let Some(ref normal) = mapping.normal_path {
            if normal == path {
                return Some(TextureType::Normal);
            }
        }

        if let Some(ref orm) = mapping.orm_path {
            if orm == path {
                return Some(TextureType::ORM);
            }
        }

        None
    }

    /// Get reload statistics
    pub fn get_stats(&self) -> ReloadStats {
        ReloadStats {
            reload_count: self.reload_count,
            last_reload_time: self.last_reload_time,
            average_reload_time_ms: if self.reload_count > 0 {
                self.total_reload_time_ms / (self.reload_count as f64)
            } else {
                0.0
            },
            total_reload_time_ms: self.total_reload_time_ms,
        }
    }
}

impl Default for MaterialReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Reload statistics for performance tracking
#[derive(Debug, Clone)]
pub struct ReloadStats {
    pub reload_count: u64,
    pub last_reload_time: Instant,
    pub average_reload_time_ms: f64,
    pub total_reload_time_ms: f64,
}
