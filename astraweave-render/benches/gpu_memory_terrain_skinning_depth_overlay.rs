// v5.34: GPU Memory Budget, Terrain Materials, Skinning GPU, Depth Buffers, Overlay Effects
//
// Comprehensive benchmarks for GPU resource management, terrain PBR layers,
// skeletal animation pipelines, depth buffer operations, and cinematic overlay effects.
//
// Benchmark Groups:
// 1. GPU Memory Budget - Category tracking, budget allocation, pressure events
// 2. Terrain Materials - Layer GPU, splat maps, triplanar blending
// 3. Skinning GPU Pipeline - Joint palettes, matrix uploads, bind groups
// 4. Depth Buffer Operations - Creation, view generation, format handling
// 5. Overlay Effects - Fade/letterbox params, effect configuration
// 6. Advanced Post-Processing - TAA, Motion Blur, DOF, Color Grading config
// 7. Combined Scenarios - Full frame setup, pipeline integration

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box;

// ============================================================================
// MOCK TYPES FOR GPU MEMORY BUDGET BENCHMARKS
// ============================================================================

/// Memory allocation categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MemoryCategory {
    Geometry = 0,
    Textures = 1,
    RenderTargets = 2,
    Uniforms = 3,
    Staging = 4,
    Shadows = 5,
    Environment = 6,
    Other = 7,
}

impl MemoryCategory {
    pub const ALL: [MemoryCategory; 8] = [
        MemoryCategory::Geometry,
        MemoryCategory::Textures,
        MemoryCategory::RenderTargets,
        MemoryCategory::Uniforms,
        MemoryCategory::Staging,
        MemoryCategory::Shadows,
        MemoryCategory::Environment,
        MemoryCategory::Other,
    ];

    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}

/// Budget configuration for a memory category
#[derive(Debug, Clone, Copy)]
pub struct CategoryBudget {
    pub soft_limit: u64,
    pub hard_limit: u64,
    pub current: u64,
}

impl Default for CategoryBudget {
    fn default() -> Self {
        Self {
            soft_limit: 256 * 1024 * 1024, // 256 MB soft
            hard_limit: 512 * 1024 * 1024, // 512 MB hard
            current: 0,
        }
    }
}

impl CategoryBudget {
    #[inline]
    pub fn remaining(&self) -> u64 {
        self.hard_limit.saturating_sub(self.current)
    }

    #[inline]
    pub fn usage_ratio(&self) -> f32 {
        if self.hard_limit == 0 {
            0.0
        } else {
            self.current as f32 / self.hard_limit as f32
        }
    }

    #[inline]
    pub fn is_over_soft_limit(&self) -> bool {
        self.current > self.soft_limit
    }

    #[inline]
    pub fn can_allocate(&self, size: u64) -> bool {
        self.current.saturating_add(size) <= self.hard_limit
    }
}

/// Budget event types
#[derive(Debug, Clone)]
pub enum BudgetEvent {
    SoftLimitExceeded {
        category: MemoryCategory,
        current: u64,
        limit: u64,
    },
    HardLimitBlocked {
        category: MemoryCategory,
        requested: u64,
        available: u64,
    },
    MemoryPressure {
        total_used: u64,
        total_budget: u64,
        percentage: f32,
    },
}

/// GPU Memory Budget Manager (mock for benchmarks)
pub struct MockGpuMemoryBudget {
    budgets: [CategoryBudget; 8],
    total_used: u64,
    total_budget: u64,
    pressure_threshold: f32,
}

impl Default for MockGpuMemoryBudget {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGpuMemoryBudget {
    pub fn new() -> Self {
        Self {
            budgets: [CategoryBudget::default(); 8],
            total_used: 0,
            total_budget: 2 * 1024 * 1024 * 1024, // 2 GB
            pressure_threshold: 0.85,
        }
    }

    pub fn with_total_budget(total_bytes: u64) -> Self {
        let mut mgr = Self::new();
        mgr.total_budget = total_bytes;
        let per_category = total_bytes / 8;
        for budget in &mut mgr.budgets {
            budget.soft_limit = (per_category as f64 * 0.75) as u64;
            budget.hard_limit = per_category;
        }
        // Give extra to textures
        mgr.budgets[MemoryCategory::Textures.index()].soft_limit =
            (total_bytes as f64 * 0.3) as u64;
        mgr.budgets[MemoryCategory::Textures.index()].hard_limit =
            (total_bytes as f64 * 0.4) as u64;
        mgr
    }

    #[inline]
    pub fn try_allocate(&mut self, category: MemoryCategory, size: u64) -> Result<(), BudgetEvent> {
        let budget = &mut self.budgets[category.index()];
        if !budget.can_allocate(size) {
            return Err(BudgetEvent::HardLimitBlocked {
                category,
                requested: size,
                available: budget.remaining(),
            });
        }
        budget.current += size;
        self.total_used += size;

        if budget.is_over_soft_limit() {
            // Could emit soft limit event here, but for benchmark we just continue
        }

        Ok(())
    }

    #[inline]
    pub fn free(&mut self, category: MemoryCategory, size: u64) {
        let budget = &mut self.budgets[category.index()];
        budget.current = budget.current.saturating_sub(size);
        self.total_used = self.total_used.saturating_sub(size);
    }

    #[inline]
    pub fn get_category_usage(&self, category: MemoryCategory) -> u64 {
        self.budgets[category.index()].current
    }

    #[inline]
    pub fn get_total_usage(&self) -> u64 {
        self.total_used
    }

    #[inline]
    pub fn pressure_level(&self) -> f32 {
        if self.total_budget == 0 {
            0.0
        } else {
            self.total_used as f32 / self.total_budget as f32
        }
    }

    #[inline]
    pub fn is_under_pressure(&self) -> bool {
        self.pressure_level() > self.pressure_threshold
    }

    pub fn category_report(&self) -> [(MemoryCategory, u64, f32); 8] {
        let mut report = [(MemoryCategory::Geometry, 0u64, 0.0f32); 8];
        for (i, cat) in MemoryCategory::ALL.iter().enumerate() {
            let budget = &self.budgets[i];
            report[i] = (*cat, budget.current, budget.usage_ratio());
        }
        report
    }
}

// ============================================================================
// MOCK TYPES FOR TERRAIN MATERIALS BENCHMARKS
// ============================================================================

/// GPU representation of a single terrain layer (64 bytes)
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainLayerGpu {
    pub texture_indices: [u32; 4],  // 16 bytes
    pub uv_scale: [f32; 2],         // 8 bytes
    pub height_range: [f32; 2],     // 8 bytes
    pub blend_sharpness: f32,       // 4 bytes
    pub triplanar_power: f32,       // 4 bytes
    pub material_factors: [f32; 2], // 8 bytes
    pub _pad: [u32; 4],             // 16 bytes
}

impl TerrainLayerGpu {
    pub const SIZE: usize = 64;

    pub fn new(
        albedo_idx: u32,
        normal_idx: u32,
        orm_idx: u32,
        height_idx: u32,
        uv_scale: [f32; 2],
    ) -> Self {
        Self {
            texture_indices: [albedo_idx, normal_idx, orm_idx, height_idx],
            uv_scale,
            height_range: [0.0, 100.0],
            blend_sharpness: 0.5,
            triplanar_power: 4.0,
            material_factors: [0.0, 0.5],
            _pad: [0; 4],
        }
    }

    #[inline]
    pub fn set_height_blend(&mut self, min_height: f32, max_height: f32, sharpness: f32) {
        self.height_range = [min_height, max_height];
        self.blend_sharpness = sharpness.clamp(0.0, 1.0);
    }

    #[inline]
    pub fn set_material(&mut self, metallic: f32, roughness: f32) {
        self.material_factors = [metallic.clamp(0.0, 1.0), roughness.clamp(0.0, 1.0)];
    }
}

/// Extended terrain material with 8 layers (576 bytes)
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct TerrainMaterialGpu {
    pub layers: [TerrainLayerGpu; 8],   // 512 bytes
    pub splat_map_index_0: u32,         // 4 bytes
    pub splat_map_index_1: u32,         // 4 bytes
    pub splat_uv_scale: f32,            // 4 bytes
    pub triplanar_enabled: u32,         // 4 bytes
    pub normal_blend_method: u32,       // 4 bytes
    pub triplanar_slope_threshold: f32, // 4 bytes
    pub height_blend_enabled: u32,      // 4 bytes
    pub active_layer_count: u32,        // 4 bytes
    pub _pad: [u32; 8],                 // 32 bytes
}

impl Default for TerrainMaterialGpu {
    fn default() -> Self {
        Self {
            layers: [TerrainLayerGpu::default(); 8],
            splat_map_index_0: 0,
            splat_map_index_1: 0,
            splat_uv_scale: 1.0,
            triplanar_enabled: 1,
            normal_blend_method: 1, // RNM
            triplanar_slope_threshold: 45.0,
            height_blend_enabled: 1,
            active_layer_count: 4,
            _pad: [0; 8],
        }
    }
}

impl TerrainMaterialGpu {
    pub const SIZE: usize = 576;

    pub fn with_layers(layers: [TerrainLayerGpu; 8]) -> Self {
        Self {
            layers,
            ..Default::default()
        }
    }

    #[inline]
    pub fn get_layer(&self, index: usize) -> Option<&TerrainLayerGpu> {
        self.layers.get(index)
    }

    #[inline]
    pub fn set_splat_map(&mut self, index: u32, uv_scale: f32) {
        self.splat_map_index_0 = index;
        self.splat_uv_scale = uv_scale;
    }

    #[inline]
    pub fn set_triplanar(&mut self, enabled: bool, slope_threshold: f32) {
        self.triplanar_enabled = enabled as u32;
        self.triplanar_slope_threshold = slope_threshold;
    }

    /// Calculate splat weight from RGBA (layer0, layer1, layer2, layer3)
    #[inline]
    pub fn compute_layer_weight(splat_rgba: [f32; 4], layer_index: usize) -> f32 {
        if layer_index < 4 {
            splat_rgba[layer_index]
        } else {
            0.0
        }
    }

    /// Normalize splat weights to sum to 1.0
    #[inline]
    pub fn normalize_weights(weights: &mut [f32; 4]) {
        let sum: f32 = weights.iter().sum();
        if sum > 0.0001 {
            for w in weights.iter_mut() {
                *w /= sum;
            }
        }
    }
}

// ============================================================================
// MOCK TYPES FOR SKINNING GPU BENCHMARKS
// ============================================================================

/// Handle for a joint palette buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointPaletteHandle(pub u32);

/// Maximum joints per skeleton
pub const MAX_JOINTS: usize = 256;

/// Joint palette data (256 matrices, 16KB)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JointPalette {
    pub matrices: [[f32; 16]; MAX_JOINTS],
    pub joint_count: u32,
    pub _pad: [u32; 3],
}

impl Default for JointPalette {
    fn default() -> Self {
        Self {
            matrices: [[0.0; 16]; MAX_JOINTS],
            joint_count: 0,
            _pad: [0; 3],
        }
    }
}

impl JointPalette {
    pub const SIZE: usize = MAX_JOINTS * 64 + 16; // 16400 bytes

    pub fn from_identity(count: usize) -> Self {
        let count = count.min(MAX_JOINTS);
        let mut palette = Self {
            joint_count: count as u32,
            ..Default::default()
        };

        // Identity matrix
        let identity: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        for i in 0..count {
            palette.matrices[i] = identity;
        }
        palette
    }

    #[inline]
    pub fn set_matrix(&mut self, index: usize, matrix: [f32; 16]) {
        if index < MAX_JOINTS {
            self.matrices[index] = matrix;
        }
    }

    #[inline]
    pub fn get_matrix(&self, index: usize) -> Option<[f32; 16]> {
        if index < self.joint_count as usize {
            Some(self.matrices[index])
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

/// Mock joint palette manager for benchmarks
pub struct MockJointPaletteManager {
    palettes: HashMap<JointPaletteHandle, JointPalette>,
    next_handle: u32,
}

impl Default for MockJointPaletteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MockJointPaletteManager {
    pub fn new() -> Self {
        Self {
            palettes: HashMap::new(),
            next_handle: 0,
        }
    }

    #[inline]
    pub fn allocate(&mut self) -> JointPaletteHandle {
        let handle = JointPaletteHandle(self.next_handle);
        self.next_handle += 1;
        self.palettes.insert(handle, JointPalette::default());
        handle
    }

    #[inline]
    pub fn upload(&mut self, handle: JointPaletteHandle, palette: JointPalette) -> bool {
        if let Some(p) = self.palettes.get_mut(&handle) {
            *p = palette;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn get(&self, handle: JointPaletteHandle) -> Option<&JointPalette> {
        self.palettes.get(&handle)
    }

    #[inline]
    pub fn free(&mut self, handle: JointPaletteHandle) {
        self.palettes.remove(&handle);
    }

    pub fn count(&self) -> usize {
        self.palettes.len()
    }
}

// ============================================================================
// MOCK TYPES FOR DEPTH BUFFER BENCHMARKS
// ============================================================================

/// Depth buffer format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthFormat {
    Depth16Unorm,
    Depth24Plus,
    Depth24PlusStencil8,
    Depth32Float,
    Depth32FloatStencil8,
}

impl DepthFormat {
    #[inline]
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            DepthFormat::Depth16Unorm => 2,
            DepthFormat::Depth24Plus => 4,
            DepthFormat::Depth24PlusStencil8 => 4,
            DepthFormat::Depth32Float => 4,
            DepthFormat::Depth32FloatStencil8 => 8,
        }
    }

    #[inline]
    pub fn has_stencil(&self) -> bool {
        matches!(
            self,
            DepthFormat::Depth24PlusStencil8 | DepthFormat::Depth32FloatStencil8
        )
    }
}

/// Depth buffer descriptor
#[derive(Debug, Clone)]
pub struct DepthDesc {
    pub width: u32,
    pub height: u32,
    pub format: DepthFormat,
    pub sample_count: u32,
}

impl DepthDesc {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: DepthFormat::Depth32Float,
            sample_count: 1,
        }
    }

    #[inline]
    pub fn memory_size(&self) -> u64 {
        self.width as u64
            * self.height as u64
            * self.format.bytes_per_pixel() as u64
            * self.sample_count as u64
    }

    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn with_msaa(mut self, samples: u32) -> Self {
        self.sample_count = samples;
        self
    }
}

/// Mock depth buffer
pub struct MockDepthBuffer {
    pub desc: DepthDesc,
    pub clear_value: f32,
}

impl MockDepthBuffer {
    pub fn new(desc: DepthDesc) -> Self {
        Self {
            desc,
            clear_value: 1.0,
        }
    }

    #[inline]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.desc.width = width;
        self.desc.height = height;
    }

    #[inline]
    pub fn memory_size(&self) -> u64 {
        self.desc.memory_size()
    }
}

// ============================================================================
// MOCK TYPES FOR OVERLAY EFFECTS BENCHMARKS
// ============================================================================

/// Overlay effect parameters (16 bytes GPU aligned)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OverlayParams {
    pub fade: f32,      // 0..1 black fade
    pub letterbox: f32, // 0..0.45 fraction of screen height for bars
    pub vignette: f32,  // 0..1 vignette intensity
    pub chromatic: f32, // 0..1 chromatic aberration
}

impl OverlayParams {
    pub const SIZE: usize = 16;

    #[inline]
    pub fn set_fade(&mut self, fade: f32) {
        self.fade = fade.clamp(0.0, 1.0);
    }

    #[inline]
    pub fn set_letterbox(&mut self, ratio: f32) {
        self.letterbox = ratio.clamp(0.0, 0.45);
    }

    #[inline]
    pub fn set_vignette(&mut self, intensity: f32) {
        self.vignette = intensity.clamp(0.0, 1.0);
    }

    #[inline]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            fade: self.fade + (other.fade - self.fade) * t,
            letterbox: self.letterbox + (other.letterbox - self.letterbox) * t,
            vignette: self.vignette + (other.vignette - self.vignette) * t,
            chromatic: self.chromatic + (other.chromatic - self.chromatic) * t,
        }
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.fade > 0.001
            || self.letterbox > 0.001
            || self.vignette > 0.001
            || self.chromatic > 0.001
    }

    pub fn as_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.fade.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.letterbox.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.vignette.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.chromatic.to_le_bytes());
        bytes
    }
}

// ============================================================================
// MOCK TYPES FOR ADVANCED POST-PROCESSING BENCHMARKS
// ============================================================================

/// TAA configuration
#[derive(Debug, Clone, Copy)]
pub struct TaaConfig {
    pub enabled: bool,
    pub blend_factor: f32,
    pub jitter_scale: f32,
}

impl Default for TaaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            blend_factor: 0.95,
            jitter_scale: 1.0,
        }
    }
}

impl TaaConfig {
    /// Halton sequence for jitter
    #[inline]
    pub fn halton_jitter(&self, frame: u32, width: u32, height: u32) -> (f32, f32) {
        let h2 = halton_sequence(frame, 2);
        let h3 = halton_sequence(frame, 3);
        let jx = (h2 - 0.5) * self.jitter_scale * 2.0 / width as f32;
        let jy = (h3 - 0.5) * self.jitter_scale * 2.0 / height as f32;
        (jx, jy)
    }
}

/// Motion blur configuration
#[derive(Debug, Clone, Copy)]
pub struct MotionBlurConfig {
    pub enabled: bool,
    pub sample_count: u32,
    pub strength: f32,
    pub max_blur_radius: f32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_count: 8,
            strength: 1.0,
            max_blur_radius: 32.0,
        }
    }
}

/// DOF configuration
#[derive(Debug, Clone, Copy)]
pub struct DofConfig {
    pub enabled: bool,
    pub focus_distance: f32,
    pub focus_range: f32,
    pub bokeh_size: f32,
    pub near_blur_scale: f32,
    pub far_blur_scale: f32,
}

impl Default for DofConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 10.0,
            focus_range: 5.0,
            bokeh_size: 2.0,
            near_blur_scale: 1.0,
            far_blur_scale: 1.0,
        }
    }
}

impl DofConfig {
    /// Calculate COC (circle of confusion) for a given depth
    #[inline]
    pub fn calculate_coc(&self, depth: f32) -> f32 {
        let near_focus = self.focus_distance - self.focus_range * 0.5;
        let far_focus = self.focus_distance + self.focus_range * 0.5;

        if depth < near_focus {
            ((near_focus - depth) / near_focus * self.near_blur_scale).min(self.bokeh_size)
        } else if depth > far_focus {
            ((depth - far_focus) / far_focus * self.far_blur_scale).min(self.bokeh_size)
        } else {
            0.0 // In focus
        }
    }
}

/// Color grading configuration
#[derive(Debug, Clone, Copy)]
pub struct ColorGradingConfig {
    pub enabled: bool,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
    pub shadows: [f32; 3],
    pub midtones: [f32; 3],
    pub highlights: [f32; 3],
}

impl Default for ColorGradingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            shadows: [0.0, 0.0, 0.0],
            midtones: [0.0, 0.0, 0.0],
            highlights: [0.0, 0.0, 0.0],
        }
    }
}

impl ColorGradingConfig {
    /// Apply color grade to linear RGB
    #[inline]
    pub fn apply(&self, color: [f32; 3]) -> [f32; 3] {
        // Exposure
        let mut c = [
            color[0] * (2.0f32).powf(self.exposure),
            color[1] * (2.0f32).powf(self.exposure),
            color[2] * (2.0f32).powf(self.exposure),
        ];

        // Contrast
        c = [
            (c[0] - 0.5) * self.contrast + 0.5,
            (c[1] - 0.5) * self.contrast + 0.5,
            (c[2] - 0.5) * self.contrast + 0.5,
        ];

        // Saturation
        let luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
        c = [
            luma + (c[0] - luma) * self.saturation,
            luma + (c[1] - luma) * self.saturation,
            luma + (c[2] - luma) * self.saturation,
        ];

        c
    }
}

/// Combined post-process config
#[derive(Debug, Clone, Default)]
pub struct PostProcessConfig {
    pub taa: TaaConfig,
    pub motion_blur: MotionBlurConfig,
    pub dof: DofConfig,
    pub color_grading: ColorGradingConfig,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Halton sequence for low-discrepancy sampling
#[inline]
fn halton_sequence(index: u32, base: u32) -> f32 {
    let mut result = 0.0f32;
    let mut f = 1.0f32 / base as f32;
    let mut i = index;
    while i > 0 {
        result += f * (i % base) as f32;
        i /= base;
        f /= base as f32;
    }
    result
}

// ============================================================================
// BENCHMARK FUNCTIONS
// ============================================================================

fn bench_gpu_memory_budget(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_memory_budget");

    // 1. Budget manager creation
    group.bench_function("manager_creation_default", |b| {
        b.iter(|| black_box(MockGpuMemoryBudget::new()));
    });

    // 2. Budget manager with custom size
    group.bench_function("manager_creation_custom_4gb", |b| {
        b.iter(|| {
            black_box(MockGpuMemoryBudget::with_total_budget(
                4 * 1024 * 1024 * 1024,
            ))
        });
    });

    // 3. Category budget remaining calculation
    group.bench_function("category_remaining", |b| {
        let budget = CategoryBudget {
            soft_limit: 256 * 1024 * 1024,
            hard_limit: 512 * 1024 * 1024,
            current: 128 * 1024 * 1024,
        };
        b.iter(|| black_box(budget.remaining()));
    });

    // 4. Usage ratio calculation
    group.bench_function("usage_ratio", |b| {
        let budget = CategoryBudget {
            soft_limit: 256 * 1024 * 1024,
            hard_limit: 512 * 1024 * 1024,
            current: 384 * 1024 * 1024,
        };
        b.iter(|| black_box(budget.usage_ratio()));
    });

    // 5. Allocation check
    group.bench_function("can_allocate_check", |b| {
        let budget = CategoryBudget {
            soft_limit: 256 * 1024 * 1024,
            hard_limit: 512 * 1024 * 1024,
            current: 256 * 1024 * 1024,
        };
        let size = 64 * 1024 * 1024u64;
        b.iter(|| black_box(budget.can_allocate(black_box(size))));
    });

    // 6. Try allocate operation
    group.bench_function("try_allocate", |b| {
        let mut mgr = MockGpuMemoryBudget::new();
        let size = 16 * 1024 * 1024u64;
        b.iter(|| {
            let _ = mgr.try_allocate(black_box(MemoryCategory::Textures), black_box(size));
            mgr.free(MemoryCategory::Textures, size); // Reset for next iteration
        });
    });

    // 7. Free operation
    group.bench_function("free", |b| {
        let mut mgr = MockGpuMemoryBudget::new();
        let _ = mgr.try_allocate(MemoryCategory::Geometry, 100 * 1024 * 1024);
        b.iter(|| {
            mgr.free(
                black_box(MemoryCategory::Geometry),
                black_box(1024 * 1024u64),
            );
        });
    });

    // 8. Pressure level calculation
    group.bench_function("pressure_level", |b| {
        let mut mgr = MockGpuMemoryBudget::new();
        let _ = mgr.try_allocate(MemoryCategory::Textures, 1024 * 1024 * 1024);
        b.iter(|| black_box(mgr.pressure_level()));
    });

    // 9. Category iteration
    group.bench_function("category_all_iteration", |b| {
        b.iter(|| {
            let mut sum = 0u8;
            for cat in MemoryCategory::ALL.iter() {
                sum = sum.wrapping_add(cat.index() as u8);
            }
            black_box(sum)
        });
    });

    // 10. Full category report
    group.bench_function("category_report", |b| {
        let mut mgr = MockGpuMemoryBudget::new();
        let _ = mgr.try_allocate(MemoryCategory::Textures, 256 * 1024 * 1024);
        let _ = mgr.try_allocate(MemoryCategory::Geometry, 128 * 1024 * 1024);
        b.iter(|| black_box(mgr.category_report()));
    });

    // 11. Allocation throughput (multiple categories)
    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count));
        group.bench_with_input(
            BenchmarkId::new("allocation_throughput", count),
            &count,
            |b, &count| {
                let mut mgr = MockGpuMemoryBudget::with_total_budget(16 * 1024 * 1024 * 1024); // 16GB to avoid limits
                let size = 1024u64;
                b.iter(|| {
                    for i in 0..count {
                        let cat = MemoryCategory::ALL[(i % 8) as usize];
                        let _ = mgr.try_allocate(cat, size);
                    }
                    // Reset
                    for cat in MemoryCategory::ALL {
                        mgr.free(cat, size * count / 8);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_terrain_materials(c: &mut Criterion) {
    let mut group = c.benchmark_group("terrain_materials");

    // 1. Layer GPU creation (default)
    group.bench_function("layer_gpu_default", |b| {
        b.iter(|| black_box(TerrainLayerGpu::default()));
    });

    // 2. Layer GPU creation (custom)
    group.bench_function("layer_gpu_new", |b| {
        b.iter(|| {
            black_box(TerrainLayerGpu::new(
                black_box(0),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box([2.0, 2.0]),
            ))
        });
    });

    // 3. Layer height blend setup
    group.bench_function("layer_set_height_blend", |b| {
        b.iter(|| {
            let mut layer = TerrainLayerGpu::default();
            layer.set_height_blend(black_box(0.0), black_box(50.0), black_box(0.7));
            black_box(layer)
        });
    });

    // 4. Layer material setup
    group.bench_function("layer_set_material", |b| {
        b.iter(|| {
            let mut layer = TerrainLayerGpu::default();
            layer.set_material(black_box(0.2), black_box(0.6));
            black_box(layer)
        });
    });

    // 5. Terrain material GPU default
    group.bench_function("terrain_material_default", |b| {
        b.iter(|| black_box(TerrainMaterialGpu::default()));
    });

    // 6. Terrain material with layers
    group.bench_function("terrain_material_with_layers", |b| {
        let layers = [
            TerrainLayerGpu::new(0, 1, 2, 3, [1.0, 1.0]),
            TerrainLayerGpu::new(4, 5, 6, 7, [2.0, 2.0]),
            TerrainLayerGpu::new(8, 9, 10, 11, [4.0, 4.0]),
            TerrainLayerGpu::new(12, 13, 14, 15, [8.0, 8.0]),
            TerrainLayerGpu::default(),
            TerrainLayerGpu::default(),
            TerrainLayerGpu::default(),
            TerrainLayerGpu::default(),
        ];
        b.iter(|| black_box(TerrainMaterialGpu::with_layers(black_box(layers))));
    });

    // 7. Layer access
    group.bench_function("get_layer", |b| {
        let mat = TerrainMaterialGpu::default();
        b.iter(|| black_box(mat.get_layer(black_box(2))));
    });

    // 8. Splat map setup
    group.bench_function("set_splat_map", |b| {
        b.iter(|| {
            let mut mat = TerrainMaterialGpu::default();
            mat.set_splat_map(black_box(5), black_box(0.5));
            black_box(mat)
        });
    });

    // 9. Triplanar setup
    group.bench_function("set_triplanar", |b| {
        b.iter(|| {
            let mut mat = TerrainMaterialGpu::default();
            mat.set_triplanar(black_box(true), black_box(60.0));
            black_box(mat)
        });
    });

    // 10. Layer weight computation
    group.bench_function("compute_layer_weight", |b| {
        let splat = [0.3f32, 0.4, 0.2, 0.1];
        b.iter(|| {
            black_box(TerrainMaterialGpu::compute_layer_weight(
                black_box(splat),
                black_box(1),
            ))
        });
    });

    // 11. Weight normalization
    group.bench_function("normalize_weights", |b| {
        let mut weights = [0.3f32, 0.5, 0.2, 0.4];
        b.iter(|| {
            TerrainMaterialGpu::normalize_weights(black_box(&mut weights));
            black_box(weights)
        });
    });

    // 12. Size constants
    group.bench_function("size_constant_layer", |b| {
        b.iter(|| black_box(TerrainLayerGpu::SIZE));
    });

    group.bench_function("size_constant_material", |b| {
        b.iter(|| black_box(TerrainMaterialGpu::SIZE));
    });

    group.finish();
}

fn bench_skinning_gpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("skinning_gpu");

    // 1. Joint palette handle creation
    group.bench_function("handle_creation", |b| {
        b.iter(|| black_box(JointPaletteHandle(black_box(42))));
    });

    // 2. Joint palette default creation
    group.bench_function("palette_default", |b| {
        b.iter(|| black_box(JointPalette::default()));
    });

    // 3. Joint palette from identity (varying joint counts)
    for count in [16, 64, 128, 256] {
        group.bench_with_input(
            BenchmarkId::new("palette_from_identity", count),
            &count,
            |b, &count| {
                b.iter(|| black_box(JointPalette::from_identity(black_box(count))));
            },
        );
    }

    // 4. Set matrix operation
    group.bench_function("set_matrix", |b| {
        let mut palette = JointPalette::from_identity(64);
        let matrix: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 2.0, 3.0, 1.0,
        ];
        b.iter(|| {
            palette.set_matrix(black_box(32), black_box(matrix));
        });
    });

    // 5. Get matrix operation
    group.bench_function("get_matrix", |b| {
        let palette = JointPalette::from_identity(128);
        b.iter(|| black_box(palette.get_matrix(black_box(64))));
    });

    // 6. Palette as bytes
    group.bench_function("palette_as_bytes", |b| {
        let palette = JointPalette::from_identity(128);
        b.iter(|| {
            let bytes = palette.as_bytes();
            black_box(bytes.len())
        });
    });

    // 7. Manager creation
    group.bench_function("manager_creation", |b| {
        b.iter(|| black_box(MockJointPaletteManager::new()));
    });

    // 8. Manager allocate
    group.bench_function("manager_allocate", |b| {
        let mut mgr = MockJointPaletteManager::new();
        b.iter(|| black_box(mgr.allocate()));
    });

    // 9. Manager upload
    group.bench_function("manager_upload", |b| {
        let mut mgr = MockJointPaletteManager::new();
        let handle = mgr.allocate();
        let palette = JointPalette::from_identity(64);
        b.iter(|| black_box(mgr.upload(black_box(handle), black_box(palette))));
    });

    // 10. Manager get
    group.bench_function("manager_get", |b| {
        let mut mgr = MockJointPaletteManager::new();
        let handle = mgr.allocate();
        let _ = mgr.upload(handle, JointPalette::from_identity(64));
        b.iter(|| black_box(mgr.get(black_box(handle))));
    });

    // 11. Manager free
    group.bench_function("manager_free", |b| {
        b.iter(|| {
            let mut mgr = MockJointPaletteManager::new();
            let handle = mgr.allocate();
            mgr.free(handle);
            black_box(mgr.count())
        });
    });

    // 12. Palette size constant
    group.bench_function("palette_size_constant", |b| {
        b.iter(|| black_box(JointPalette::SIZE));
    });

    group.finish();
}

fn bench_depth_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("depth_buffer");

    // 1. Depth format bytes per pixel
    group.bench_function("format_bytes_per_pixel", |b| {
        let format = DepthFormat::Depth32Float;
        b.iter(|| black_box(format.bytes_per_pixel()));
    });

    // 2. Format has stencil check
    group.bench_function("format_has_stencil", |b| {
        let format = DepthFormat::Depth24PlusStencil8;
        b.iter(|| black_box(format.has_stencil()));
    });

    // 3. Depth desc creation
    group.bench_function("desc_creation", |b| {
        b.iter(|| black_box(DepthDesc::new(black_box(1920), black_box(1080))));
    });

    // 4. Desc memory size calculation
    group.bench_function("desc_memory_size", |b| {
        let desc = DepthDesc::new(1920, 1080);
        b.iter(|| black_box(desc.memory_size()));
    });

    // 5. Desc aspect ratio
    group.bench_function("desc_aspect_ratio", |b| {
        let desc = DepthDesc::new(1920, 1080);
        b.iter(|| black_box(desc.aspect_ratio()));
    });

    // 6. Desc with MSAA
    group.bench_function("desc_with_msaa", |b| {
        let desc = DepthDesc::new(1920, 1080);
        b.iter(|| black_box(desc.clone().with_msaa(black_box(4))));
    });

    // 7. Depth buffer creation (mock)
    group.bench_function("buffer_creation", |b| {
        b.iter(|| {
            let desc = DepthDesc::new(1920, 1080);
            black_box(MockDepthBuffer::new(black_box(desc)))
        });
    });

    // 8. Buffer resize
    group.bench_function("buffer_resize", |b| {
        let mut buffer = MockDepthBuffer::new(DepthDesc::new(1920, 1080));
        b.iter(|| {
            buffer.resize(black_box(2560), black_box(1440));
        });
    });

    // 9. Buffer memory size query
    group.bench_function("buffer_memory_size", |b| {
        let buffer = MockDepthBuffer::new(DepthDesc::new(1920, 1080));
        b.iter(|| black_box(buffer.memory_size()));
    });

    // 10. Memory size at various resolutions
    for (width, height) in [(1280, 720), (1920, 1080), (2560, 1440), (3840, 2160)] {
        group.bench_with_input(
            BenchmarkId::new("memory_size_resolution", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                let desc = DepthDesc::new(w, h);
                b.iter(|| black_box(desc.memory_size()));
            },
        );
    }

    group.finish();
}

fn bench_overlay_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlay_effects");

    // 1. Overlay params default
    group.bench_function("params_default", |b| {
        b.iter(|| black_box(OverlayParams::default()));
    });

    // 2. Set fade
    group.bench_function("set_fade", |b| {
        b.iter(|| {
            let mut params = OverlayParams::default();
            params.set_fade(black_box(0.5));
            black_box(params)
        });
    });

    // 3. Set letterbox
    group.bench_function("set_letterbox", |b| {
        b.iter(|| {
            let mut params = OverlayParams::default();
            params.set_letterbox(black_box(0.125));
            black_box(params)
        });
    });

    // 4. Set vignette
    group.bench_function("set_vignette", |b| {
        b.iter(|| {
            let mut params = OverlayParams::default();
            params.set_vignette(black_box(0.3));
            black_box(params)
        });
    });

    // 5. Lerp between params
    group.bench_function("params_lerp", |b| {
        let from = OverlayParams {
            fade: 0.0,
            letterbox: 0.0,
            vignette: 0.0,
            chromatic: 0.0,
        };
        let to = OverlayParams {
            fade: 1.0,
            letterbox: 0.125,
            vignette: 0.5,
            chromatic: 0.02,
        };
        b.iter(|| black_box(from.lerp(&to, black_box(0.5))));
    });

    // 6. Is active check
    group.bench_function("is_active", |b| {
        let params = OverlayParams {
            fade: 0.3,
            letterbox: 0.0,
            vignette: 0.0,
            chromatic: 0.0,
        };
        b.iter(|| black_box(params.is_active()));
    });

    // 7. As bytes
    group.bench_function("as_bytes", |b| {
        let params = OverlayParams {
            fade: 0.5,
            letterbox: 0.125,
            vignette: 0.3,
            chromatic: 0.01,
        };
        b.iter(|| black_box(params.as_bytes()));
    });

    // 8. Size constant
    group.bench_function("size_constant", |b| {
        b.iter(|| black_box(OverlayParams::SIZE));
    });

    // 9. Animation sequence (multiple lerps)
    group.bench_function("animation_sequence_10_frames", |b| {
        let from = OverlayParams::default();
        let to = OverlayParams {
            fade: 1.0,
            letterbox: 0.125,
            vignette: 0.5,
            chromatic: 0.02,
        };
        b.iter(|| {
            let mut result = from;
            for i in 0..10 {
                let t = i as f32 / 10.0;
                result = from.lerp(&to, t);
            }
            black_box(result)
        });
    });

    group.finish();
}

fn bench_advanced_post_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("advanced_post_processing");

    // TAA benchmarks
    group.bench_function("taa_config_default", |b| {
        b.iter(|| black_box(TaaConfig::default()));
    });

    group.bench_function("taa_halton_jitter", |b| {
        let config = TaaConfig::default();
        b.iter(|| black_box(config.halton_jitter(black_box(42), black_box(1920), black_box(1080))));
    });

    // Halton sequence directly
    group.bench_function("halton_sequence", |b| {
        b.iter(|| black_box(halton_sequence(black_box(256), black_box(2))));
    });

    // Motion blur benchmarks
    group.bench_function("motion_blur_config_default", |b| {
        b.iter(|| black_box(MotionBlurConfig::default()));
    });

    // DOF benchmarks
    group.bench_function("dof_config_default", |b| {
        b.iter(|| black_box(DofConfig::default()));
    });

    group.bench_function("dof_calculate_coc", |b| {
        let config = DofConfig::default();
        b.iter(|| black_box(config.calculate_coc(black_box(15.0))));
    });

    // COC at various depths
    for depth in [5.0f32, 10.0, 15.0, 20.0, 50.0] {
        group.bench_with_input(
            BenchmarkId::new("dof_coc_at_depth", format!("{:.0}m", depth)),
            &depth,
            |b, &d| {
                let config = DofConfig::default();
                b.iter(|| black_box(config.calculate_coc(black_box(d))));
            },
        );
    }

    // Color grading benchmarks
    group.bench_function("color_grading_config_default", |b| {
        b.iter(|| black_box(ColorGradingConfig::default()));
    });

    group.bench_function("color_grading_apply", |b| {
        let config = ColorGradingConfig {
            enabled: true,
            exposure: 0.5,
            contrast: 1.2,
            saturation: 1.1,
            ..Default::default()
        };
        let color = [0.5f32, 0.3, 0.7];
        b.iter(|| black_box(config.apply(black_box(color))));
    });

    // Combined post-process config
    group.bench_function("post_process_config_default", |b| {
        b.iter(|| black_box(PostProcessConfig::default()));
    });

    // Full post-process frame simulation
    group.bench_function("full_post_process_setup", |b| {
        b.iter(|| {
            let config = PostProcessConfig::default();
            let taa_jitter = config.taa.halton_jitter(0, 1920, 1080);
            let dof_coc = config.dof.calculate_coc(10.0);
            let color = config.color_grading.apply([0.5, 0.5, 0.5]);
            black_box((taa_jitter, dof_coc, color))
        });
    });

    group.finish();
}

fn bench_combined_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_scenarios");

    // 1. Full terrain setup (8 layers + material)
    group.bench_function("terrain_full_setup", |b| {
        b.iter(|| {
            let layers = [
                TerrainLayerGpu::new(0, 1, 2, 3, [1.0, 1.0]),
                TerrainLayerGpu::new(4, 5, 6, 7, [2.0, 2.0]),
                TerrainLayerGpu::new(8, 9, 10, 11, [4.0, 4.0]),
                TerrainLayerGpu::new(12, 13, 14, 15, [8.0, 8.0]),
                TerrainLayerGpu::default(),
                TerrainLayerGpu::default(),
                TerrainLayerGpu::default(),
                TerrainLayerGpu::default(),
            ];
            let mut mat = TerrainMaterialGpu::with_layers(layers);
            mat.set_splat_map(16, 0.5);
            mat.set_triplanar(true, 45.0);
            black_box(mat)
        });
    });

    // 2. Full skeletal animation frame (64 joints)
    group.bench_function("skeletal_frame_64_joints", |b| {
        b.iter(|| {
            let mut mgr = MockJointPaletteManager::new();
            let handle = mgr.allocate();
            let palette = JointPalette::from_identity(64);
            mgr.upload(handle, palette);
            let result = mgr.get(handle).is_some();
            black_box(result)
        });
    });

    // 3. GPU memory allocation batch
    group.bench_function("memory_allocation_batch", |b| {
        let mut mgr = MockGpuMemoryBudget::with_total_budget(8 * 1024 * 1024 * 1024);
        b.iter(|| {
            // Simulate frame allocations
            let _ = mgr.try_allocate(MemoryCategory::Textures, 64 * 1024 * 1024);
            let _ = mgr.try_allocate(MemoryCategory::Geometry, 16 * 1024 * 1024);
            let _ = mgr.try_allocate(MemoryCategory::Uniforms, 1024 * 1024);
            let _ = mgr.try_allocate(MemoryCategory::RenderTargets, 32 * 1024 * 1024);
            let pressure = mgr.pressure_level();

            // Free
            mgr.free(MemoryCategory::Textures, 64 * 1024 * 1024);
            mgr.free(MemoryCategory::Geometry, 16 * 1024 * 1024);
            mgr.free(MemoryCategory::Uniforms, 1024 * 1024);
            mgr.free(MemoryCategory::RenderTargets, 32 * 1024 * 1024);

            black_box(pressure)
        });
    });

    // 4. Depth + overlay frame setup
    group.bench_function("depth_overlay_frame", |b| {
        b.iter(|| {
            let depth_desc = DepthDesc::new(1920, 1080);
            let depth = MockDepthBuffer::new(depth_desc);

            let overlay = OverlayParams {
                fade: 0.1,
                letterbox: 0.0,
                vignette: 0.2,
                chromatic: 0.005,
            };

            let _mem = depth.memory_size();
            let _active = overlay.is_active();
            let _bytes = overlay.as_bytes();

            black_box((depth, overlay))
        });
    });

    // 5. Full render frame setup
    group.bench_function("full_render_frame_setup", |b| {
        b.iter(|| {
            // Depth buffer
            let depth = MockDepthBuffer::new(DepthDesc::new(1920, 1080));

            // Terrain
            let terrain = TerrainMaterialGpu::default();

            // Skinning
            let palette = JointPalette::from_identity(64);

            // Overlay
            let overlay = OverlayParams {
                fade: 0.0,
                letterbox: 0.0,
                vignette: 0.15,
                chromatic: 0.0,
            };

            // Post-processing
            let post = PostProcessConfig::default();
            let jitter = post.taa.halton_jitter(0, 1920, 1080);

            black_box((depth, terrain, palette, overlay, jitter))
        });
    });

    // 6. Multi-skeleton batch (10 characters)
    group.bench_function("multi_skeleton_batch_10", |b| {
        let mut mgr = MockJointPaletteManager::new();
        let handles: Vec<_> = (0..10).map(|_| mgr.allocate()).collect();

        b.iter(|| {
            for &handle in &handles {
                let palette = JointPalette::from_identity(64);
                mgr.upload(handle, palette);
            }
            black_box(mgr.count())
        });
    });

    // 7. Resolution change (resize all)
    group.bench_function("resolution_change", |b| {
        let mut depth = MockDepthBuffer::new(DepthDesc::new(1920, 1080));

        b.iter(|| {
            // Resize to 4K
            depth.resize(3840, 2160);
            let mem_4k = depth.memory_size();

            // Resize back to 1080p
            depth.resize(1920, 1080);
            let mem_1080p = depth.memory_size();

            black_box((mem_4k, mem_1080p))
        });
    });

    group.finish();
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

criterion_group!(
    benches,
    bench_gpu_memory_budget,
    bench_terrain_materials,
    bench_skinning_gpu,
    bench_depth_buffer,
    bench_overlay_effects,
    bench_advanced_post_processing,
    bench_combined_scenarios,
);

criterion_main!(benches);
