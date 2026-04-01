//! Velocity buffer (motion vector) generation for temporal effects.
//!
//! Provides per-pixel screen-space motion vectors by comparing current and previous
//! frame projections. Required by TAA, motion blur, temporal upscaling, and SSGI
//! temporal reprojection.

/// GPU-side uniform for the velocity pass: current and previous view-projection matrices.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VelocityUniforms {
    pub curr_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
}

impl Default for VelocityUniforms {
    fn default() -> Self {
        Self {
            curr_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            prev_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

/// GPU-side uniform for per-object previous transform (for moving objects).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PrevTransformUniforms {
    pub prev_model: [[f32; 4]; 4],
}

impl Default for PrevTransformUniforms {
    fn default() -> Self {
        Self {
            prev_model: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

/// Manages the velocity buffer resources and previous-frame state.
pub struct VelocityBuffer {
    /// Previous frame's view-projection matrix.
    prev_view_proj: glam::Mat4,
    /// GPU uniform buffer for velocity pass.
    uniform_buf: wgpu::Buffer,
    /// GPU uniform buffer for per-object previous transform.
    prev_transform_buf: wgpu::Buffer,
    /// Bind group layout for the velocity pass.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for the velocity pass.
    bind_group: wgpu::BindGroup,
    /// Velocity render target texture.
    velocity_texture: wgpu::Texture,
    /// Velocity render target view.
    velocity_view: wgpu::TextureView,
    /// Dimensions.
    width: u32,
    height: u32,
}

impl VelocityBuffer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        use wgpu::util::DeviceExt;

        let uniforms = VelocityUniforms::default();
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocity_uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let prev_transform = PrevTransformUniforms::default();
        let prev_transform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocity_prev_transform"),
            contents: bytemuck::bytes_of(&prev_transform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velocity_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velocity_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: prev_transform_buf.as_entire_binding(),
                },
            ],
        });

        let velocity_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velocity_buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let velocity_view = velocity_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            prev_view_proj: glam::Mat4::IDENTITY,
            uniform_buf,
            prev_transform_buf,
            bind_group_layout,
            bind_group,
            velocity_texture,
            velocity_view,
            width,
            height,
        }
    }

    /// Update uniforms for this frame. Call once per frame before rendering.
    /// `curr_view_proj` is the current frame's combined view * projection matrix.
    pub fn begin_frame(&mut self, queue: &wgpu::Queue, curr_view_proj: glam::Mat4) {
        let uniforms = VelocityUniforms {
            curr_view_proj: curr_view_proj.to_cols_array_2d(),
            prev_view_proj: self.prev_view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Commit the current frame's view-projection as previous for next frame.
    /// Call once at the end of the frame after all rendering is complete.
    pub fn end_frame(&mut self, curr_view_proj: glam::Mat4) {
        self.prev_view_proj = curr_view_proj;
    }

    /// Update per-object previous transform uniform.
    pub fn set_prev_transform(&self, queue: &wgpu::Queue, prev_model: glam::Mat4) {
        let u = PrevTransformUniforms {
            prev_model: prev_model.to_cols_array_2d(),
        };
        queue.write_buffer(&self.prev_transform_buf, 0, bytemuck::bytes_of(&u));
    }

    /// Resize the velocity buffer.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.velocity_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velocity_buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.velocity_view = self
            .velocity_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.width = width;
        self.height = height;
    }

    /// Get the velocity texture view for sampling in post-processing.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.velocity_view
    }

    /// Get the velocity texture for read-back or further processing.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.velocity_texture
    }

    /// Get the bind group layout (for pipeline creation).
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get the bind group (for rendering).
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get a render pass color attachment for the velocity buffer.
    pub fn color_attachment(&self) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: &self.velocity_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        }
    }

    /// Get the previous frame's view-projection matrix.
    pub fn prev_view_proj(&self) -> glam::Mat4 {
        self.prev_view_proj
    }

    /// Get dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_uniforms_default() {
        let u = VelocityUniforms::default();
        let id = glam::Mat4::IDENTITY.to_cols_array_2d();
        assert_eq!(u.curr_view_proj, id);
        assert_eq!(u.prev_view_proj, id);
    }

    #[test]
    fn velocity_uniforms_pod_size() {
        assert_eq!(
            std::mem::size_of::<VelocityUniforms>(),
            128, // 2 * 4x4 f32 matrices = 2 * 64 bytes
        );
    }

    #[test]
    fn prev_transform_uniforms_default() {
        let u = PrevTransformUniforms::default();
        let id = glam::Mat4::IDENTITY.to_cols_array_2d();
        assert_eq!(u.prev_model, id);
    }

    #[test]
    fn prev_transform_uniforms_pod_size() {
        assert_eq!(
            std::mem::size_of::<PrevTransformUniforms>(),
            64, // 1 * 4x4 f32 matrix = 64 bytes
        );
    }

    #[test]
    fn velocity_buffer_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let vb = VelocityBuffer::new(&device, 1920, 1080);
        assert_eq!(vb.dimensions(), (1920, 1080));
        assert_eq!(vb.prev_view_proj(), glam::Mat4::IDENTITY);
    }

    #[test]
    fn velocity_buffer_end_frame_stores_prev() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let mut vb = VelocityBuffer::new(&device, 800, 600);
        let test_mat = glam::Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0));
        vb.end_frame(test_mat);
        assert_eq!(vb.prev_view_proj(), test_mat);
    }

    #[test]
    fn velocity_buffer_resize() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let mut vb = VelocityBuffer::new(&device, 800, 600);
        vb.resize(&device, 1920, 1080);
        assert_eq!(vb.dimensions(), (1920, 1080));
        // Resize to same size should be a no-op
        vb.resize(&device, 1920, 1080);
        assert_eq!(vb.dimensions(), (1920, 1080));
    }
}
