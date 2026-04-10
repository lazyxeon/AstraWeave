//! Sparse Virtual Texturing (SVT) system.
//!
//! Provides tile-based streaming of texture pages from disk, enabling unique
//! terrain detail at arbitrary resolution without repeating textures.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐    ┌───────────────┐    ┌────────────────┐
//! │  Feedback     │───▶│  Page Table   │───▶│  Physical      │
//! │  (GPU→CPU)    │    │  (indirection)│    │  Cache Atlas   │
//! └──────────────┘    └───────────────┘    └────────────────┘
//!      ▲                                          ▲
//!      │ GPU compute identifies               │ CPU streams
//!      │ needed pages per frame                │ pages from disk
//! ```
//!
//! 1. **Feedback pass**: GPU compute identifies which virtual pages are visible
//! 2. **CPU readback**: Reads feedback buffer, deduplicates page requests
//! 3. **Streaming**: Loads requested pages from disk into the physical cache
//! 4. **Page table update**: Updates indirection texture so shaders resolve
//!    virtual UVs to physical atlas coordinates
//!
//! # Page ID Encoding
//!
//! Each page is identified by a packed u32:
//! - Bits [0..9]: page X (0..1023)
//! - Bits [10..19]: page Y (0..1023)
//! - Bits [20..24]: mip level (0..31)

use bytemuck::{Pod, Zeroable};

/// WGSL source for the virtual texture feedback shader.
pub const VIRTUAL_TEXTURE_WGSL: &str = include_str!("../shaders/virtual_texture.wgsl");

/// Default page size in texels (128×128).
pub const DEFAULT_PAGE_SIZE: u32 = 128;

/// Default physical cache atlas size (e.g., 4096×4096 = 1024 pages of 128×128).
pub const DEFAULT_CACHE_SIZE: u32 = 4096;

/// Maximum feedback entries per frame.
pub const MAX_FEEDBACK_ENTRIES: u32 = 65536;

/// Pack a page coordinate and mip level into a u32.
pub fn pack_page_id(page_x: u32, page_y: u32, mip: u32) -> u32 {
    (page_x & 0x3FF) | ((page_y & 0x3FF) << 10) | ((mip & 0x1F) << 20)
}

/// Unpack page X from a packed page ID.
pub fn unpack_page_x(packed: u32) -> u32 {
    packed & 0x3FF
}

/// Unpack page Y from a packed page ID.
pub fn unpack_page_y(packed: u32) -> u32 {
    (packed >> 10) & 0x3FF
}

/// Unpack mip level from a packed page ID.
pub fn unpack_mip(packed: u32) -> u32 {
    (packed >> 20) & 0x1F
}

/// A requested virtual texture page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    pub page_x: u32,
    pub page_y: u32,
    pub mip: u32,
}

impl PageRequest {
    pub fn from_packed(packed: u32) -> Self {
        Self {
            page_x: unpack_page_x(packed),
            page_y: unpack_page_y(packed),
            mip: unpack_mip(packed),
        }
    }

    pub fn to_packed(&self) -> u32 {
        pack_page_id(self.page_x, self.page_y, self.mip)
    }
}

/// GPU-side feedback pass parameters.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct FeedbackParams {
    pub vt_size: [f32; 2],
    pub inv_vt_size: [f32; 2],
    pub page_size: f32,
    pub inv_page_size: f32,
    pub max_mip_level: f32,
    pub _pad0: f32,
    pub screen_size: [f32; 2],
    pub inv_screen_size: [f32; 2],
}

const _: () = assert!(std::mem::size_of::<FeedbackParams>() == 48);

/// Entry in the page table indirection texture.
/// Maps a virtual page to a physical cache slot.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PageTableEntry {
    /// Physical X coordinate in the cache atlas (in pages).
    pub phys_x: u16,
    /// Physical Y coordinate in the cache atlas (in pages).
    pub phys_y: u16,
}

/// Physical cache slot state.
#[derive(Debug, Clone)]
struct CacheSlot {
    /// Which virtual page occupies this slot (if any).
    page: Option<PageRequest>,
    /// Frame number when this slot was last used.
    last_used_frame: u64,
}

/// Virtual texture configuration.
#[derive(Debug, Clone)]
pub struct VirtualTextureConfig {
    /// Full virtual texture size in texels (width).
    pub vt_width: u32,
    /// Full virtual texture size in texels (height).
    pub vt_height: u32,
    /// Page size in texels (must be power of 2).
    pub page_size: u32,
    /// Physical cache atlas size in texels (width = height, must be power of 2).
    pub cache_size: u32,
    /// Maximum mip levels to support.
    pub max_mip_levels: u32,
}

impl Default for VirtualTextureConfig {
    fn default() -> Self {
        Self {
            vt_width: 16384,
            vt_height: 16384,
            page_size: DEFAULT_PAGE_SIZE,
            cache_size: DEFAULT_CACHE_SIZE,
            max_mip_levels: 8,
        }
    }
}

impl VirtualTextureConfig {
    /// Number of pages along X at mip 0.
    pub fn pages_x(&self) -> u32 {
        self.vt_width / self.page_size
    }

    /// Number of pages along Y at mip 0.
    pub fn pages_y(&self) -> u32 {
        self.vt_height / self.page_size
    }

    /// Total number of cache slots in the physical atlas.
    pub fn cache_slots(&self) -> u32 {
        let slots_per_side = self.cache_size / self.page_size;
        slots_per_side * slots_per_side
    }

    /// Number of pages along X at a given mip level.
    pub fn pages_x_at_mip(&self, mip: u32) -> u32 {
        (self.pages_x() >> mip).max(1)
    }

    /// Number of pages along Y at a given mip level.
    pub fn pages_y_at_mip(&self, mip: u32) -> u32 {
        (self.pages_y() >> mip).max(1)
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), String> {
        if !self.page_size.is_power_of_two() || self.page_size == 0 {
            return Err(format!(
                "page_size must be a power of 2, got {}",
                self.page_size
            ));
        }
        if !self.cache_size.is_power_of_two() || self.cache_size == 0 {
            return Err(format!(
                "cache_size must be a power of 2, got {}",
                self.cache_size
            ));
        }
        if self.vt_width == 0 || self.vt_height == 0 {
            return Err("Virtual texture size must be > 0".into());
        }
        if self.cache_size < self.page_size {
            return Err("cache_size must be >= page_size".into());
        }
        Ok(())
    }
}

/// Manages the physical texture cache with LRU eviction.
pub struct PageCache {
    /// All cache slots.
    slots: Vec<CacheSlot>,
    /// Map from virtual page to cache slot index for O(1) lookup.
    page_to_slot: std::collections::HashMap<PageRequest, usize>,
    /// Pages per side in the cache atlas.
    slots_per_side: u32,
    /// Current frame number (for LRU tracking).
    current_frame: u64,
}

impl PageCache {
    /// Create a new page cache.
    pub fn new(config: &VirtualTextureConfig) -> Self {
        let slots_per_side = config.cache_size / config.page_size;
        let total_slots = (slots_per_side * slots_per_side) as usize;
        let slots = (0..total_slots)
            .map(|_| CacheSlot {
                page: None,
                last_used_frame: 0,
            })
            .collect();

        Self {
            slots,
            page_to_slot: std::collections::HashMap::new(),
            slots_per_side,
            current_frame: 0,
        }
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Check if a page is resident in the cache.
    pub fn is_resident(&self, page: &PageRequest) -> bool {
        self.page_to_slot.contains_key(page)
    }

    /// Touch a page (mark as recently used). Returns the physical slot coordinates.
    pub fn touch(&mut self, page: &PageRequest) -> Option<(u32, u32)> {
        if let Some(&slot_idx) = self.page_to_slot.get(page) {
            self.slots[slot_idx].last_used_frame = self.current_frame;
            let sx = (slot_idx as u32) % self.slots_per_side;
            let sy = (slot_idx as u32) / self.slots_per_side;
            Some((sx, sy))
        } else {
            None
        }
    }

    /// Allocate a cache slot for a page, evicting LRU if needed.
    /// Returns the slot index and physical (x, y) in pages.
    pub fn allocate(&mut self, page: PageRequest) -> (usize, u32, u32) {
        // Check if already resident
        if let Some(&slot_idx) = self.page_to_slot.get(&page) {
            self.slots[slot_idx].last_used_frame = self.current_frame;
            let sx = (slot_idx as u32) % self.slots_per_side;
            let sy = (slot_idx as u32) / self.slots_per_side;
            return (slot_idx, sx, sy);
        }

        // Find an empty slot or LRU slot
        let slot_idx = self.find_slot();

        // Evict old page if necessary
        if let Some(old_page) = self.slots[slot_idx].page.take() {
            self.page_to_slot.remove(&old_page);
        }

        // Install new page
        self.slots[slot_idx].page = Some(page);
        self.slots[slot_idx].last_used_frame = self.current_frame;
        self.page_to_slot.insert(page, slot_idx);

        let sx = (slot_idx as u32) % self.slots_per_side;
        let sy = (slot_idx as u32) / self.slots_per_side;
        (slot_idx, sx, sy)
    }

    fn find_slot(&self) -> usize {
        // First, try to find an empty slot
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.page.is_none() {
                return i;
            }
        }
        // Otherwise, evict the LRU (oldest last_used_frame)
        self.slots
            .iter()
            .enumerate()
            .min_by_key(|(_, s)| s.last_used_frame)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Number of occupied slots.
    pub fn occupancy(&self) -> usize {
        self.page_to_slot.len()
    }

    /// Total number of slots.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

/// Process raw feedback buffer data into deduplicated page requests.
pub fn process_feedback(
    feedback_data: &[u32],
    count: u32,
    config: &VirtualTextureConfig,
) -> Vec<PageRequest> {
    let mut seen = std::collections::HashSet::new();
    let mut requests = Vec::new();
    let actual_count = (count as usize).min(feedback_data.len());

    for &packed in feedback_data.iter().take(actual_count) {
        if packed == 0 {
            continue;
        }
        let page = PageRequest::from_packed(packed);
        // Validate page coordinates are in range
        if page.mip < config.max_mip_levels
            && page.page_x < config.pages_x_at_mip(page.mip)
            && page.page_y < config.pages_y_at_mip(page.mip)
            && seen.insert(packed)
        {
            requests.push(page);
        }
    }

    // Sort by mip level (finest first) for priority streaming
    requests.sort_by_key(|r| r.mip);
    requests
}

/// GPU resources for the virtual texture feedback pass.
pub struct VirtualTextureFeedback {
    /// Compute pipeline for the feedback pass.
    pipeline: wgpu::ComputePipeline,
    /// Bind group layout.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Feedback parameter uniform buffer.
    params_buffer: wgpu::Buffer,
    /// Feedback output storage buffer (page IDs).
    feedback_buffer: wgpu::Buffer,
    /// Atomic counter buffer (single u32).
    counter_buffer: wgpu::Buffer,
    /// Readback buffer (for CPU access).
    readback_buffer: wgpu::Buffer,
    /// Counter readback buffer.
    counter_readback_buffer: wgpu::Buffer,
    /// Configuration.
    config: VirtualTextureConfig,
}

impl VirtualTextureFeedback {
    /// Create the virtual texture feedback system.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &VirtualTextureConfig,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let params = FeedbackParams {
            vt_size: [config.vt_width as f32, config.vt_height as f32],
            inv_vt_size: [1.0 / config.vt_width as f32, 1.0 / config.vt_height as f32],
            page_size: config.page_size as f32,
            inv_page_size: 1.0 / config.page_size as f32,
            max_mip_level: (config.max_mip_levels - 1) as f32,
            _pad0: 0.0,
            screen_size: [screen_width as f32, screen_height as f32],
            inv_screen_size: [1.0 / screen_width as f32, 1.0 / screen_height as f32],
        };

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vt_feedback_params"),
            size: std::mem::size_of::<FeedbackParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let feedback_buf_size = (MAX_FEEDBACK_ENTRIES as u64) * 4;
        let feedback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vt_feedback"),
            size: feedback_buf_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let counter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vt_feedback_counter"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Initialize counter to 0
        queue.write_buffer(&counter_buffer, 0, &[0u8; 4]);

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vt_feedback_readback"),
            size: feedback_buf_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let counter_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vt_counter_readback"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vt_feedback_bgl"),
            entries: &[
                // binding 0: params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: feedback storage
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: counter storage
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: depth texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vt_feedback_shader"),
            source: wgpu::ShaderSource::Wgsl(VIRTUAL_TEXTURE_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vt_feedback_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("vt_feedback_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("feedback_pass"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            feedback_buffer,
            counter_buffer,
            readback_buffer,
            counter_readback_buffer,
            config: config.clone(),
        }
    }

    /// Get the bind group layout for creating bind groups with custom depth textures.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Create a bind group for a specific depth texture view.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vt_feedback_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.feedback_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
            ],
        })
    }

    /// Record the feedback compute pass into an encoder.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        screen_width: u32,
        screen_height: u32,
    ) {
        // Reset counter to 0
        encoder.clear_buffer(&self.counter_buffer, 0, None);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("vt_feedback_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        let groups_x = screen_width.div_ceil(8);
        let groups_y = screen_height.div_ceil(8);
        pass.dispatch_workgroups(groups_x, groups_y, 1);
        drop(pass);

        // Copy results to readback buffers
        encoder.copy_buffer_to_buffer(
            &self.feedback_buffer,
            0,
            &self.readback_buffer,
            0,
            self.feedback_buffer.size(),
        );
        encoder.copy_buffer_to_buffer(&self.counter_buffer, 0, &self.counter_readback_buffer, 0, 4);
    }

    /// Configuration reference.
    pub fn config(&self) -> &VirtualTextureConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_params_size() {
        assert_eq!(std::mem::size_of::<FeedbackParams>(), 48);
    }

    #[test]
    fn page_id_roundtrip() {
        for mip in 0..16 {
            for page_x in [0, 1, 127, 511, 1023] {
                for page_y in [0, 1, 255, 1023] {
                    let packed = pack_page_id(page_x, page_y, mip);
                    assert_eq!(unpack_page_x(packed), page_x);
                    assert_eq!(unpack_page_y(packed), page_y);
                    assert_eq!(unpack_mip(packed), mip);
                }
            }
        }
    }

    #[test]
    fn page_request_roundtrip() {
        let req = PageRequest {
            page_x: 42,
            page_y: 99,
            mip: 3,
        };
        let packed = req.to_packed();
        let req2 = PageRequest::from_packed(packed);
        assert_eq!(req, req2);
    }

    #[test]
    fn default_config_valid() {
        let config = VirtualTextureConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.pages_x(), 128); // 16384/128
        assert_eq!(config.pages_y(), 128);
        assert_eq!(config.cache_slots(), 1024); // (4096/128)^2 = 32*32
    }

    #[test]
    fn config_pages_at_mip() {
        let config = VirtualTextureConfig::default();
        assert_eq!(config.pages_x_at_mip(0), 128);
        assert_eq!(config.pages_x_at_mip(1), 64);
        assert_eq!(config.pages_x_at_mip(2), 32);
        assert_eq!(config.pages_x_at_mip(7), 1);
    }

    #[test]
    fn config_validation_rejects_invalid() {
        let mut config = VirtualTextureConfig::default();
        config.page_size = 100; // not power of 2
        assert!(config.validate().is_err());

        config.page_size = 128;
        config.cache_size = 64; // < page_size
        assert!(config.validate().is_err());
    }

    #[test]
    fn page_cache_basic() {
        let config = VirtualTextureConfig {
            vt_width: 512,
            vt_height: 512,
            page_size: 128,
            cache_size: 256, // 2×2 = 4 slots
            max_mip_levels: 4,
        };
        let mut cache = PageCache::new(&config);
        assert_eq!(cache.capacity(), 4);
        assert_eq!(cache.occupancy(), 0);

        let page = PageRequest {
            page_x: 0,
            page_y: 0,
            mip: 0,
        };
        let (_, sx, sy) = cache.allocate(page);
        assert_eq!(cache.occupancy(), 1);
        assert!(cache.is_resident(&page));
        assert!(sx < 2 && sy < 2);
    }

    #[test]
    fn page_cache_lru_eviction() {
        let config = VirtualTextureConfig {
            vt_width: 512,
            vt_height: 512,
            page_size: 128,
            cache_size: 256, // 4 slots
            max_mip_levels: 4,
        };
        let mut cache = PageCache::new(&config);

        // Fill all 4 slots
        for i in 0..4 {
            cache.begin_frame();
            cache.allocate(PageRequest {
                page_x: i,
                page_y: 0,
                mip: 0,
            });
        }
        assert_eq!(cache.occupancy(), 4);

        // Allocate a 5th page — should evict the LRU (page_x=0, frame 1)
        cache.begin_frame();
        cache.allocate(PageRequest {
            page_x: 4,
            page_y: 0,
            mip: 0,
        });
        assert_eq!(cache.occupancy(), 4); // still 4 (evicted one)
        assert!(!cache.is_resident(&PageRequest {
            page_x: 0,
            page_y: 0,
            mip: 0
        }));
        assert!(cache.is_resident(&PageRequest {
            page_x: 4,
            page_y: 0,
            mip: 0
        }));
    }

    #[test]
    fn page_cache_touch_updates_lru() {
        let config = VirtualTextureConfig {
            vt_width: 512,
            vt_height: 512,
            page_size: 128,
            cache_size: 256,
            max_mip_levels: 4,
        };
        let mut cache = PageCache::new(&config);

        // Fill all 4 slots
        for i in 0..4 {
            cache.begin_frame();
            cache.allocate(PageRequest {
                page_x: i,
                page_y: 0,
                mip: 0,
            });
        }

        // Touch page_x=0 so it's no longer LRU
        cache.begin_frame();
        let page0 = PageRequest {
            page_x: 0,
            page_y: 0,
            mip: 0,
        };
        cache.touch(&page0);

        // Eviction should now pick page_x=1 (oldest untouched)
        cache.begin_frame();
        cache.allocate(PageRequest {
            page_x: 5,
            page_y: 0,
            mip: 0,
        });
        assert!(cache.is_resident(&page0)); // page 0 was touched, so it survives
        assert!(!cache.is_resident(&PageRequest {
            page_x: 1,
            page_y: 0,
            mip: 0
        }));
    }

    #[test]
    fn process_feedback_deduplicates() {
        let config = VirtualTextureConfig::default();
        let p1 = pack_page_id(10, 20, 0);
        let p2 = pack_page_id(11, 20, 0);
        let feedback = vec![p1, p2, p1, p1, p2]; // duplicates
        let requests = process_feedback(&feedback, 5, &config);
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn process_feedback_sorts_by_mip() {
        let config = VirtualTextureConfig::default();
        let p_mip3 = pack_page_id(0, 0, 3);
        let p_mip0 = pack_page_id(1, 1, 0);
        let p_mip1 = pack_page_id(2, 2, 1);
        let feedback = vec![p_mip3, p_mip0, p_mip1];
        let requests = process_feedback(&feedback, 3, &config);
        assert_eq!(requests[0].mip, 0);
        assert_eq!(requests[1].mip, 1);
        assert_eq!(requests[2].mip, 3);
    }

    #[test]
    fn process_feedback_filters_out_of_range() {
        let config = VirtualTextureConfig {
            vt_width: 256,
            vt_height: 256,
            page_size: 128,
            cache_size: 256,
            max_mip_levels: 2,
        };
        // pages_x at mip 0 = 2, so page_x=5 is out of range
        let p_valid = pack_page_id(1, 1, 0);
        let p_invalid = pack_page_id(5, 0, 0);
        let feedback = vec![p_valid, p_invalid];
        let requests = process_feedback(&feedback, 2, &config);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].page_x, 1);
    }

    #[test]
    fn parse_virtual_texture_wgsl() {
        let module = naga::front::wgsl::parse_str(VIRTUAL_TEXTURE_WGSL);
        assert!(
            module.is_ok(),
            "Failed to parse virtual_texture.wgsl: {:?}",
            module.err()
        );
    }

    #[test]
    fn vt_feedback_pipeline_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                ..Default::default()
            })) {
                Ok(a) => a,
                Err(_) => return,
            };
        let (device, queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
                Ok(dq) => dq,
                Err(_) => return,
            };

        let config = VirtualTextureConfig::default();
        let feedback = VirtualTextureFeedback::new(&device, &queue, &config, 1920, 1080);
        let _bgl = feedback.bind_group_layout();
        assert_eq!(feedback.config().page_size, 128);
    }
}
