// MSAA (Multisample Anti-Aliasing) Configuration and Utilities
//
// Provides configurable MSAA support for render pipelines with automatic
// MSAA texture management and resolve operations.

use anyhow::Result;
use wgpu;

/// MSAA sample count configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MsaaMode {
    /// No anti-aliasing
    Off,
    /// 2x MSAA
    X2,
    /// 4x MSAA (recommended)
    #[default]
    X4,
    /// 8x MSAA (high quality, more expensive)
    X8,
}

impl MsaaMode {
    /// Get the wgpu sample count
    pub fn sample_count(self) -> u32 {
        match self {
            MsaaMode::Off => 1,
            MsaaMode::X2 => 2,
            MsaaMode::X4 => 4,
            MsaaMode::X8 => 8,
        }
    }

    /// Check if MSAA is enabled
    pub fn is_enabled(self) -> bool {
        self != MsaaMode::Off
    }

    /// Get the wgpu MultisampleState for this mode
    pub fn multisample_state(self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: self.sample_count(),
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    /// Get MultisampleState with alpha-to-coverage enabled.
    /// Use for vegetation and masked-alpha pipelines when MSAA is active.
    pub fn multisample_state_alpha_to_coverage(self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: self.sample_count(),
            mask: !0,
            alpha_to_coverage_enabled: self.is_enabled(),
        }
    }
}

/// MSAA render target manager
///
/// Manages MSAA textures and provides automatic creation/recreation when
/// the window size or MSAA mode changes.
pub struct MsaaRenderTarget {
    /// Current MSAA mode
    mode: MsaaMode,
    /// MSAA texture (multisampled)
    msaa_texture: Option<wgpu::Texture>,
    /// MSAA texture view
    msaa_view: Option<wgpu::TextureView>,
    /// Current texture size
    width: u32,
    height: u32,
    /// Texture format
    format: wgpu::TextureFormat,
}

impl MsaaRenderTarget {
    /// Create a new MSAA render target manager
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            mode: MsaaMode::default(),
            msaa_texture: None,
            msaa_view: None,
            width: 0,
            height: 0,
            format,
        }
    }

    /// Set the MSAA mode (recreates texture if changed)
    pub fn set_mode(&mut self, device: &wgpu::Device, mode: MsaaMode) -> Result<()> {
        if mode != self.mode {
            self.mode = mode;
            if self.width > 0 && self.height > 0 {
                self.create_texture(device, self.width, self.height)?;
            }
        }
        Ok(())
    }

    /// Get the current MSAA mode
    pub fn mode(&self) -> MsaaMode {
        self.mode
    }

    /// Resize the MSAA texture (recreates if size changed)
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) -> Result<()> {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            if self.mode.is_enabled() {
                self.create_texture(device, width, height)?;
            }
        }
        Ok(())
    }

    /// Create or recreate the MSAA texture
    fn create_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) -> Result<()> {
        if !self.mode.is_enabled() {
            self.msaa_texture = None;
            self.msaa_view = None;
            return Ok(());
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.mode.sample_count(),
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.msaa_texture = Some(texture);
        self.msaa_view = Some(view);

        Ok(())
    }

    /// Get the MSAA texture view (returns None if MSAA is disabled)
    pub fn view(&self) -> Option<&wgpu::TextureView> {
        self.msaa_view.as_ref()
    }

    /// Get the color attachment for render pass
    ///
    /// Returns the appropriate color attachment configuration:
    /// - If MSAA enabled: MSAA view as attachment, resolve target as resolve
    /// - If MSAA disabled: None (caller should use resolve target directly)
    pub fn color_attachment<'a>(
        &'a self,
        resolve_target: &'a wgpu::TextureView,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) -> wgpu::RenderPassColorAttachment<'a> {
        if let Some(msaa_view) = &self.msaa_view {
            wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(resolve_target),
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            }
        } else {
            wgpu::RenderPassColorAttachment {
                view: resolve_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            }
        }
    }
}

/// Helper to create MSAA-compatible depth texture
pub fn create_msaa_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    msaa_mode: MsaaMode,
    label: Option<&str>,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: msaa_mode.sample_count(),
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msaa_mode_sample_counts() {
        assert_eq!(MsaaMode::Off.sample_count(), 1);
        assert_eq!(MsaaMode::X2.sample_count(), 2);
        assert_eq!(MsaaMode::X4.sample_count(), 4);
        assert_eq!(MsaaMode::X8.sample_count(), 8);
    }

    #[test]
    fn test_msaa_mode_is_enabled() {
        assert!(!MsaaMode::Off.is_enabled());
        assert!(MsaaMode::X2.is_enabled());
        assert!(MsaaMode::X4.is_enabled());
        assert!(MsaaMode::X8.is_enabled());
    }

    #[test]
    fn test_msaa_mode_default() {
        assert_eq!(MsaaMode::default(), MsaaMode::X4);
    }

    #[test]
    fn test_msaa_multisample_state() {
        let state = MsaaMode::X4.multisample_state();
        assert_eq!(state.count, 4);
        assert_eq!(state.mask, !0);
        assert!(!state.alpha_to_coverage_enabled);
    }

    #[test]
    fn test_msaa_render_target_new() {
        let target = MsaaRenderTarget::new(wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(target.mode(), MsaaMode::X4);
        assert!(target.view().is_none());
    }
}
