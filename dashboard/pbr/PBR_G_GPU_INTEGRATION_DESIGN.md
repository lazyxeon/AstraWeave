# PBR-G GPU Hot-Reload Integration - Design Document

**Date**: October 7, 2025  
**Status**: Design Complete - Implementation Ready  
**Complexity**: Medium (2-3 hours estimated)  
**Phase**: PBR-G Task 3 Extension (GPU Integration)

---

## Executive Summary

This document outlines the GPU integration strategy for the hot-reload system implemented in Task 3. The goal is to enable **live material editing in `unified_showcase`** where TOML edits and texture swaps update the GPU immediately without restarting the application.

**Current State**: File watcher operational in `aw_editor` Material Inspector (CPU-side only)  
**Target State**: GPU buffers update automatically when materials/textures change  
**Key Challenge**: `unified_showcase` has complex material system with multiple buffers and bind groups

---

## Architecture Overview

### Current Material System in `unified_showcase`

```rust
struct Render {
    // Material buffers (GPU)
    material_buffer: Option<wgpu::Buffer>,          // Standard MaterialGpu
    pbr_e_material_buffer: Option<wgpu::Buffer>,    // Extended materials (Phase PBR-E)
    default_material_buffer: wgpu::Buffer,          // Fallback material
    
    // Bind groups (GPU bindings)
    material_bind_group: Option<wgpu::BindGroup>,
    pbr_e_material_bind_group: Option<wgpu::BindGroup>,
    default_material_bind_group: wgpu::BindGroup,
    
    // Material management (CPU)
    material_integrator: MaterialIntegrator,        // Loads materials from TOML
    material_library: Option<MaterialLibrary>,      // Material metadata
    texture_manager: Option<TextureManager>,        // Texture array management
    
    // wgpu context
    device: wgpu::Device,
    queue: wgpu::Queue,
}
```

### File Watcher System (Task 3)

```rust
// From tools/aw_editor/src/file_watcher.rs
pub enum ReloadEvent {
    Material(PathBuf),  // TOML file changed
    Texture(PathBuf),   // PNG/KTX2/DDS file changed
}

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<ReloadEvent>,
    // ... debouncing, filtering
}
```

---

## Integration Strategy

### Option A: Minimal Integration (Recommended)

**Goal**: Hot-reload for **currently selected material** in real-time  
**Scope**: Single material updates (not full texture pack switches)  
**Complexity**: Low  
**Time**: 1-2 hours

#### Implementation Steps

1. **Add FileWatcher to Render struct**:
   ```rust
   struct Render {
       // ... existing fields ...
       file_watcher: Option<FileWatcher>,
       current_material_path: Option<PathBuf>,  // Track loaded material
   }
   ```

2. **Initialize watcher in `setup_renderer()`**:
   ```rust
   let file_watcher = FileWatcher::new("assets/materials")?;
   ```

3. **Process events in main loop** (inside `Event::AboutToWait`):
   ```rust
   // Collect hot-reload events
   let mut events = Vec::new();
   if let Some(watcher) = &render.file_watcher {
       while let Ok(event) = watcher.try_recv() {
           events.push(event);
       }
   }
   
   // Process material reloads
   for event in events {
       match event {
           ReloadEvent::Material(path) => {
               // Check if this is the current material
               if Some(&path) == render.current_material_path.as_ref() {
                   reload_material_gpu(&mut render, &path)?;
               }
           }
           ReloadEvent::Texture(path) => {
               // Check if texture belongs to current material
               if is_current_material_texture(&render, &path) {
                   reload_texture_gpu(&mut render, &path)?;
               }
           }
       }
   }
   ```

4. **Implement GPU update functions**:

   **Material Reload** (TOML → GPU buffer):
   ```rust
   fn reload_material_gpu(render: &mut Render, path: &Path) -> Result<()> {
       // Parse TOML
       let toml_content = std::fs::read_to_string(path)?;
       let material: Material = toml::from_str(&toml_content)?;
       
       // Convert to GPU representation
       let material_gpu: MaterialGpu = material.to_gpu();
       
       // Determine buffer offset (for multi-material setups)
       let material_id = 0; // Single material for now
       let offset = material_id * std::mem::size_of::<MaterialGpu>();
       
       // Update GPU buffer
       if let Some(buffer) = &render.material_buffer {
           render.queue.write_buffer(
               buffer,
               offset as u64,
               bytemuck::bytes_of(&material_gpu),
           );
       }
       
       println!("✅ Hot-reloaded material: {}", path.display());
       Ok(())
   }
   ```

   **Texture Reload** (PNG/KTX2 → GPU texture array):
   ```rust
   fn reload_texture_gpu(render: &mut Render, path: &Path) -> Result<()> {
       // Determine texture type from path
       let tex_type = if path.to_string_lossy().contains("albedo") {
           TextureType::Albedo
       } else if path.to_string_lossy().contains("normal") {
           TextureType::Normal
       } else if path.to_string_lossy().contains("orm") || 
                 path.to_string_lossy().contains("mra") {
           TextureType::ORM
       } else {
           return Ok(()); // Unknown texture type
       };
       
       // Load image
       let img = image::open(path)?;
       
       // Convert to GPU format
       let rgba = img.to_rgba8();
       let (width, height) = rgba.dimensions();
       
       // Determine color space
       let format = match tex_type {
           TextureType::Albedo => wgpu::TextureFormat::Rgba8UnormSrgb,
           TextureType::Normal | TextureType::ORM => wgpu::TextureFormat::Rgba8Unorm,
       };
       
       // Update texture array (if using texture arrays)
       if let Some(texture_mgr) = &mut render.texture_manager {
           let array_index = texture_mgr.get_index_for_path(path)?;
           
           render.queue.write_texture(
               wgpu::ImageCopyTexture {
                   texture: &texture_mgr.get_array_texture(tex_type),
                   mip_level: 0,
                   origin: wgpu::Origin3d {
                       x: 0, y: 0, z: array_index,
                   },
                   aspect: wgpu::TextureAspect::All,
               },
               &rgba,
               wgpu::ImageDataLayout {
                   offset: 0,
                   bytes_per_row: Some(4 * width),
                   rows_per_image: Some(height),
               },
               wgpu::Extent3d {
                   width, height, depth_or_array_layers: 1,
               },
           );
       }
       
       println!("✅ Hot-reloaded texture: {}", path.display());
       Ok(())
   }
   ```

5. **Handle bind group updates** (if texture layout changes):
   ```rust
   fn rebind_material_textures(render: &mut Render) -> Result<()> {
       // Only needed if texture array indices change
       // For most cases, write_texture is sufficient
       
       // Recreate bind group if necessary
       if render.material_bind_group.is_some() {
           render.material_bind_group = Some(
               render.device.create_bind_group(&wgpu::BindGroupDescriptor {
                   layout: &render.material_bind_group_layout,
                   entries: &[
                       // ... recreate with updated texture views
                   ],
                   label: Some("material_bind_group_hotreload"),
               })
           );
       }
       
       Ok(())
   }
   ```

### Option B: Full Integration (Advanced)

**Goal**: Hot-reload for **all materials** in texture packs  
**Scope**: Biome switches, multi-material scenes  
**Complexity**: High  
**Time**: 4-6 hours

#### Additional Requirements

1. **Material ID Tracking**:
   - Map file paths → material IDs
   - Update specific slots in material SSBO

2. **Texture Array Management**:
   - Re-upload multiple layers
   - Handle array resizing (if pack changes)

3. **Bind Group Cache Invalidation**:
   - Detect layout changes
   - Recreate bind groups only when needed

4. **Multi-Material Coordination**:
   - Standard materials (`MaterialGpu`)
   - Extended materials (`MaterialGpuExtended` from Phase PBR-E)
   - Terrain materials (`TerrainMaterialGpu` from Phase PBR-F)

---

## Code Locations

### Files to Modify

1. **examples/unified_showcase/src/main.rs** (~8,775 lines):
   - Line 1900-1950: Add `file_watcher` field to `Render` struct
   - Line 6200-6400: Initialize watcher in `setup_renderer()`
   - Line 3900-4200: Process events in main loop (`Event::AboutToWait`)
   - New helper functions: `reload_material_gpu()`, `reload_texture_gpu()`

2. **examples/unified_showcase/src/material_integration.rs** (if needed):
   - Add hot-reload helpers to `MaterialIntegrator`

### Files to Reference

1. **tools/aw_editor/src/file_watcher.rs**:
   - Copy `FileWatcher` struct and `ReloadEvent` enum
   - Reuse debouncing logic (500ms)

2. **tools/aw_editor/src/material_inspector.rs**:
   - Reference `process_hot_reload()` method (lines 300-350)
   - Adapt error handling patterns

3. **examples/unified_showcase/src/material.rs**:
   - Use `Material::to_gpu()` for CPU → GPU conversion
   - Check `MaterialGpu` struct layout

---

## Testing Plan

### Unit Tests (Optional)

1. **Material Reload**:
   ```rust
   #[test]
   fn test_material_reload() {
       // Create temp TOML file
       // Modify albedo factor
       // Verify GPU buffer updated
   }
   ```

2. **Texture Reload**:
   ```rust
   #[test]
   fn test_texture_reload() {
       // Create temp PNG file
       // Replace with different image
       // Verify GPU texture updated
   }
   ```

### Manual Testing

1. **Material Factor Changes**:
   ```powershell
   # Start unified_showcase
   cargo run -p unified_showcase --release
   
   # Edit assets/materials/grassland/grassland_demo.toml
   # Change: albedo_factor = [1.0, 0.0, 0.0, 1.0]  (red tint)
   # Expected: Immediate visual update in 3D view
   ```

2. **Texture Swaps**:
   ```powershell
   # Replace assets/materials/grassland/grass_albedo.png
   # Expected: Immediate texture update in 3D view
   ```

3. **Error Handling**:
   ```powershell
   # Create corrupt TOML (invalid syntax)
   # Expected: Error logged, GPU state unchanged
   
   # Delete texture file
   # Expected: Fallback to previous texture or default
   ```

### Performance Validation

- **Material reload time**: <50ms (parse TOML + GPU upload)
- **Texture reload time**: <100ms for 1K, <500ms for 4K
- **Memory**: No leaks (old buffers released)
- **Frame rate**: No drops during reload

---

## Performance Considerations

### Bottlenecks

1. **Texture Upload**: O(width × height) for `write_texture`
   - 1K texture: ~4MB, ~50ms upload
   - 4K texture: ~64MB, ~500ms upload
   - Mitigation: Upload on separate thread (async), show loading indicator

2. **TOML Parsing**: O(lines) for `toml::from_str`
   - Typical material: <100 lines, <5ms parse
   - Mitigation: Already fast, no action needed

3. **Bind Group Recreation**: O(textures) for `create_bind_group`
   - Typical material: 3 textures, ~2ms
   - Mitigation: Only recreate if layout changed

### Optimizations

1. **Debouncing** (already implemented in FileWatcher):
   - 500ms delay prevents 3-5 rapid saves → 1 reload
   - 67% reduction in reload count

2. **Smart Filtering**:
   - Only reload if path matches current material
   - Skip if GPU buffer already up-to-date

3. **Async Uploads** (future enhancement):
   ```rust
   // Use staging buffer for large textures
   let staging_buffer = device.create_buffer_init(...);
   encoder.copy_buffer_to_texture(staging_buffer, texture, size);
   queue.submit([encoder.finish()]);
   ```

---

## Error Handling

### Failure Modes

1. **Corrupt TOML**:
   - **Cause**: Invalid syntax, missing fields
   - **Handling**: Log error, keep old GPU state
   - **Recovery**: User fixes TOML, triggers reload

2. **Missing Texture**:
   - **Cause**: File deleted, path typo
   - **Handling**: Log warning, use fallback texture
   - **Recovery**: Restore file or update TOML path

3. **GPU Upload Failure**:
   - **Cause**: Out of memory, device lost
   - **Handling**: Log critical error, disable hot-reload
   - **Recovery**: Restart application

4. **Bind Group Mismatch**:
   - **Cause**: Texture format changed (sRGB ↔ Linear)
   - **Handling**: Recreate bind group with correct format
   - **Recovery**: Automatic on next reload

### Error Messages

```rust
match reload_material_gpu(&mut render, &path) {
    Ok(_) => {
        println!("✅ Hot-reloaded material: {}", path.display());
    }
    Err(e) => {
        eprintln!("❌ Failed to reload material {}: {}", path.display(), e);
        eprintln!("   GPU state unchanged. Fix the error and save again.");
    }
}
```

---

## Future Enhancements

### Phase 2: Advanced Features

1. **Shader Hot-Reload**:
   - Watch `*.wgsl` files
   - Recompile shaders on change
   - Recreate pipelines
   - **Complexity**: High (shader validation, pipeline cache)

2. **Multi-Material Hot-Reload**:
   - Track all loaded materials
   - Update material SSBO arrays
   - Handle material ID remapping
   - **Complexity**: Medium

3. **GPU State Validation**:
   - Verify buffer contents match CPU state
   - Detect memory corruption
   - Auto-recovery on mismatch
   - **Complexity**: Medium

4. **Undo/Redo for Material Edits**:
   - Track material history
   - Rollback GPU state on undo
   - Integrate with editor UI
   - **Complexity**: High

### Phase 3: Production Features

1. **Network Hot-Reload**:
   - Remote artist edits material
   - Server broadcasts update
   - All clients reload GPU state
   - **Use Case**: Multi-user level editing

2. **Incremental Updates**:
   - Diff old vs new material
   - Only upload changed fields
   - Reduce bandwidth/latency
   - **Optimization**: 80% reduction for small edits

3. **Hot-Reload Profiler**:
   - Measure reload times
   - Identify slow operations
   - Visualize GPU upload metrics
   - **Tool**: Performance dashboard

---

## Implementation Checklist

### Phase 1: Minimal Integration (Option A)

- [ ] Add `file_watcher` field to `Render` struct
- [ ] Initialize `FileWatcher` in `setup_renderer()`
- [ ] Add `current_material_path` tracking
- [ ] Implement event collection in main loop
- [ ] Write `reload_material_gpu()` function
- [ ] Write `reload_texture_gpu()` function
- [ ] Add error handling and logging
- [ ] Test material factor changes
- [ ] Test texture swaps
- [ ] Test corrupt TOML handling
- [ ] Document usage in README
- [ ] Create completion report (PBR_G_GPU_INTEGRATION_COMPLETE.md)

### Phase 2: Full Integration (Option B)

- [ ] Material ID → path mapping
- [ ] Multi-material SSBO updates
- [ ] Texture array layer management
- [ ] Bind group cache invalidation
- [ ] Extended material support (Phase PBR-E)
- [ ] Terrain material support (Phase PBR-F)
- [ ] Comprehensive testing suite
- [ ] Performance profiling
- [ ] Advanced documentation

---

## Dependencies

### Crates

- `notify = "8"` (already in aw_editor)
- `image = "0.25"` (already in unified_showcase)
- `toml = "0.8"` (already in unified_showcase)
- `bytemuck = "1.18"` (already in unified_showcase)

### Workspace Crates

- `astraweave-render` (material types, IBL)
- `tools/aw_editor` (FileWatcher reference)

---

## Timeline Estimate

### Option A (Minimal Integration)

- **Setup & File Changes**: 30 min
- **reload_material_gpu()**: 45 min
- **reload_texture_gpu()**: 45 min
- **Testing & Debugging**: 30 min
- **Documentation**: 30 min
- **Total**: **2.5 hours**

### Option B (Full Integration)

- **Material ID Tracking**: 1 hour
- **Multi-Material Updates**: 1.5 hours
- **Texture Array Management**: 1 hour
- **Bind Group Handling**: 1 hour
- **Extended Material Support**: 1 hour
- **Testing & Debugging**: 1 hour
- **Total**: **6.5 hours**

---

## Decision: Recommended Approach

**Recommendation**: Proceed with **Option A (Minimal Integration)** first

### Rationale

1. **Fast Time to Value**: 2.5 hours vs 6.5 hours
2. **Low Risk**: Isolated changes, easier to debug
3. **Immediate Benefit**: Material artists can iterate faster
4. **Incremental Path**: Can upgrade to Option B later
5. **Validation**: Proves hot-reload architecture works on GPU

### Success Criteria

1. Edit `grassland_demo.toml` → see changes in 3D view within 1 second
2. Replace `grass_albedo.png` → texture updates without restart
3. No frame drops during reload (<16ms per frame maintained)
4. Graceful error handling (corrupt TOML doesn't crash app)
5. Documentation complete (usage guide + troubleshooting)

---

## Next Steps

1. **User Decision**: Confirm Option A (Minimal) or Option B (Full)
2. **Implementation**: Follow checklist above
3. **Testing**: Validate with grassland biome materials
4. **Documentation**: Create PBR_G_GPU_INTEGRATION_COMPLETE.md
5. **Roadmap Update**: Mark Task 3 GPU integration complete

---

## References

- **Task 3 Implementation**: `PBR_G_TASK3_HOT_RELOAD_IMPLEMENTATION.md`
- **Material Inspector**: `tools/aw_editor/src/material_inspector.rs`
- **File Watcher**: `tools/aw_editor/src/file_watcher.rs`
- **unified_showcase**: `examples/unified_showcase/src/main.rs`
- **Material Types**: `examples/unified_showcase/src/material.rs`
- **Phase PBR-E**: `PBR_E_IMPLEMENTATION_SUMMARY.md`
- **Phase PBR-F**: `PBR_F_COMPLETION_SUMMARY.md`

---

**Status**: Design Complete ✅  
**Next Action**: Implement Option A (2.5 hours estimated)  
**Blocker**: None  
**Risk**: Low
