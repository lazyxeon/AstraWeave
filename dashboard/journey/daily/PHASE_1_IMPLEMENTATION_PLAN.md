# Phase 1: 3D Viewport Foundation - Implementation Plan

**Date**: November 4, 2025  
**Duration**: 2-3 weeks (10-15 days)  
**Goal**: Professional 3D viewport with camera controls, entity rendering, selection, and visual gizmos  
**Code Quality**: Mission-critical production standards

---

## Architecture Overview

### Module Structure

```
tools/aw_editor/src/
├── viewport/                    # NEW - 3D viewport system
│   ├── mod.rs                   # Public API
│   ├── widget.rs                # Custom egui widget (main integration)
│   ├── renderer.rs              # wgpu rendering coordinator
│   ├── camera.rs                # Orbit camera controller
│   ├── entity_renderer.rs       # Entity mesh rendering
│   ├── gizmo_renderer.rs        # Visual gizmo overlays
│   ├── picking.rs               # Ray-casting selection
│   ├── grid_renderer.rs         # Grid/axis rendering
│   └── shaders/                 # WGSL shaders
│       ├── entity.wgsl          # Entity rendering
│       ├── gizmo.wgsl           # Gizmo handles
│       ├── grid.wgsl            # Grid overlay
│       └── outline.wgsl         # Selection outline
├── main.rs                      # UPDATE: Add viewport to layout
└── ... (existing panels)
```

### Technology Stack

1. **egui_wgpu**: Integrate wgpu rendering into egui (already in dependencies via eframe)
2. **wgpu 23.0**: Direct rendering backend (already used in astraweave-render)
3. **glam**: Math library (already in use, Vec3, Mat4, Quat)
4. **Existing Infrastructure**:
   - `astraweave-render`: Material system, mesh registry, GPU pipelines
   - `gizmo/`: Transform gizmo logic (translate, rotate, scale)
   - `World`: Entity storage, simulation state

### Design Principles

**Mission-Critical Standards**:
1. ✅ **Zero Panics**: All unwraps replaced with proper error handling (Result/Option)
2. ✅ **Memory Safety**: No raw pointers, RAII for GPU resources
3. ✅ **Performance**: 60 FPS minimum (16.67ms frame budget)
4. ✅ **Testability**: Unit tests for math, integration tests for rendering
5. ✅ **Documentation**: Rustdoc on all public APIs
6. ✅ **Error Propagation**: anyhow::Result for all fallible operations
7. ✅ **Resource Cleanup**: Drop impls for GPU handles, no leaks

---

## Phase 1.1: egui_wgpu Integration (Days 1-3)

### Objective

Get wgpu rendering working inside an egui panel. Render a simple test scene (grid + cube).

### Implementation

#### Step 1: Add Dependencies (Day 1, 30 min)

```toml
# tools/aw_editor/Cargo.toml
[dependencies]
egui = "0.32"
eframe = { version = "0.32", features = ["wgpu"] }
wgpu = "23.0"
pollster = "0.4"  # For async/await in sync context
bytemuck = "1.18"
```

#### Step 2: Create ViewportWidget (Day 1, 4 hours)

**File**: `tools/aw_editor/src/viewport/widget.rs`

**Key Responsibilities**:
- Allocate egui space for 3D viewport
- Handle mouse/keyboard input
- Coordinate rendering pipeline
- Display rendered texture

**Code Structure** (500 lines):
```rust
/// Custom egui widget for 3D viewport
///
/// Integrates wgpu rendering into egui panels. Handles input, rendering,
/// and resource management. Designed for 60 FPS performance.
pub struct ViewportWidget {
    /// Rendering state (GPU resources)
    renderer: ViewportRenderer,
    
    /// Camera controller
    camera: OrbitCamera,
    
    /// Currently selected entity
    selected_entity: Option<EntityId>,
    
    /// Gizmo state (translate/rotate/scale mode)
    gizmo_state: GizmoState,
    
    /// Cached render texture (reused each frame)
    render_texture: Option<wgpu::Texture>,
    
    /// Viewport size (for resize detection)
    last_size: (u32, u32),
}

impl ViewportWidget {
    /// Create new viewport widget
    ///
    /// # Errors
    /// Returns error if wgpu device creation fails
    pub fn new(cc: &eframe::CreationContext) -> anyhow::Result<Self> {
        let renderer = ViewportRenderer::new(&cc.wgpu_render_state)?;
        Ok(Self {
            renderer,
            camera: OrbitCamera::default(),
            selected_entity: None,
            gizmo_state: GizmoState::default(),
            render_texture: None,
            last_size: (0, 0),
        })
    }
    
    /// Render viewport UI
    pub fn ui(&mut self, ui: &mut egui::Ui, world: &World) -> anyhow::Result<()> {
        // Allocate space (70% of available width)
        let available = ui.available_size();
        let viewport_size = egui::vec2(available.x * 0.7, available.y);
        let (rect, response) = ui.allocate_exact_size(
            viewport_size,
            egui::Sense::click_and_drag(),
        );
        
        // Handle input (mouse/keyboard)
        self.handle_input(&response, ui.ctx())?;
        
        // Resize texture if needed
        let size = (rect.width() as u32, rect.height() as u32);
        if size != self.last_size {
            self.resize_texture(size)?;
            self.last_size = size;
        }
        
        // Render to texture
        let texture_id = self.render_frame(world)?;
        
        // Display texture in egui
        ui.painter().image(
            texture_id,
            rect,
            egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(1.0, 1.0),
            ),
            egui::Color32::WHITE,
        );
        
        Ok(())
    }
    
    /// Handle mouse/keyboard input
    fn handle_input(&mut self, response: &egui::Response, ctx: &egui::Context) -> anyhow::Result<()> {
        // Orbit camera (left mouse drag)
        if response.dragged_by(egui::PointerButton::Primary) && !self.gizmo_state.is_dragging() {
            let delta = response.drag_delta();
            self.camera.orbit(delta.x, delta.y);
        }
        
        // Pan camera (middle mouse drag)
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera.pan(delta.x, delta.y);
        }
        
        // Zoom camera (scroll wheel)
        let scroll = ctx.input(|i| i.scroll_delta.y);
        if scroll.abs() > 0.0 {
            self.camera.zoom(scroll);
        }
        
        // Select entity (left click)
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                // Convert screen pos to viewport coords
                let viewport_pos = pos - response.rect.min;
                let ray = self.camera.ray_from_screen(viewport_pos, response.rect.size());
                
                // Ray-cast to find entity (TODO: implement in Phase 1.4)
                // self.selected_entity = pick_entity(ray, world)?;
            }
        }
        
        // Gizmo hotkeys (G/R/S)
        ctx.input(|i| {
            if i.key_pressed(egui::Key::G) {
                self.gizmo_state.set_mode(GizmoMode::Translate);
            }
            if i.key_pressed(egui::Key::R) {
                self.gizmo_state.set_mode(GizmoMode::Rotate);
            }
            if i.key_pressed(egui::Key::S) {
                self.gizmo_state.set_mode(GizmoMode::Scale);
            }
        });
        
        Ok(())
    }
    
    /// Render frame to texture
    fn render_frame(&mut self, world: &World) -> anyhow::Result<egui::TextureId> {
        let texture = self.render_texture.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Render texture not initialized"))?;
        
        // Render to texture
        self.renderer.render(
            texture,
            &self.camera,
            world,
            self.selected_entity,
            &self.gizmo_state,
        )?;
        
        // Return texture ID for egui
        Ok(todo!("Convert wgpu::Texture to egui::TextureId"))
    }
    
    /// Resize render texture
    fn resize_texture(&mut self, size: (u32, u32)) -> anyhow::Result<()> {
        if size.0 == 0 || size.1 == 0 {
            return Ok(());
        }
        
        self.render_texture = Some(self.renderer.create_render_texture(size)?);
        Ok(())
    }
}
```

**Error Handling**: All operations return `anyhow::Result`, no panics.

#### Step 3: Create ViewportRenderer (Day 2, 6 hours)

**File**: `tools/aw_editor/src/viewport/renderer.rs`

**Key Responsibilities**:
- Coordinate rendering pipeline (grid → entities → gizmos → outline)
- Manage GPU resources (buffers, pipelines, bind groups)
- Frame budget tracking (target: <10ms for rendering)

**Code Structure** (700 lines):
```rust
/// Coordinates viewport rendering pipeline
///
/// Renders in multiple passes:
/// 1. Grid overlay (floor plane)
/// 2. Entities (meshes from world)
/// 3. Gizmos (if entity selected)
/// 4. Selection outline (highlight selected entity)
pub struct ViewportRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// Sub-renderers for each pass
    grid_renderer: GridRenderer,
    entity_renderer: EntityRenderer,
    gizmo_renderer: GizmoRenderer,
    
    /// Render pipeline state
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

impl ViewportRenderer {
    pub fn new(render_state: &eframe::egui_wgpu::RenderState) -> anyhow::Result<Self> {
        let device = Arc::new(render_state.device.clone());
        let queue = Arc::new(render_state.queue.clone());
        
        Ok(Self {
            grid_renderer: GridRenderer::new(&device)?,
            entity_renderer: EntityRenderer::new(&device)?,
            gizmo_renderer: GizmoRenderer::new(&device)?,
            depth_texture: todo!("Create depth texture"),
            depth_view: todo!("Create depth view"),
            device,
            queue,
        })
    }
    
    /// Render complete frame
    pub fn render(
        &mut self,
        target: &wgpu::Texture,
        camera: &OrbitCamera,
        world: &World,
        selected: Option<EntityId>,
        gizmo_state: &GizmoState,
    ) -> anyhow::Result<()> {
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Viewport Render Encoder"),
        });
        
        // Clear pass
        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
        // Render grid
        self.grid_renderer.render(&mut encoder, &view, &self.depth_view, camera)?;
        
        // Render entities
        self.entity_renderer.render(&mut encoder, &view, &self.depth_view, camera, world)?;
        
        // Render gizmo (if entity selected)
        if let Some(entity_id) = selected {
            if let Some(transform) = world.get_component::<Transform>(entity_id) {
                self.gizmo_renderer.render(
                    &mut encoder,
                    &view,
                    &self.depth_view,
                    camera,
                    transform,
                    gizmo_state,
                )?;
            }
        }
        
        // Submit
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    /// Create render texture
    pub fn create_render_texture(&self, size: (u32, u32)) -> anyhow::Result<wgpu::Texture> {
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Render Texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }))
    }
}
```

**Performance**: Each render pass budgeted (grid: 0.5ms, entities: 8ms, gizmos: 1ms, total: <10ms).

#### Step 4: Create GridRenderer (Day 2, 3 hours)

**File**: `tools/aw_editor/src/viewport/grid_renderer.rs`

**Key Responsibilities**:
- Render infinite grid on ground plane
- Render XYZ axes (red/green/blue)
- Fade grid at distance (prevent aliasing)

**Code Structure** (300 lines):
```rust
/// Renders infinite grid overlay
///
/// Uses fragment shader trick for infinite grid (no vertex buffers needed).
/// Grid fades at distance to prevent aliasing artifacts.
pub struct GridRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl GridRenderer {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/grid.wgsl").into()),
        });
        
        // Create pipeline
        let pipeline = todo!("Create render pipeline");
        
        // Create uniform buffer (camera matrices)
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Uniform Buffer"),
            size: std::mem::size_of::<GridUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Ok(Self {
            pipeline,
            bind_group: todo!("Create bind group"),
            uniform_buffer,
        })
    }
    
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
    ) -> anyhow::Result<()> {
        // Update uniforms
        let uniforms = GridUniforms {
            view_proj: camera.view_projection_matrix(),
            camera_pos: camera.position(),
            grid_size: 1.0,  // 1 meter grid
            grid_color: [0.5, 0.5, 0.5, 0.3],
        };
        
        // TODO: Write uniforms to buffer
        
        // Render pass
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Grid Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);  // Fullscreen quad
        
        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    grid_size: f32,
    grid_color: [f32; 4],
}
```

**Shader** (`shaders/grid.wgsl`, 100 lines): Infinite grid shader using screen-space technique.

#### Step 5: Integrate with main.rs (Day 3, 2 hours)

**File**: `tools/aw_editor/src/main.rs`

**Changes**:
```rust
// Add viewport field
pub struct EditorApp {
    viewport: ViewportWidget,
    // ... existing fields
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Self {
            viewport: ViewportWidget::new(cc).expect("Failed to create viewport"),
            // ... existing init
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel (scene graph - TODO Phase 2)
                ui.vertical(|ui| {
                    ui.label("Scene Graph");
                    // TODO: Scene tree widget
                });
                
                // Center: 3D Viewport (NEW!)
                if let Err(e) = self.viewport.ui(ui, &self.sim_world) {
                    eprintln!("❌ Viewport error: {}", e);
                }
                
                // Right panel (inspector - existing)
                ui.vertical(|ui| {
                    self.show_inspector(ui);
                });
            });
        });
    }
}
```

#### Step 6: Validation (Day 3, 2 hours)

**Tests**:
1. ✅ Editor launches without panics
2. ✅ Viewport allocates space (70% width)
3. ✅ Grid renders (infinite grid visible)
4. ✅ Camera orbits (mouse drag rotates view)
5. ✅ Camera pans (middle mouse drag)
6. ✅ Camera zooms (scroll wheel)
7. ✅ Frame rate ≥60 FPS (empty scene)

**Deliverable**: Editor with functional 3D viewport showing grid!

---

## Phase 1.2: Orbit Camera (Days 4-5)

### Objective

Professional orbit camera controller (Unity/Blender-style).

### Implementation

**File**: `tools/aw_editor/src/viewport/camera.rs` (400 lines)

**Features**:
- Orbit around focal point (spherical coordinates)
- Pan focal point (screen-space)
- Zoom in/out (distance from focal point)
- Frame selected entity (F key)
- Constraints (min/max distance, pitch limits)

**Code Structure**:
```rust
/// Professional orbit camera controller
///
/// Uses spherical coordinates (distance, yaw, pitch) around a focal point.
/// Supports orbit, pan, zoom, and frame-selected operations.
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    /// Focal point (what camera orbits around)
    focal_point: Vec3,
    
    /// Distance from focal point
    distance: f32,
    
    /// Yaw angle (rotation around Y axis, radians)
    yaw: f32,
    
    /// Pitch angle (rotation around X axis, radians)
    pitch: f32,
    
    /// Field of view (degrees)
    fov: f32,
    
    /// Aspect ratio (width / height)
    aspect: f32,
    
    /// Near/far clip planes
    near: f32,
    far: f32,
    
    /// Constraints
    min_distance: f32,
    max_distance: f32,
    min_pitch: f32,
    max_pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focal_point: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: std::f32::consts::PI / 4.0,  // 45 degrees
            fov: 60.0,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            min_distance: 1.0,
            max_distance: 100.0,
            min_pitch: -std::f32::consts::PI / 2.0 + 0.01,
            max_pitch: std::f32::consts::PI / 2.0 - 0.01,
        }
    }
}

impl OrbitCamera {
    /// Orbit camera (rotate around focal point)
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.01;
        
        self.yaw -= delta_x * SENSITIVITY;
        self.pitch = (self.pitch - delta_y * SENSITIVITY)
            .clamp(self.min_pitch, self.max_pitch);
    }
    
    /// Pan camera (move focal point in screen space)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.01;
        
        // Calculate right and up vectors
        let forward = self.forward();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        
        // Pan in screen space
        let pan_speed = self.distance * SENSITIVITY;
        self.focal_point -= right * delta_x * pan_speed;
        self.focal_point += up * delta_y * pan_speed;
    }
    
    /// Zoom camera (change distance from focal point)
    pub fn zoom(&mut self, delta: f32) {
        const SENSITIVITY: f32 = 0.1;
        
        self.distance = (self.distance - delta * SENSITIVITY)
            .clamp(self.min_distance, self.max_distance);
    }
    
    /// Frame entity (set focal point and distance to view entity)
    pub fn frame_entity(&mut self, entity_pos: Vec3, entity_radius: f32) {
        self.focal_point = entity_pos;
        self.distance = entity_radius * 2.5;  // Nice framing
    }
    
    /// Get camera position (calculated from spherical coords)
    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.yaw.cos() * self.pitch.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.yaw.sin() * self.pitch.cos();
        
        self.focal_point + Vec3::new(x, y, z)
    }
    
    /// Get forward vector
    pub fn forward(&self) -> Vec3 {
        (self.focal_point - self.position()).normalize()
    }
    
    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.focal_point, Vec3::Y)
    }
    
    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect,
            self.near,
            self.far,
        )
    }
    
    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
    
    /// Create ray from screen position (for picking)
    pub fn ray_from_screen(&self, screen_pos: egui::Pos2, viewport_size: egui::Vec2) -> Ray {
        // Convert screen pos to NDC [-1, 1]
        let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0;
        
        // Unproject to world space
        let inv_vp = self.view_projection_matrix().inverse();
        let near_point = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, -1.0));
        let far_point = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
        
        Ray {
            origin: near_point,
            direction: (far_point - near_point).normalize(),
        }
    }
}

/// Ray for picking (origin + direction)
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_orbit_camera_default() {
        let camera = OrbitCamera::default();
        assert_eq!(camera.focal_point, Vec3::ZERO);
        assert_eq!(camera.distance, 10.0);
    }
    
    #[test]
    fn test_orbit_camera_position() {
        let camera = OrbitCamera::default();
        let pos = camera.position();
        assert!((pos.length() - 10.0).abs() < 0.01);
    }
    
    #[test]
    fn test_orbit_camera_zoom() {
        let mut camera = OrbitCamera::default();
        camera.zoom(10.0);  // Zoom in
        assert!(camera.distance < 10.0);
        
        camera.zoom(-20.0);  // Zoom out
        assert!(camera.distance > 9.0);
    }
    
    #[test]
    fn test_frame_entity() {
        let mut camera = OrbitCamera::default();
        camera.frame_entity(Vec3::new(5.0, 0.0, 5.0), 2.0);
        assert_eq!(camera.focal_point, Vec3::new(5.0, 0.0, 5.0));
        assert_eq!(camera.distance, 5.0);  // 2.0 * 2.5
    }
}
```

**Testing**:
- Unit tests for camera math (position, zoom, frame)
- Manual tests for controls (orbit, pan, zoom feel)
- Performance: Camera update <0.1ms

**Deliverable**: Smooth, professional camera controls matching Unity/Blender UX.

---

## Phase 1.3: Entity Rendering (Days 6-7)

**File**: `tools/aw_editor/src/viewport/entity_renderer.rs` (600 lines)

**Implementation**: Render entities from `sim_world` as simple shapes (boxes/spheres) with color coding.

**Details in next document** (continuing implementation plan...)

---

## Code Quality Standards

### Error Handling
- ✅ **NO `.unwrap()` or `.expect()`** in production code
- ✅ **Use `anyhow::Result`** for all fallible operations
- ✅ **Provide context** with `.context()` on all errors
- ✅ **Graceful degradation** (render empty scene if error, don't crash)

### Performance
- ✅ **60 FPS minimum** (16.67ms frame budget)
- ✅ **Profile critical paths** (Tracy integration)
- ✅ **Batch GPU operations** (minimize draw calls)
- ✅ **Lazy resource creation** (don't allocate until needed)

### Documentation
- ✅ **Rustdoc on all public items** (`///` comments)
- ✅ **Examples in docs** (especially complex APIs)
- ✅ **Module-level docs** explaining architecture
- ✅ **TODO comments** for deferred work (with context)

### Testing
- ✅ **Unit tests** for math, utilities
- ✅ **Integration tests** for rendering (snapshot tests)
- ✅ **Manual tests** documented (test plan)
- ✅ **Benchmark critical paths** (camera updates, rendering)

---

## Next Session: Implementation Begins

**Ready to start**: Phase 1.1 Day 1 - egui_wgpu integration and ViewportWidget scaffold.

**First concrete task**: Add dependencies, create `viewport/` module structure, implement `ViewportWidget::new()`.

Let's build something incredible! 🚀
