//! Free-fly camera producer.
//!
//! `FreeFly` is the canonical engine free-fly camera, formerly known as
//! `astraweave_render::camera::Camera`. Moved into `astraweave-camera` in
//! sub-phase C.3.A of the Unified Camera campaign. See
//! `docs/current/CAMERA_CONVENTIONS.md` for the canonical conventions this
//! type complies with; specifically:
//!
//! - §2.1: `fovy` stores radians.
//! - §2.2: wgpu [0, 1] depth.
//! - §2.4: right-handed, +Y up.
//! - §2.5: view matrix built via `Mat4::look_to_rh`.
//! - §2.6: `Mat4::perspective_rh` for projection.
//! - §2.8: at yaw=0, pitch=0, [`FreeFly::dir`] returns `Vec3::X` (+X forward).
//! - §2.9: implements [`crate::CameraProducer`].

use glam::{Mat4, Vec2, Vec3};

use crate::{CameraProducer, Projection, RenderView};

/// Free-fly camera (yaw/pitch/position with explicit projection parameters).
///
/// Formerly `astraweave_render::camera::Camera`; renamed during C.3.A to
/// reflect that it's one producer among several in the unified camera
/// system. The original name is preserved as a backward-compatibility shim
/// at `astraweave-render/src/camera.rs` (deleted in C.3.C).
pub struct FreeFly {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl FreeFly {
    pub fn view_matrix(&self) -> Mat4 {
        let dir = Self::dir(self.yaw, self.pitch);
        // Standard right-handed view: camera looks along `dir` with world +Y up.
        // Previous `-Vec3::Y` caused clip-space w to go negative for all
        // visible geometry when the editor's OrbitCamera fed its yaw/pitch
        // directly, producing chunk-aligned rectangular voids in terrain.
        Mat4::look_to_rh(self.position, dir, Vec3::Y)
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect.max(0.01), self.znear, self.zfar)
    }

    pub fn vp(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }

    pub fn dir(yaw: f32, pitch: f32) -> Vec3 {
        let cy = yaw.cos();
        let sy = yaw.sin();
        let cp = pitch.cos();
        let sp = pitch.sin();
        // Standard direction: positive pitch = look up
        Vec3::new(cy * cp, sp, sy * cp).normalize()
    }

    /// View matrix with the camera placed at the world origin (rotation only).
    ///
    /// Used for camera-relative rendering: all geometry is offset on the CPU so
    /// the view matrix carries no large translation, avoiding f32 jitter at
    /// large world coordinates.
    pub fn view_matrix_camera_relative(&self) -> Mat4 {
        let dir = Self::dir(self.yaw, self.pitch);
        Mat4::look_to_rh(Vec3::ZERO, dir, Vec3::Y)
    }

    /// Camera-relative `RenderView` from FreeFly state.
    ///
    /// Builds the view matrix with camera position pre-subtracted (eye is at
    /// origin in the resulting view space); world geometry is expected to be
    /// transformed accordingly by the consuming pipeline. Per
    /// `CAMERA_CONVENTIONS.md` §2.9, this is **not** on the
    /// [`CameraProducer`] trait — it's an opt-in concrete capability. Trait
    /// callers (e.g. cinematics blenders, generic camera managers) get the
    /// world-relative view from [`CameraProducer::to_render_view`]; producers
    /// that need camera-relative rendering call this method on the concrete
    /// `FreeFly` type.
    ///
    /// `position` field of the returned `RenderView` reflects the original
    /// world position (consumers may need it for world-space reconstruction,
    /// fog distance, etc.). Only the matrices are camera-relative.
    pub fn to_render_view_camera_relative(&self) -> RenderView {
        let projection = Projection::perspective(self.fovy, self.aspect, self.znear, self.zfar);
        let view = self.view_matrix_camera_relative();
        let view_dir = Self::dir(self.yaw, self.pitch);
        RenderView::new(view, &projection, self.position, view_dir)
    }

    /// Clamp invalid configurations to valid ones.
    ///
    /// Callers invoke explicitly when they may have received pathological
    /// input (e.g., deserialization from an untrusted source; user-modified
    /// scene state; programmatically-generated camera fixtures).
    /// `sanitize()` does **not** run automatically at projection time —
    /// that path is hot and pathological inputs are rare. Callers who
    /// want defensive validation invoke this method.
    ///
    /// Specifically, `sanitize()` ensures:
    ///
    /// - `self.fovy` is in `[10°, 170°].to_radians()` (sensible range;
    ///   matches `astraweave_cinematics::CameraKey::sanitize`'s
    ///   `[10°, 170°]` clamp range. Pre-C.7.D the cinematics layer
    ///   had a separate `is_typical_fov` query returning a bool for the
    ///   tighter `30°..=120°` range, but it had zero callers; C.7.D
    ///   removed it and harmonized both layers to this wider canonical
    ///   range).
    /// - `self.znear` is `> 0.0001` (tiny but non-zero; prevents
    ///   division-by-zero in `Mat4::perspective_rh` while preserving
    ///   the geometric meaning of "near plane").
    /// - `self.zfar` is `> self.znear + 0.001` (prevents degenerate
    ///   projection when zfar == znear, where the perspective matrix
    ///   collapses).
    /// - `self.aspect` is `>= 0.01` (matches the existing aspect guard
    ///   at [`Self::proj_matrix`] which calls `self.aspect.max(0.01)`;
    ///   `sanitize()` updates the field itself so subsequent reads
    ///   observe the clamped value).
    ///
    /// Added in Unified Camera campaign sub-phase C.6.F per C.5 audit
    /// finding L.5.16 (missing FOV/near-far validation).
    pub fn sanitize(&mut self) {
        let fovy_min = 10_f32.to_radians();
        let fovy_max = 170_f32.to_radians();
        self.fovy = self.fovy.clamp(fovy_min, fovy_max);
        self.znear = self.znear.max(0.0001);
        self.zfar = self.zfar.max(self.znear + 0.001);
        self.aspect = self.aspect.max(0.01);
    }
}

impl CameraProducer for FreeFly {
    /// World-relative `RenderView` from FreeFly state.
    ///
    /// Per `CAMERA_CONVENTIONS.md` §2.9, this is the canonical trait
    /// implementation: world-space view matrix, no camera-relative
    /// translation. For camera-relative rendering (used by certain
    /// shadow/atmospheric paths to mitigate large-world float precision),
    /// call [`FreeFly::to_render_view_camera_relative`] instead.
    fn to_render_view(&self) -> RenderView {
        let projection = Projection::perspective(self.fovy, self.aspect, self.znear, self.zfar);
        let view = self.view_matrix();
        let view_dir = Self::dir(self.yaw, self.pitch);
        RenderView::new(view, &projection, self.position, view_dir)
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum CameraMode {
    FreeFly,
    Orbit,
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub zoom_sensitivity: f32,
    /// Exponential smoothing factor for mouse look [0..1], higher = snappier
    pub mouse_smooth: f32,
    /// Ignore tiny mouse deltas (raw input noise)
    pub mouse_deadzone: f32,
    pub mode: CameraMode,
    pub orbit_target: Vec3,
    pub orbit_distance: f32,
    // Smoothed look targets
    yaw_target: f32,
    pitch_target: f32,
    targets_initialized: bool,
    fwd: f32,
    back: f32,
    left: f32,
    right: f32,
    up: f32,
    down: f32,
    // Speed modifiers
    sprint_active: bool,
    precision_active: bool,
    sprint_mult: f32,
    precision_mult: f32,
    dragging: bool,
    last_mouse: Option<Vec2>,
    raw_used_this_frame: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            zoom_sensitivity: 0.1,
            mouse_smooth: 0.15,
            mouse_deadzone: 0.25,
            mode: CameraMode::FreeFly,
            orbit_target: Vec3::ZERO,
            orbit_distance: 5.0,
            yaw_target: 0.0,
            pitch_target: 0.0,
            targets_initialized: false,
            fwd: 0.0,
            back: 0.0,
            left: 0.0,
            right: 0.0,
            up: 0.0,
            down: 0.0,
            sprint_active: false,
            precision_active: false,
            sprint_mult: 2.0,
            precision_mult: 0.25,
            dragging: false,
            last_mouse: None,
            raw_used_this_frame: false,
        }
    }

    /// Is the right-mouse look active?
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn process_keyboard(&mut self, key: winit::keyboard::KeyCode, pressed: bool) {
        let v = if pressed { 1.0 } else { 0.0 };
        match key {
            winit::keyboard::KeyCode::KeyW => self.fwd = v,
            winit::keyboard::KeyCode::KeyS => self.back = v,
            winit::keyboard::KeyCode::KeyA => self.left = v,
            winit::keyboard::KeyCode::KeyD => self.right = v,
            // Support both Space and 'E' for up
            winit::keyboard::KeyCode::Space | winit::keyboard::KeyCode::KeyE => self.up = v,
            // Support both 'Q' and 'C' for down
            winit::keyboard::KeyCode::KeyQ | winit::keyboard::KeyCode::KeyC => self.down = v,
            // Modifiers: Shift = sprint, Ctrl = precision/slow
            winit::keyboard::KeyCode::ShiftLeft | winit::keyboard::KeyCode::ShiftRight => {
                self.sprint_active = pressed;
            }
            winit::keyboard::KeyCode::ControlLeft | winit::keyboard::KeyCode::ControlRight => {
                self.precision_active = pressed;
            }
            _ => {}
        }
    }

    pub fn process_mouse_button(&mut self, button: winit::event::MouseButton, pressed: bool) {
        if button == winit::event::MouseButton::Right {
            self.dragging = pressed;
            if !pressed {
                self.last_mouse = None;
            }
        }
    }

    pub fn process_mouse_move(&mut self, camera: &mut FreeFly, pos: Vec2) {
        if self.dragging {
            // If raw deltas already consumed this frame, skip absolute to avoid double-apply
            if self.raw_used_this_frame {
                return;
            }
            if let Some(last) = self.last_mouse {
                let delta = (pos - last) * self.sensitivity;
                // Update smooth targets (actual camera moves toward these in update_camera)
                if !self.targets_initialized {
                    self.yaw_target = camera.yaw;
                    self.pitch_target = camera.pitch;
                    self.targets_initialized = true;
                }
                self.yaw_target -= delta.x;
                self.pitch_target = (self.pitch_target - delta.y).clamp(-1.54, 1.54);
            }
            self.last_mouse = Some(pos);
        }
    }

    pub fn process_mouse_delta(&mut self, camera: &mut FreeFly, delta: Vec2) {
        if self.dragging {
            // Apply deadzone to raw deltas to avoid drift
            if delta.x.abs() < self.mouse_deadzone && delta.y.abs() < self.mouse_deadzone {
                return;
            }
            let scaled_delta = delta * self.sensitivity;
            if !self.targets_initialized {
                self.yaw_target = camera.yaw;
                self.pitch_target = camera.pitch;
                self.targets_initialized = true;
            }
            self.yaw_target -= scaled_delta.x;
            self.pitch_target = (self.pitch_target - scaled_delta.y).clamp(-1.54, 1.54);
            self.raw_used_this_frame = true;
        }
    }

    /// Reset per-frame input accumulation flags; call once per frame before events
    pub fn begin_frame(&mut self) {
        self.raw_used_this_frame = false;
    }

    pub fn process_scroll(&mut self, camera: &mut FreeFly, delta: f32) {
        match self.mode {
            CameraMode::FreeFly => {
                // Zoom by adjusting FOV
                let fov_delta = delta * self.zoom_sensitivity;
                camera.fovy = (camera.fovy - fov_delta).clamp(0.1, 3.0);
            }
            CameraMode::Orbit => {
                // Zoom by adjusting orbit distance
                self.orbit_distance = (self.orbit_distance - delta * 0.5).clamp(1.0, 50.0);
                self.update_orbit_position(camera);
            }
        }
    }

    pub fn toggle_mode(&mut self, camera: &mut FreeFly) {
        match self.mode {
            CameraMode::FreeFly => {
                self.mode = CameraMode::Orbit;
                // Set orbit target to current look direction
                let look_dir = FreeFly::dir(camera.yaw, camera.pitch);
                self.orbit_target = camera.position + look_dir * self.orbit_distance;
            }
            CameraMode::Orbit => {
                self.mode = CameraMode::FreeFly;
                // Keep current position when switching to free fly
            }
        }
    }

    pub fn set_orbit_target(&mut self, target: Vec3, camera: &mut FreeFly) {
        self.orbit_target = target;
        if matches!(self.mode, CameraMode::Orbit) {
            self.update_orbit_position(camera);
        }
    }

    fn update_orbit_position(&mut self, camera: &mut FreeFly) {
        let dir = FreeFly::dir(camera.yaw, camera.pitch);
        camera.position = self.orbit_target - dir * self.orbit_distance;
    }

    pub fn update_camera(&mut self, camera: &mut FreeFly, dt: f32) {
        // Initialize look targets on first update
        if !self.targets_initialized {
            self.yaw_target = camera.yaw;
            self.pitch_target = camera.pitch;
            self.targets_initialized = true;
        }

        // Exponential smoothing toward targets; dt-aware
        let t = 1.0 - (-self.mouse_smooth * dt.max(1e-4)).exp();
        camera.yaw = camera.yaw + (self.yaw_target - camera.yaw) * t;
        camera.pitch = (camera.pitch + (self.pitch_target - camera.pitch) * t).clamp(-1.54, 1.54);

        // Effective speed with runtime modifiers
        let mut eff_speed = self.speed;
        if self.sprint_active {
            eff_speed *= self.sprint_mult;
        }
        if self.precision_active {
            eff_speed *= self.precision_mult;
        }

        match self.mode {
            CameraMode::FreeFly => {
                let dir = FreeFly::dir(camera.yaw, camera.pitch);
                let right = dir.cross(Vec3::Y).normalize();
                let up = Vec3::Y;

                let mut vel = Vec3::ZERO;
                vel += dir * (self.fwd - self.back);
                vel += right * (self.right - self.left);
                vel += up * (self.up - self.down);
                if vel.length_squared() > 0.0 {
                    camera.position += vel.normalize() * eff_speed * dt;
                }
            }
            CameraMode::Orbit => {
                // In orbit mode, WASD moves the orbit target
                let dir = FreeFly::dir(camera.yaw, camera.pitch);
                let right = dir.cross(Vec3::Y).normalize();
                let forward = Vec3::new(dir.x, 0.0, dir.z).normalize(); // Horizontal movement only
                let up = Vec3::Y;

                let mut target_vel = Vec3::ZERO;
                target_vel += forward * (self.fwd - self.back);
                target_vel += right * (self.right - self.left);
                target_vel += up * (self.up - self.down);

                if target_vel.length_squared() > 0.0 {
                    self.orbit_target += target_vel.normalize() * eff_speed * dt;
                }
                // Always recompute camera position from current yaw/pitch and orbit distance
                // so that mouse look in Orbit mode rotates around the orbit target even without WASD input.
                self.update_orbit_position(camera);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_basic_functionality() {
        let camera = FreeFly {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // Test view matrix generation
        let view_mat = camera.view_matrix();
        assert!(!view_mat.is_nan());

        // Test projection matrix generation
        let proj_mat = camera.proj_matrix();
        assert!(!proj_mat.is_nan());

        // Test direction calculation
        let dir = FreeFly::dir(0.0, 0.0);
        assert!((dir - Vec3::new(1.0, 0.0, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_camera_controller_movement() {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // Test keyboard input processing
        controller.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        assert_eq!(controller.fwd, 1.0);

        controller.process_keyboard(winit::keyboard::KeyCode::KeyW, false);
        assert_eq!(controller.fwd, 0.0);

        // Test camera update
        controller.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        let initial_pos = camera.position;
        controller.update_camera(&mut camera, 0.1);

        // Camera should have moved forward
        assert!(camera.position != initial_pos);
    }

    #[test]
    fn test_camera_zoom_functionality() {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let initial_fov = camera.fovy;

        // Test zoom in
        controller.process_scroll(&mut camera, 1.0);
        assert!(camera.fovy < initial_fov);

        // Test zoom out
        controller.process_scroll(&mut camera, -1.0);
        assert!(camera.fovy > initial_fov - 0.1);
    }

    #[test]
    fn test_camera_mode_toggle() {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = FreeFly {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // Initially in FreeFly mode
        assert!(matches!(controller.mode, CameraMode::FreeFly));

        // Toggle to Orbit mode
        controller.toggle_mode(&mut camera);
        assert!(matches!(controller.mode, CameraMode::Orbit));
        assert!(controller.orbit_target != Vec3::ZERO);

        // Toggle back to FreeFly mode
        controller.toggle_mode(&mut camera);
        assert!(matches!(controller.mode, CameraMode::FreeFly));
    }

    #[test]
    fn test_orbit_mode_behavior() {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = FreeFly {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // Switch to orbit mode
        controller.toggle_mode(&mut camera);

        // Test orbit distance zoom
        let initial_distance = controller.orbit_distance;
        controller.process_scroll(&mut camera, 1.0);
        assert!(controller.orbit_distance < initial_distance);

        // Test orbit target movement
        let initial_target = controller.orbit_target;
        controller.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        controller.update_camera(&mut camera, 0.1);
        assert!(controller.orbit_target != initial_target);
    }

    #[test]
    fn test_mouse_delta_processing() {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // Test that mouse delta processing works without dragging
        let initial_yaw = camera.yaw;
        let initial_pitch = camera.pitch;
        controller.process_mouse_delta(&mut camera, Vec2::new(10.0, 5.0));
        // Without dragging, targets won't update and camera won't change on update
        controller.update_camera(&mut camera, 0.016);
        assert!((camera.yaw - initial_yaw).abs() < 1e-6);
        assert!((camera.pitch - initial_pitch).abs() < 1e-6);

        // Enable dragging
        controller.process_mouse_button(winit::event::MouseButton::Right, true);

        // Test mouse delta processing with dragging
        let initial_yaw = camera.yaw;
        let initial_pitch = camera.pitch;
        controller.process_mouse_delta(&mut camera, Vec2::new(10.0, 5.0));
        // Apply update to realize smoothed motion
        controller.update_camera(&mut camera, 0.016);
        // Yaw should decrease (negative delta.x)
        assert!(camera.yaw < initial_yaw);
        // Pitch should decrease (negative delta.y)
        assert!(camera.pitch < initial_pitch);

        // Test orbit mode delta processing
        controller.toggle_mode(&mut camera);
        let initial_pos = camera.position;
        controller.process_mouse_delta(&mut camera, Vec2::new(5.0, 0.0));
        // Apply update to realize orbit movement from accumulated deltas
        controller.update_camera(&mut camera, 0.016);
        // Position should change due to orbit update
        assert!(camera.position != initial_pos);
    }

    #[test]
    fn view_matrix_camera_relative_strips_translation() {
        let camera = FreeFly {
            position: Vec3::new(10000.0, 500.0, 20000.0),
            yaw: 0.5,
            pitch: 0.2,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let view_std = camera.view_matrix();
        let view_cr = camera.view_matrix_camera_relative();

        // Rotation part (upper-left 3x3) should be identical
        for col in 0..3 {
            let c_std = [
                view_std.col(col).x,
                view_std.col(col).y,
                view_std.col(col).z,
            ];
            let c_cr = [view_cr.col(col).x, view_cr.col(col).y, view_cr.col(col).z];
            for i in 0..3 {
                assert!(
                    (c_std[i] - c_cr[i]).abs() < 1e-6,
                    "rotation mismatch at col={col} row={i}"
                );
            }
        }

        // Translation column (w_axis) should be zero for camera-relative
        assert!((view_cr.w_axis.x).abs() < 1e-6);
        assert!((view_cr.w_axis.y).abs() < 1e-6);
        assert!((view_cr.w_axis.z).abs() < 1e-6);
        assert!((view_cr.w_axis.w - 1.0).abs() < 1e-6);

        // Standard view should have non-zero translation (camera far from origin)
        assert!(view_std.w_axis.truncate().length() > 1.0);
    }

    // ─────────────────────────────────────────────────────────────────────
    // CameraProducer impl tests (added in C.3.A)
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn to_render_view_yaw_zero_pitch_zero_has_x_forward() {
        // CAMERA_CONVENTIONS.md §2.8 — the producer-side anchor for the
        // canonical convention. FreeFly at yaw=0, pitch=0 must emit a
        // RenderView with view_dir == +X.
        let cam = FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let rv = cam.to_render_view();
        assert!(
            (rv.view_dir - Vec3::X).length() < 1e-5,
            "FreeFly::to_render_view() at yaw=0,pitch=0 must emit view_dir=+X per §2.8; got {:?}",
            rv.view_dir
        );
    }

    #[test]
    fn to_render_view_camera_relative_position_field_preserved() {
        // Even though the view matrix has translation stripped, the position
        // field of RenderView still reports the original world position so
        // consumers can do world-space reconstruction (fog distance, etc.).
        let cam = FreeFly {
            position: Vec3::new(10000.0, 500.0, 20000.0),
            yaw: 0.7,
            pitch: 0.2,
            fovy: 60f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let rv = cam.to_render_view_camera_relative();
        assert_eq!(
            rv.position, cam.position,
            "to_render_view_camera_relative must preserve world position in RenderView.position"
        );

        // And the view matrix's translation column must be ZERO (camera at origin).
        assert!(rv.view.w_axis.truncate().length() < 1e-4);
    }

    // ─────────────────────────────────────────────────────────────────────
    // sanitize() contract tests (C.6.F per C.5 audit L.5.16)
    // ─────────────────────────────────────────────────────────────────────

    /// Fixture: a default valid FreeFly state for sanitize() tests.
    /// Mirrors the existing in-file test pattern (literal struct expression
    /// rather than a Default impl, which FreeFly intentionally does not
    /// provide).
    fn valid_fixture() -> FreeFly {
        FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60_f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }

    #[test]
    fn sanitize_clamps_fovy_below_minimum() {
        let mut cam = valid_fixture();
        cam.fovy = 5_f32.to_radians(); // below 10° minimum
        cam.sanitize();
        let expected = 10_f32.to_radians();
        assert!(
            (cam.fovy - expected).abs() < 1e-6,
            "fovy should clamp to 10° minimum; got {} radians, expected {}",
            cam.fovy,
            expected,
        );
    }

    #[test]
    fn sanitize_clamps_fovy_above_maximum() {
        let mut cam = valid_fixture();
        cam.fovy = 200_f32.to_radians(); // above 170° maximum
        cam.sanitize();
        let expected = 170_f32.to_radians();
        assert!(
            (cam.fovy - expected).abs() < 1e-6,
            "fovy should clamp to 170° maximum; got {} radians, expected {}",
            cam.fovy,
            expected,
        );
    }

    #[test]
    fn sanitize_clamps_znear_to_positive() {
        let mut cam = valid_fixture();
        cam.znear = -1.0; // negative
        cam.sanitize();
        assert!(
            cam.znear > 0.0,
            "znear should clamp to positive; got {}",
            cam.znear
        );
    }

    #[test]
    fn sanitize_ensures_zfar_greater_than_znear() {
        let mut cam = valid_fixture();
        cam.znear = 10.0;
        cam.zfar = 5.0; // less than znear (pathological)
        cam.sanitize();
        assert!(
            cam.zfar > cam.znear,
            "zfar should exceed znear; got znear={} zfar={}",
            cam.znear,
            cam.zfar,
        );
    }

    #[test]
    fn sanitize_clamps_aspect_to_minimum() {
        let mut cam = valid_fixture();
        cam.aspect = 0.0; // pathological
        cam.sanitize();
        assert!(
            cam.aspect >= 0.01,
            "aspect should clamp to 0.01 minimum; got {}",
            cam.aspect,
        );
    }

    #[test]
    fn sanitize_is_idempotent_on_valid_state() {
        let mut cam = valid_fixture();
        let before = (cam.fovy, cam.znear, cam.zfar, cam.aspect);
        cam.sanitize();
        let after = (cam.fovy, cam.znear, cam.zfar, cam.aspect);
        assert_eq!(
            before, after,
            "sanitize on already-valid state should be a no-op"
        );
    }
}
