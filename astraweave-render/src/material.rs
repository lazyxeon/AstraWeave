use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// GPU representation of material properties for shader access
/// Stored in a storage buffer, indexed by material_id
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialGpu {
    /// Packed texture indices: [albedo_idx, normal_idx, orm_idx, unused]
    pub texture_indices: [u32; 4],
    /// Tiling factors: [u_tile, v_tile, triplanar_scale, unused]
    pub tiling_triplanar: [f32; 4],
    /// Material factors: [metallic, roughness, ao, alpha]
    pub factors: [f32; 4],
    /// Flags bitfield (has_albedo, has_normal, has_orm, triplanar, etc.)
    pub flags: u32,
    /// Padding for alignment
    pub _padding: [u32; 3],
}

impl MaterialGpu {
    /// Flag indicating the material has an albedo texture
    pub const FLAG_HAS_ALBEDO: u32 = 1 << 0;
    /// Flag indicating the material has a normal map
    pub const FLAG_HAS_NORMAL: u32 = 1 << 1;
    /// Flag indicating the material has an ORM (occlusion/roughness/metallic) map
    pub const FLAG_HAS_ORM: u32 = 1 << 2;
    /// Flag indicating triplanar projection should be used
    pub const FLAG_TRIPLANAR: u32 = 1 << 3;

    /// Create a neutral/default material with the given layer index
    pub fn neutral(layer_idx: u32) -> Self {
        Self {
            texture_indices: [layer_idx, layer_idx, layer_idx, 0],
            tiling_triplanar: [1.0, 1.0, 16.0, 0.0],
            factors: [0.0, 0.5, 1.0, 1.0], // metallic=0, roughness=0.5, ao=1, alpha=1
            flags: 0,
            _padding: [0; 3],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialLayerDesc {
    pub key: String,
    pub albedo: Option<PathBuf>,
    pub normal: Option<PathBuf>,
    pub mra: Option<PathBuf>,
    pub metallic: Option<PathBuf>,
    pub roughness: Option<PathBuf>,
    pub ao: Option<PathBuf>,
    pub tiling: [f32; 2],
    pub triplanar_scale: f32,
    pub atlas: Option<String>,
}

impl Default for MaterialLayerDesc {
    fn default() -> Self {
        Self {
            key: String::new(),
            albedo: None,
            normal: None,
            mra: None,
            metallic: None,
            roughness: None,
            ao: None,
            tiling: [1.0, 1.0],
            triplanar_scale: 16.0,
            atlas: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MaterialPackDesc {
    pub biome: String,
    pub layers: Vec<MaterialLayerDesc>,
}

#[derive(Clone, Debug, Default)]
pub struct ArrayLayout {
    pub layer_indices: HashMap<String, u32>,
    pub count: u32,
}

pub struct MaterialGpuArrays {
    pub albedo: wgpu::TextureView,
    pub normal: wgpu::TextureView,
    pub mra: wgpu::TextureView,
    pub sampler_albedo: wgpu::Sampler,
    pub sampler_linear: wgpu::Sampler,
    pub layout: ArrayLayout,
    /// Material metadata records (one per layer)
    pub materials: Vec<MaterialGpu>,
    /// GPU storage buffer containing MaterialGpu array
    pub material_buffer: wgpu::Buffer,
}

// ---------------------------------------------------------------------------
// Canonical material UBO (Rust-side counterpart to WGSL `MaterialUbo`)
// ---------------------------------------------------------------------------

/// Rust-side representation of the per-draw material UBO.
///
/// Matches the inline WGSL `MaterialUbo` struct used in both the static and
/// skinned PBR render pipelines. Write to the material uniform buffer via
/// `bytemuck::bytes_of(&ubo)` instead of raw `[f32; 8]` slices.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUboData {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_cutoff: f32,
    pub _pad: f32,
}

impl Default for MaterialUboData {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 1.0,
            alpha_cutoff: 0.5,
            _pad: 0.0,
        }
    }
}

impl MaterialUboData {
    /// Create from base color and PBR parameters.
    pub fn new(base_color: [f32; 4], metallic: f32, roughness: f32, alpha_cutoff: f32) -> Self {
        Self {
            base_color,
            metallic,
            roughness,
            alpha_cutoff,
            _pad: 0.0,
        }
    }
}

impl From<&crate::material_bindless::GpuMaterialEntry> for MaterialUboData {
    fn from(entry: &crate::material_bindless::GpuMaterialEntry) -> Self {
        Self {
            base_color: entry.base_color,
            metallic: entry.metallic_factor,
            roughness: entry.roughness_factor,
            alpha_cutoff: entry.alpha_cutoff,
            _pad: 0.0,
        }
    }
}

impl From<&MaterialGpu> for MaterialUboData {
    fn from(gpu: &MaterialGpu) -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, gpu.factors[3]], // MaterialGpu has no base color RGB
            metallic: gpu.factors[0],
            roughness: gpu.factors[1],
            alpha_cutoff: 0.5, // MaterialGpu has no alpha_cutoff
            _pad: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MaterialLoadStats {
    pub biome: String,
    pub layers_total: usize,
    pub albedo_loaded: usize,
    pub albedo_substituted: usize,
    pub normal_loaded: usize,
    pub normal_substituted: usize,
    pub mra_loaded: usize,
    pub mra_packed: usize,
    pub mra_substituted: usize,
    pub gpu_memory_bytes: u64,
}

impl MaterialLoadStats {
    /// Returns a concise single-line summary suitable for logs/telemetry.
    pub fn concise_summary(&self) -> String {
        format!(
            "[materials] biome={} layers={} | albedo L/S={}/{} | normal L/S={}/{} | mra L+P/S={}+{}/{} | gpu={:.2} MiB",
            self.biome,
            self.layers_total,
            self.albedo_loaded,
            self.albedo_substituted,
            self.normal_loaded,
            self.normal_substituted,
            self.mra_loaded,
            self.mra_packed,
            self.mra_substituted,
            (self.gpu_memory_bytes as f64) / (1024.0 * 1024.0)
        )
    }
}

pub struct MaterialManager {
    // Keep strong refs to textures so views remain valid
    _albedo_tex: Option<wgpu::Texture>,
    _normal_tex: Option<wgpu::Texture>,
    _mra_tex: Option<wgpu::Texture>,
    // Cached bind group layout (created once, reused)
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    // Current GPU arrays and layout
    current_arrays: Option<MaterialGpuArrays>,
    current_stats: Option<MaterialLoadStats>,
}

impl Default for MaterialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialManager {
    pub fn new() -> Self {
        Self {
            _albedo_tex: None,
            _normal_tex: None,
            _mra_tex: None,
            bind_group_layout: None,
            current_arrays: None,
            current_stats: None,
        }
    }

    /// Create or get the cached bind group layout for materials (group 1)
    /// Layout:
    /// - @binding(0): albedo_array (texture_2d_array<f32>, rgba8_srgb)
    /// - @binding(1): sampler (filtering)
    /// - @binding(2): normal_array (texture_2d_array<f32>, rg8_unorm)
    /// - @binding(3): sampler_linear
    /// - @binding(4): mra_array (texture_2d_array<f32>, rgba8_unorm)
    pub fn get_or_create_bind_group_layout(
        &mut self,
        device: &wgpu::Device,
    ) -> &wgpu::BindGroupLayout {
        self.bind_group_layout.get_or_insert_with(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("material-arrays-bgl"),
                entries: &[
                    // 0: albedo array
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1: sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 2: normal array
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 3: sampler_linear
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 4: mra array
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    /// Create a bind group from the current material arrays
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<wgpu::BindGroup> {
        let arrays = self
            .current_arrays
            .as_ref()
            .context("No materials loaded")?;

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material-arrays-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&arrays.albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&arrays.sampler_albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&arrays.normal),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&arrays.sampler_linear),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&arrays.mra),
                },
            ],
        }))
    }

    /// Get current material stats (if loaded)
    pub fn current_stats(&self) -> Option<&MaterialLoadStats> {
        self.current_stats.as_ref()
    }

    /// Get current array layout (if loaded)
    pub fn current_layout(&self) -> Option<&ArrayLayout> {
        self.current_arrays.as_ref().map(|a| &a.layout)
    }

    /// Get reference to albedo texture array (for hot-reload)
    pub fn albedo_texture(&self) -> Option<&wgpu::Texture> {
        self._albedo_tex.as_ref()
    }

    /// Get reference to normal texture array (for hot-reload)
    pub fn normal_texture(&self) -> Option<&wgpu::Texture> {
        self._normal_tex.as_ref()
    }

    /// Get reference to MRA texture array (for hot-reload)
    pub fn mra_texture(&self) -> Option<&wgpu::Texture> {
        self._mra_tex.as_ref()
    }
}

#[cfg(feature = "textures")]
impl MaterialManager {
    /// Load a biome from a directory containing materials.toml and arrays.toml
    /// This is the primary convenience API for loading materials.
    pub async fn load_biome(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        biome_dir: &std::path::Path,
    ) -> Result<MaterialLoadStats> {
        let materials_toml = biome_dir.join("materials.toml");
        let arrays_toml = biome_dir.join("arrays.toml");

        // Validate files exist
        if !materials_toml.exists() {
            anyhow::bail!("materials.toml not found at {}", materials_toml.display());
        }
        if !arrays_toml.exists() {
            anyhow::bail!("arrays.toml not found at {}", arrays_toml.display());
        }

        let (arrays, stats) = self
            .load_pack_from_toml(device, queue, biome_dir, &materials_toml, &arrays_toml)
            .await?;

        self.current_arrays = Some(arrays);
        self.current_stats = Some(stats.clone());

        Ok(stats)
    }

    /// Reload the current biome (hot-reload support)
    pub async fn reload_biome(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        biome_dir: &std::path::Path,
    ) -> Result<MaterialLoadStats> {
        log::info!(
            "[materials] Hot-reloading biome from {}",
            biome_dir.display()
        );
        self.load_biome(device, queue, biome_dir).await
    }

    /// Load a pack from authored TOML files under assets/materials/{biome}
    pub async fn load_pack_from_toml(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        base_dir: &std::path::Path,
        materials_toml: &std::path::Path,
        arrays_toml: &std::path::Path,
    ) -> Result<(MaterialGpuArrays, MaterialLoadStats)> {
        // Parse materials.toml
        #[derive(Deserialize)]
        struct MaterialsDoc {
            biome: BiomeHeader,
            #[serde(default)]
            layer: Vec<MaterialLayerToml>,
        }
        #[derive(Deserialize)]
        struct BiomeHeader {
            name: String,
        }
        #[derive(Deserialize, Default)]
        struct MaterialLayerToml {
            key: String,
            albedo: Option<String>,
            normal: Option<String>,
            mra: Option<String>,
            metallic: Option<String>,
            roughness: Option<String>,
            ao: Option<String>,
            #[serde(default = "default_tiling")]
            tiling: [f32; 2],
            #[serde(default = "default_triplanar")]
            triplanar_scale: f32,
            atlas: Option<String>,
        }
        fn default_tiling() -> [f32; 2] {
            [1.0, 1.0]
        }
        fn default_triplanar() -> f32 {
            16.0
        }

        let mats_src = std::fs::read_to_string(materials_toml)
            .with_context(|| format!("read {}", materials_toml.display()))?;
        let doc: MaterialsDoc = toml::from_str(&mats_src)
            .with_context(|| format!("parse {}", materials_toml.display()))?;

        // Validate biome name
        if doc.biome.name.is_empty() {
            anyhow::bail!("Biome name cannot be empty in {}", materials_toml.display());
        }

        // Parse arrays.toml mapping
        #[derive(Deserialize)]
        struct ArraysDoc {
            layers: HashMap<String, u32>,
        }
        let arrays_src = std::fs::read_to_string(arrays_toml)
            .with_context(|| format!("read {}", arrays_toml.display()))?;
        let arrays: ArraysDoc = toml::from_str(&arrays_src)
            .with_context(|| format!("parse {}", arrays_toml.display()))?;

        // Validate array indices are unique
        let mut index_counts: HashMap<u32, Vec<String>> = HashMap::new();
        for (key, &idx) in &arrays.layers {
            index_counts.entry(idx).or_default().push(key.clone());
        }
        for (idx, keys) in index_counts {
            if keys.len() > 1 {
                anyhow::bail!(
                    "Duplicate array index {} in arrays.toml for keys: {:?}",
                    idx,
                    keys
                );
            }
        }

        let mut layers: Vec<(String, MaterialLayerDesc)> = Vec::new();
        let mut skipped = 0usize;
        for l in doc.layer {
            // Validate layer key
            if l.key.is_empty() {
                log::warn!("[materials] Skipping layer with empty key");
                skipped += 1;
                continue;
            }

            // Validate tiling
            if l.tiling[0] <= 0.0 || l.tiling[1] <= 0.0 {
                log::warn!(
                    "[materials] Warning: Layer '{}' has invalid tiling {:?}, using default",
                    l.key,
                    l.tiling
                );
            }

            if !arrays.layers.contains_key(&l.key) {
                skipped += 1;
                log::warn!(
                    "[materials] arrays.toml missing key '{}' → skip layer",
                    l.key
                );
                continue;
            }
            let to_path =
                |s: Option<String>| -> Option<PathBuf> { s.map(|p| base_dir.join(p).normalize()) };
            let desc = MaterialLayerDesc {
                key: l.key.clone(),
                albedo: to_path(l.albedo),
                normal: to_path(l.normal),
                mra: to_path(l.mra),
                metallic: to_path(l.metallic),
                roughness: to_path(l.roughness),
                ao: to_path(l.ao),
                tiling: l.tiling,
                triplanar_scale: l.triplanar_scale,
                atlas: l.atlas,
            };
            layers.push((l.key, desc));
        }

        // Stable order by arrays mapping index
        layers.sort_by_key(|(k, _)| arrays.layers.get(k).copied().unwrap_or(u32::MAX));

        // Upload into texture arrays (delegated to helper in this module)
        let (gpu, stats, albedo_tex, normal_tex, mra_tex) =
            crate::material_loader::material_loader_impl::build_arrays(
                device,
                queue,
                &layers,
                &arrays.layers,
                &doc.biome.name,
            )?;

        self._albedo_tex = Some(albedo_tex);
        self._normal_tex = Some(normal_tex);
        self._mra_tex = Some(mra_tex);

        if skipped > 0 {
            log::warn!(
                "[materials] skipped {} layers not present in arrays.toml",
                skipped
            );
        }

        Ok((gpu, stats))
    }

    pub fn unload_current(&mut self) {
        self._albedo_tex = None;
        self._normal_tex = None;
        self._mra_tex = None;
        self.current_arrays = None;
        self.current_stats = None;
        log::info!("[materials] Unloaded current biome");
    }
}

#[cfg(not(feature = "textures"))]
impl MaterialManager {
    pub async fn load_pack_from_toml(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _base_dir: &std::path::Path,
        _materials_toml: &std::path::Path,
        _arrays_toml: &std::path::Path,
    ) -> anyhow::Result<(MaterialGpuArrays, MaterialLoadStats)> {
        Err(anyhow::anyhow!(
            "textures feature is disabled; material packs are unavailable"
        ))
    }

    pub fn unload_current(&mut self) { /* no-op */
    }
}

/// Validate a MaterialPackDesc for correctness
pub fn validate_material_pack(pack: &MaterialPackDesc) -> Result<()> {
    // Check biome name is not empty
    if pack.biome.is_empty() {
        anyhow::bail!("Biome name cannot be empty");
    }

    // Check all layers have unique keys
    let mut keys = std::collections::HashSet::new();
    for layer in &pack.layers {
        if layer.key.is_empty() {
            anyhow::bail!("Layer key cannot be empty");
        }
        if !keys.insert(&layer.key) {
            anyhow::bail!("Duplicate layer key: '{}'", layer.key);
        }

        // Validate tiling values are positive
        if layer.tiling[0] <= 0.0 || layer.tiling[1] <= 0.0 {
            anyhow::bail!(
                "Layer '{}': tiling values must be positive, got {:?}",
                layer.key,
                layer.tiling
            );
        }

        // Validate triplanar scale is positive
        if layer.triplanar_scale <= 0.0 {
            anyhow::bail!(
                "Layer '{}': triplanar_scale must be positive, got {}",
                layer.key,
                layer.triplanar_scale
            );
        }

        // Check that at least one texture path is provided
        if layer.albedo.is_none()
            && layer.normal.is_none()
            && layer.mra.is_none()
            && layer.metallic.is_none()
            && layer.roughness.is_none()
            && layer.ao.is_none()
        {
            log::warn!(
                "[materials] Warning: Layer '{}' has no texture paths",
                layer.key
            );
        }
    }

    Ok(())
}

/// Validate array layout for correctness
pub fn validate_array_layout(layout: &ArrayLayout) -> Result<()> {
    // Check for gaps in index space (warn only)
    if layout.count > 0 {
        let max_index = layout.layer_indices.values().max().copied().unwrap_or(0);
        if max_index >= layout.count {
            log::warn!(
                "[materials] Warning: Max index {} >= count {}, possible gap",
                max_index,
                layout.count
            );
        }

        // Check for duplicate indices
        let mut index_counts: HashMap<u32, usize> = HashMap::new();
        for &idx in layout.layer_indices.values() {
            *index_counts.entry(idx).or_insert(0) += 1;
        }
        for (idx, count) in index_counts {
            if count > 1 {
                anyhow::bail!("Duplicate array index {} used {} times", idx, count);
            }
        }
    }

    Ok(())
}

// Small helper to normalize PathBuf joins (remove .. etc.)
// Note: Reserved for future use in path normalization
#[allow(dead_code)]
trait NormalizePath {
    fn normalize(self) -> PathBuf;
}
#[allow(dead_code)]
impl NormalizePath for PathBuf {
    fn normalize(self) -> PathBuf {
        std::path::Path::new(".")
            .join(self)
            .components()
            .as_path()
            .to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_material_layer_desc_default() {
        let desc = MaterialLayerDesc::default();
        assert_eq!(desc.key, "");
        assert!(desc.albedo.is_none());
        assert_eq!(desc.tiling, [1.0, 1.0]);
        assert_eq!(desc.triplanar_scale, 16.0);
    }

    #[test]
    fn test_toml_parsing_basic() -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = r#"
[biome]
name = "test_biome"

[[layer]]
key = "grass"
albedo = "grass_albedo.png"
normal = "grass_normal.png"
mra = "grass_mra.png"
tiling = [2.0, 2.0]
triplanar_scale = 8.0

[[layer]]
key = "dirt"
albedo = "dirt_albedo.png"
"#;
        #[derive(serde::Deserialize)]
        struct MaterialsDoc {
            biome: BiomeHeader,
            layer: Vec<MaterialLayerToml>,
        }
        #[derive(serde::Deserialize)]
        struct BiomeHeader {
            name: String,
        }
        #[derive(serde::Deserialize, Default)]
        #[allow(dead_code)]
        struct MaterialLayerToml {
            key: String,
            albedo: Option<String>,
            normal: Option<String>,
            mra: Option<String>,
            metallic: Option<String>,
            roughness: Option<String>,
            ao: Option<String>,
            tiling: Option<[f32; 2]>,
            triplanar_scale: Option<f32>,
            atlas: Option<String>,
        }

        let doc: MaterialsDoc =
            toml::from_str(toml_str).map_err(|e| anyhow::anyhow!("TOML parse error: {}", e))?;
        assert_eq!(doc.biome.name, "test_biome");
        assert_eq!(doc.layer.len(), 2);
        assert_eq!(doc.layer[0].key, "grass");
        assert_eq!(doc.layer[0].albedo, Some("grass_albedo.png".to_string()));
        assert_eq!(doc.layer[0].tiling, Some([2.0, 2.0]));
        assert_eq!(doc.layer[1].key, "dirt");
        assert_eq!(doc.layer[1].normal, None);
        Ok(())
    }

    #[test]
    fn test_arrays_toml_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = r#"
[layers]
grass = 0
dirt = 1
stone = 2
"#;
        #[derive(serde::Deserialize)]
        struct ArraysDoc {
            layers: std::collections::HashMap<String, u32>,
        }
        let arrays: ArraysDoc =
            toml::from_str(toml_str).map_err(|e| anyhow::anyhow!("TOML parse error: {}", e))?;
        assert_eq!(arrays.layers.get("grass"), Some(&0));
        assert_eq!(arrays.layers.get("dirt"), Some(&1));
        assert_eq!(arrays.layers.get("stone"), Some(&2));
        Ok(())
    }

    #[test]
    fn test_stable_layer_index_mapping() {
        // Simulate layers and arrays mapping
        let mut layers = vec![
            (
                "stone".to_string(),
                MaterialLayerDesc {
                    key: "stone".to_string(),
                    ..Default::default()
                },
            ),
            (
                "grass".to_string(),
                MaterialLayerDesc {
                    key: "grass".to_string(),
                    ..Default::default()
                },
            ),
            (
                "dirt".to_string(),
                MaterialLayerDesc {
                    key: "dirt".to_string(),
                    ..Default::default()
                },
            ),
        ];
        let arrays_layers = std::collections::HashMap::from([
            ("grass".to_string(), 0),
            ("dirt".to_string(), 1),
            ("stone".to_string(), 2),
        ]);

        // Sort by index
        layers.sort_by_key(|(k, _)| arrays_layers.get(k).copied().unwrap_or(u32::MAX));

        assert_eq!(layers[0].0, "grass");
        assert_eq!(layers[1].0, "dirt");
        assert_eq!(layers[2].0, "stone");
    }

    #[test]
    fn test_fallback_coverage() {
        // Test that missing paths are handled
        let desc = MaterialLayerDesc {
            key: "test".to_string(),
            albedo: None,
            normal: Some(PathBuf::from("normal.png")),
            mra: None,
            ..Default::default()
        };
        // In real loading, fallbacks would be applied in build_arrays
        // Here, just check the desc has None where expected
        assert!(desc.albedo.is_none());
        assert!(desc.mra.is_none());
        assert!(desc.normal.is_some());
    }

    #[test]
    fn test_material_load_stats_concise_summary() {
        let stats = MaterialLoadStats {
            biome: "forest".to_string(),
            layers_total: 5,
            albedo_loaded: 3,
            albedo_substituted: 2,
            normal_loaded: 4,
            normal_substituted: 1,
            mra_loaded: 2,
            mra_packed: 1,
            mra_substituted: 2,
            gpu_memory_bytes: 1024 * 1024 * 10, // 10 MiB
        };
        let summary = stats.concise_summary();
        assert!(summary.contains("biome=forest"));
        assert!(summary.contains("layers=5"));
        assert!(summary.contains("albedo L/S=3/2"));
        assert!(summary.contains("gpu=10.00 MiB"));
    }

    #[test]
    fn test_validate_material_pack_empty_biome() {
        let pack = MaterialPackDesc {
            biome: String::new(),
            layers: vec![],
        };
        assert!(validate_material_pack(&pack).is_err());
    }

    #[test]
    fn test_validate_material_pack_duplicate_keys() {
        let pack = MaterialPackDesc {
            biome: "test".to_string(),
            layers: vec![
                MaterialLayerDesc {
                    key: "grass".to_string(),
                    ..Default::default()
                },
                MaterialLayerDesc {
                    key: "grass".to_string(),
                    ..Default::default()
                },
            ],
        };
        assert!(validate_material_pack(&pack).is_err());
    }

    #[test]
    fn test_validate_material_pack_invalid_tiling() {
        let pack = MaterialPackDesc {
            biome: "test".to_string(),
            layers: vec![MaterialLayerDesc {
                key: "grass".to_string(),
                tiling: [-1.0, 2.0],
                ..Default::default()
            }],
        };
        assert!(validate_material_pack(&pack).is_err());
    }

    #[test]
    fn test_validate_material_pack_invalid_triplanar() {
        let pack = MaterialPackDesc {
            biome: "test".to_string(),
            layers: vec![MaterialLayerDesc {
                key: "grass".to_string(),
                triplanar_scale: -5.0,
                ..Default::default()
            }],
        };
        assert!(validate_material_pack(&pack).is_err());
    }

    #[test]
    fn test_validate_material_pack_valid() {
        let pack = MaterialPackDesc {
            biome: "forest".to_string(),
            layers: vec![
                MaterialLayerDesc {
                    key: "grass".to_string(),
                    albedo: Some(PathBuf::from("grass.png")),
                    tiling: [2.0, 2.0],
                    triplanar_scale: 16.0,
                    ..Default::default()
                },
                MaterialLayerDesc {
                    key: "dirt".to_string(),
                    normal: Some(PathBuf::from("dirt_n.png")),
                    ..Default::default()
                },
            ],
        };
        assert!(validate_material_pack(&pack).is_ok());
    }

    #[test]
    fn test_validate_array_layout_duplicate_indices() {
        let mut layout = ArrayLayout {
            layer_indices: HashMap::new(),
            count: 3,
        };
        layout.layer_indices.insert("grass".to_string(), 0);
        layout.layer_indices.insert("dirt".to_string(), 0); // Duplicate!

        assert!(validate_array_layout(&layout).is_err());
    }

    #[test]
    fn test_validate_array_layout_valid() {
        let mut layout = ArrayLayout {
            layer_indices: HashMap::new(),
            count: 3,
        };
        layout.layer_indices.insert("grass".to_string(), 0);
        layout.layer_indices.insert("dirt".to_string(), 1);
        layout.layer_indices.insert("stone".to_string(), 2);

        assert!(validate_array_layout(&layout).is_ok());
    }
}
