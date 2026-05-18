//! Camera, Primitives & Instancing Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Camera operations (view/projection matrix, direction calculations)
//! - CameraController (mouse/keyboard input, update loop)
//! - Primitive generation (cube, plane, sphere)
//! - Instancing (transforms, batches, patterns)
//! - Overlay effects (parameters, updates)
//! - Effects system (weather particles)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::{Mat4, Quat, Vec2, Vec3};
use std::hint::black_box;
use std::hint::black_box as bb;

// ============================================================================
// Mock Types (matching actual API without wgpu dependencies)
// ============================================================================

/// Camera structure for view/projection calculations
#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        let dir = Self::dir(self.yaw, self.pitch);
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
        Vec3::new(cy * cp, sp, sy * cp).normalize()
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }
}

/// Camera mode enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    FreeFly,
    Orbit,
}

/// Camera controller for input handling
#[derive(Clone, Debug)]
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub mouse_smooth: f32,
    pub mouse_deadzone: f32,
    pub mode: CameraMode,
    pub orbit_target: Vec3,
    pub orbit_distance: f32,
    yaw_target: f32,
    pitch_target: f32,
    targets_initialized: bool,
    fwd: f32,
    back: f32,
    left: f32,
    right: f32,
    up: f32,
    down: f32,
    sprint_active: bool,
    precision_active: bool,
    sprint_mult: f32,
    precision_mult: f32,
    dragging: bool,
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
        }
    }

    pub fn process_keyboard_w(&mut self, pressed: bool) {
        self.fwd = if pressed { 1.0 } else { 0.0 };
    }

    pub fn process_keyboard_s(&mut self, pressed: bool) {
        self.back = if pressed { 1.0 } else { 0.0 };
    }

    pub fn process_mouse_button_right(&mut self, pressed: bool) {
        self.dragging = pressed;
    }

    pub fn process_mouse_delta(&mut self, camera: &mut Camera, delta: Vec2) {
        if self.dragging {
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
        }
    }

    pub fn process_scroll(&mut self, camera: &mut Camera, delta: f32) {
        match self.mode {
            CameraMode::FreeFly => {
                let fov_delta = delta * self.zoom_sensitivity;
                camera.fovy = (camera.fovy - fov_delta).clamp(0.1, 3.0);
            }
            CameraMode::Orbit => {
                self.orbit_distance = (self.orbit_distance - delta * 0.5).clamp(1.0, 50.0);
                self.update_orbit_position(camera);
            }
        }
    }

    pub fn toggle_mode(&mut self, camera: &mut Camera) {
        match self.mode {
            CameraMode::FreeFly => {
                self.mode = CameraMode::Orbit;
                let look_dir = Camera::dir(camera.yaw, camera.pitch);
                self.orbit_target = camera.position + look_dir * self.orbit_distance;
            }
            CameraMode::Orbit => {
                self.mode = CameraMode::FreeFly;
            }
        }
    }

    fn update_orbit_position(&mut self, camera: &mut Camera) {
        let dir = Camera::dir(camera.yaw, camera.pitch);
        camera.position = self.orbit_target - dir * self.orbit_distance;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: f32) {
        if !self.targets_initialized {
            self.yaw_target = camera.yaw;
            self.pitch_target = camera.pitch;
            self.targets_initialized = true;
        }

        let t = 1.0 - (-self.mouse_smooth * dt.max(1e-4)).exp();
        camera.yaw = camera.yaw + (self.yaw_target - camera.yaw) * t;
        camera.pitch = (camera.pitch + (self.pitch_target - camera.pitch) * t).clamp(-1.54, 1.54);

        let mut eff_speed = self.speed;
        if self.sprint_active {
            eff_speed *= self.sprint_mult;
        }
        if self.precision_active {
            eff_speed *= self.precision_mult;
        }

        match self.mode {
            CameraMode::FreeFly => {
                let dir = Camera::dir(camera.yaw, camera.pitch);
                let right = dir.cross(-Vec3::Y).normalize();
                let up = -Vec3::Y;

                let mut vel = Vec3::ZERO;
                vel += dir * (self.fwd - self.back);
                vel += right * (self.right - self.left);
                vel += up * (self.down - self.up);
                if vel.length_squared() > 0.0 {
                    camera.position += vel.normalize() * eff_speed * dt;
                }
            }
            CameraMode::Orbit => {
                self.update_orbit_position(camera);
            }
        }
    }
}

// ============================================================================
// Primitive Types (matching actual API)
// ============================================================================

/// Vertex structure for primitives
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

/// Generate a unit cube
pub fn cube() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let faces = [
        ([1.0, -1.0, -1.0], [1.0, 0.0, 0.0]),
        ([1.0, 1.0, -1.0], [1.0, 0.0, 0.0]),
        ([1.0, 1.0, 1.0], [1.0, 0.0, 0.0]),
        ([1.0, -1.0, 1.0], [1.0, 0.0, 0.0]),
        ([-1.0, -1.0, 1.0], [-1.0, 0.0, 0.0]),
        ([-1.0, 1.0, 1.0], [-1.0, 0.0, 0.0]),
        ([-1.0, 1.0, -1.0], [-1.0, 0.0, 0.0]),
        ([-1.0, -1.0, -1.0], [-1.0, 0.0, 0.0]),
        ([-1.0, 1.0, -1.0], [0.0, 1.0, 0.0]),
        ([1.0, 1.0, -1.0], [0.0, 1.0, 0.0]),
        ([1.0, 1.0, 1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 1.0, 1.0], [0.0, 1.0, 0.0]),
        ([-1.0, -1.0, 1.0], [0.0, -1.0, 0.0]),
        ([1.0, -1.0, 1.0], [0.0, -1.0, 0.0]),
        ([1.0, -1.0, -1.0], [0.0, -1.0, 0.0]),
        ([-1.0, -1.0, -1.0], [0.0, -1.0, 0.0]),
        ([-1.0, -1.0, 1.0], [0.0, 0.0, 1.0]),
        ([-1.0, 1.0, 1.0], [0.0, 0.0, 1.0]),
        ([1.0, 1.0, 1.0], [0.0, 0.0, 1.0]),
        ([1.0, -1.0, 1.0], [0.0, 0.0, 1.0]),
        ([1.0, -1.0, -1.0], [0.0, 0.0, -1.0]),
        ([1.0, 1.0, -1.0], [0.0, 0.0, -1.0]),
        ([-1.0, 1.0, -1.0], [0.0, 0.0, -1.0]),
        ([-1.0, -1.0, -1.0], [0.0, 0.0, -1.0]),
    ];
    for (idx, (p, n)) in faces.iter().enumerate() {
        let tangent = [1.0, 0.0, 0.0, 1.0];
        let corner = (idx % 4) as u32;
        let uv = match corner {
            0 => [0.0, 0.0],
            1 => [1.0, 0.0],
            2 => [1.0, 1.0],
            _ => [0.0, 1.0],
        };
        v.push(Vertex {
            position: *p,
            normal: *n,
            tangent,
            uv,
        });
        if idx % 4 == 3 {
            let base = idx as u32 - 3;
            i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    (v, i)
}

/// Generate a ground plane
pub fn plane() -> (Vec<Vertex>, Vec<u32>) {
    let n = [0.0, 1.0, 0.0];
    let t = [1.0, 0.0, 0.0, 1.0];
    let v = vec![
        Vertex {
            position: [-1.0, 0.0, -1.0],
            normal: n,
            tangent: t,
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [1.0, 0.0, -1.0],
            normal: n,
            tangent: t,
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [1.0, 0.0, 1.0],
            normal: n,
            tangent: t,
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 0.0, 1.0],
            normal: n,
            tangent: t,
            uv: [0.0, 1.0],
        },
    ];
    let i = vec![0, 1, 2, 0, 2, 3];
    (v, i)
}

/// Generate a UV sphere
pub fn sphere(stacks: u32, slices: u32, radius: f32) -> (Vec<Vertex>, Vec<u32>) {
    let stacks = stacks.max(3);
    let slices = slices.max(3);
    let mut verts: Vec<Vertex> = Vec::with_capacity(((stacks + 1) * (slices + 1)) as usize);
    let mut indices: Vec<u32> = Vec::with_capacity((stacks * slices * 6) as usize);

    for i in 0..=stacks {
        let v = i as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        for j in 0..=slices {
            let u = j as f32 / slices as f32;
            let theta = u * std::f32::consts::PI * 2.0;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let nx = sin_phi * cos_theta;
            let ny = cos_phi;
            let nz = sin_phi * sin_theta;
            let px = radius * nx;
            let py = radius * ny;
            let pz = radius * nz;
            let tx = -sin_theta;
            let ty = 0.0;
            let tz = cos_theta;
            let tangent = [tx, ty, tz, 1.0];
            let uv = [u, 1.0 - v];
            verts.push(Vertex {
                position: [px, py, pz],
                normal: [nx, ny, nz],
                tangent,
                uv,
            });
        }
    }

    let row = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * row + j;
            let b = a + 1;
            let c = (i + 1) * row + j;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    (verts, indices)
}

// ============================================================================
// Instancing Types (matching actual API)
// ============================================================================

/// Raw instance data for GPU
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn from_transform(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let model = Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self {
            model: model.to_cols_array_2d(),
        }
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        Self {
            model: matrix.to_cols_array_2d(),
        }
    }
}

/// High-level instance transform
#[derive(Clone, Debug)]
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Instance {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw::from_transform(self.position, self.rotation, self.scale)
    }
}

/// Instance batch for mesh
pub struct InstanceBatch {
    pub mesh_id: u64,
    pub instances: Vec<Instance>,
}

impl InstanceBatch {
    pub fn new(mesh_id: u64) -> Self {
        Self {
            mesh_id,
            instances: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }

    pub fn to_raw_data(&self) -> Vec<InstanceRaw> {
        self.instances.iter().map(|i| i.to_raw()).collect()
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

/// Instance manager
pub struct InstanceManager {
    batches: std::collections::HashMap<u64, InstanceBatch>,
    total_instances: usize,
    draw_calls_saved: usize,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            batches: std::collections::HashMap::new(),
            total_instances: 0,
            draw_calls_saved: 0,
        }
    }

    pub fn add_instance(&mut self, mesh_id: u64, instance: Instance) {
        let batch = self
            .batches
            .entry(mesh_id)
            .or_insert_with(|| InstanceBatch::new(mesh_id));
        batch.add_instance(instance);
        self.total_instances += 1;
    }

    pub fn add_instances(&mut self, mesh_id: u64, instances: Vec<Instance>) {
        let count = instances.len();
        let batch = self
            .batches
            .entry(mesh_id)
            .or_insert_with(|| InstanceBatch::new(mesh_id));
        for instance in instances {
            batch.add_instance(instance);
        }
        self.total_instances += count;
    }

    pub fn total_instances(&self) -> usize {
        self.total_instances
    }

    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    pub fn calculate_draw_call_savings(&mut self) {
        let without_instancing = self.total_instances;
        let with_instancing = self.batches.len();
        self.draw_calls_saved = without_instancing.saturating_sub(with_instancing);
    }

    pub fn draw_calls_saved(&self) -> usize {
        self.draw_calls_saved
    }

    pub fn draw_call_reduction_percent(&self) -> f32 {
        if self.total_instances == 0 {
            return 0.0;
        }
        (self.draw_calls_saved as f32 / self.total_instances as f32) * 100.0
    }

    pub fn clear(&mut self) {
        self.batches.clear();
        self.total_instances = 0;
        self.draw_calls_saved = 0;
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Instance pattern builder
pub struct InstancePatternBuilder {
    instances: Vec<Instance>,
}

impl InstancePatternBuilder {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn grid(mut self, rows: usize, cols: usize, spacing: f32) -> Self {
        for row in 0..rows {
            for col in 0..cols {
                let x = col as f32 * spacing;
                let z = row as f32 * spacing;
                self.instances.push(Instance::new(
                    Vec3::new(x, 0.0, z),
                    Quat::IDENTITY,
                    Vec3::ONE,
                ));
            }
        }
        self
    }

    pub fn circle(mut self, count: usize, radius: f32) -> Self {
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let rotation = Quat::from_rotation_y(angle + std::f32::consts::PI);
            self.instances
                .push(Instance::new(Vec3::new(x, 0.0, z), rotation, Vec3::ONE));
        }
        self
    }

    pub fn random_scatter(mut self, count: usize, bounds: f32) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        for _ in 0..count {
            let x = rng.random_range(-bounds..bounds);
            let z = rng.random_range(-bounds..bounds);
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let scale = rng.random_range(0.5..1.5);
            self.instances.push(Instance::new(
                Vec3::new(x, 0.0, z),
                Quat::from_rotation_y(angle),
                Vec3::splat(scale),
            ));
        }
        self
    }

    pub fn build(self) -> Vec<Instance> {
        self.instances
    }
}

impl Default for InstancePatternBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Overlay/Effects Types
// ============================================================================

/// Overlay parameters
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OverlayParams {
    pub fade: f32,
    pub letterbox: f32,
    pub _pad: [f32; 2],
}

impl OverlayParams {
    pub fn new(fade: f32, letterbox: f32) -> Self {
        Self {
            fade,
            letterbox,
            _pad: [0.0; 2],
        }
    }

    pub fn fade_to_black() -> Self {
        Self::new(1.0, 0.0)
    }

    pub fn cinematic() -> Self {
        Self::new(0.0, 0.12)
    }

    pub fn none() -> Self {
        Self::new(0.0, 0.0)
    }
}

// ============================================================================
// Camera Benchmarks
// ============================================================================

fn bench_camera_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("camera_operations");

    let camera = Camera::default();

    // View matrix generation
    group.bench_function("view_matrix", |b| b.iter(|| bb(camera.view_matrix())));

    // Projection matrix generation
    group.bench_function("proj_matrix", |b| b.iter(|| bb(camera.proj_matrix())));

    // Combined view-projection
    group.bench_function("view_projection", |b| b.iter(|| bb(camera.vp())));

    // Direction calculation
    group.bench_function("direction_calc", |b| {
        b.iter(|| bb(Camera::dir(black_box(0.5), black_box(0.3))))
    });

    // Direction calc - multiple angles
    let angles: Vec<(f32, f32)> = (0..16)
        .map(|i| {
            let yaw = (i as f32 / 16.0) * std::f32::consts::TAU;
            let pitch = ((i as f32 / 16.0) - 0.5) * std::f32::consts::PI * 0.9;
            (yaw, pitch)
        })
        .collect();

    group.bench_function("direction_batch_16", |b| {
        b.iter(|| {
            for &(yaw, pitch) in &angles {
                bb(Camera::dir(yaw, pitch));
            }
        })
    });

    group.finish();
}

fn bench_camera_controller(c: &mut Criterion) {
    let mut group = c.benchmark_group("camera_controller");

    // Controller creation
    group.bench_function("new", |b| {
        b.iter(|| bb(CameraController::new(black_box(5.0), black_box(0.01))))
    });

    // Keyboard processing
    let mut controller = CameraController::new(5.0, 0.01);
    group.bench_function("process_keyboard", |b| {
        b.iter(|| {
            controller.process_keyboard_w(black_box(true));
            controller.process_keyboard_s(black_box(false));
        })
    });

    // Mouse delta processing
    let mut controller = CameraController::new(5.0, 0.01);
    let mut camera = Camera::default();
    controller.process_mouse_button_right(true);

    group.bench_function("process_mouse_delta", |b| {
        b.iter(|| controller.process_mouse_delta(&mut camera, black_box(Vec2::new(10.0, 5.0))))
    });

    // Scroll/zoom
    let mut controller = CameraController::new(5.0, 0.01);
    let mut camera = Camera::default();

    group.bench_function("process_scroll_freefly", |b| {
        b.iter(|| controller.process_scroll(&mut camera, black_box(0.5)))
    });

    controller.toggle_mode(&mut camera);
    group.bench_function("process_scroll_orbit", |b| {
        b.iter(|| controller.process_scroll(&mut camera, black_box(0.5)))
    });

    // Mode toggle
    let mut controller = CameraController::new(5.0, 0.01);
    let mut camera = Camera::default();

    group.bench_function("toggle_mode", |b| {
        b.iter(|| controller.toggle_mode(&mut camera))
    });

    // Full update cycle
    let mut controller = CameraController::new(5.0, 0.01);
    let mut camera = Camera::default();
    controller.process_keyboard_w(true);

    group.bench_function("update_freefly", |b| {
        b.iter(|| controller.update_camera(&mut camera, black_box(0.016)))
    });

    controller.toggle_mode(&mut camera);
    group.bench_function("update_orbit", |b| {
        b.iter(|| controller.update_camera(&mut camera, black_box(0.016)))
    });

    group.finish();
}

// ============================================================================
// Primitive Benchmarks
// ============================================================================

fn bench_primitive_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("primitive_generation");

    // Cube
    group.bench_function("cube", |b| b.iter(|| bb(cube())));

    // Plane
    group.bench_function("plane", |b| b.iter(|| bb(plane())));

    // Sphere - various resolutions
    for &(stacks, slices) in &[(8, 8), (16, 16), (32, 32), (64, 64)] {
        group.bench_with_input(
            BenchmarkId::new("sphere", format!("{}x{}", stacks, slices)),
            &(stacks, slices),
            |b, &(stacks, slices)| b.iter(|| bb(sphere(stacks, slices, 1.0))),
        );
    }

    // Measure vertex/index counts
    let (cube_v, _cube_i) = cube();
    let (_plane_v, _plane_i) = plane();
    let (sphere_v, _sphere_i) = sphere(16, 16, 1.0);

    group.throughput(Throughput::Elements(cube_v.len() as u64));
    group.bench_function("cube_per_vertex", |b| b.iter(|| bb(cube())));

    group.throughput(Throughput::Elements(sphere_v.len() as u64));
    group.bench_function("sphere_16x16_per_vertex", |b| {
        b.iter(|| bb(sphere(16, 16, 1.0)))
    });

    group.finish();
}

// ============================================================================
// Instance Benchmarks
// ============================================================================

fn bench_instance_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("instance_transforms");

    // Instance creation
    group.bench_function("new_identity", |b| b.iter(|| bb(Instance::identity())));

    group.bench_function("new_positioned", |b| {
        b.iter(|| {
            bb(Instance::new(
                black_box(Vec3::new(1.0, 2.0, 3.0)),
                black_box(Quat::from_rotation_y(0.5)),
                black_box(Vec3::splat(1.5)),
            ))
        })
    });

    // to_raw conversion
    let instance = Instance::new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_rotation_y(0.5),
        Vec3::ONE,
    );
    group.bench_function("to_raw", |b| b.iter(|| bb(instance.to_raw())));

    // InstanceRaw creation methods
    group.bench_function("raw_from_transform", |b| {
        b.iter(|| {
            bb(InstanceRaw::from_transform(
                black_box(Vec3::new(1.0, 2.0, 3.0)),
                black_box(Quat::from_rotation_y(0.5)),
                black_box(Vec3::ONE),
            ))
        })
    });

    let matrix = Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::ZERO);
    group.bench_function("raw_from_matrix", |b| {
        b.iter(|| bb(InstanceRaw::from_matrix(black_box(matrix))))
    });

    group.finish();
}

fn bench_instance_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("instance_batching");

    // Batch creation
    group.bench_function("batch_new", |b| {
        b.iter(|| bb(InstanceBatch::new(black_box(42))))
    });

    // Add instances to batch
    for count in [10, 100, 1000, 5000] {
        let instances: Vec<Instance> = (0..count)
            .map(|i| Instance::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE))
            .collect();

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("add_instances", count),
            &instances,
            |b, instances| {
                b.iter(|| {
                    let mut batch = InstanceBatch::new(1);
                    for inst in instances.iter().cloned() {
                        batch.add_instance(bb(inst));
                    }
                    bb(batch.instance_count())
                })
            },
        );
    }

    // Convert batch to raw data (GPU upload prep)
    for count in [100, 1000, 5000] {
        let mut batch = InstanceBatch::new(1);
        for i in 0..count {
            batch.add_instance(Instance::new(
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::ONE,
            ));
        }

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("to_raw_data", count),
            &batch,
            |b, batch| b.iter(|| bb(batch.to_raw_data())),
        );
    }

    group.finish();
}

fn bench_instance_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("instance_manager");

    // Manager creation
    group.bench_function("new", |b| b.iter(|| bb(InstanceManager::new())));

    // Add instances - different mesh distributions
    for &(mesh_count, instances_per_mesh) in &[(1, 1000), (10, 100), (100, 10), (1000, 1)] {
        let total = mesh_count * instances_per_mesh;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new(
                "add",
                format!("{}meshes_x_{}", mesh_count, instances_per_mesh),
            ),
            &(mesh_count, instances_per_mesh),
            |b, &(mesh_count, instances_per_mesh)| {
                b.iter(|| {
                    let mut manager = InstanceManager::new();
                    for mesh in 0..mesh_count {
                        for i in 0..instances_per_mesh {
                            manager.add_instance(
                                mesh as u64,
                                Instance::new(
                                    Vec3::new(i as f32, 0.0, 0.0),
                                    Quat::IDENTITY,
                                    Vec3::ONE,
                                ),
                            );
                        }
                    }
                    bb(manager.total_instances())
                })
            },
        );
    }

    // Calculate draw call savings
    let mut manager = InstanceManager::new();
    for mesh in 0..10 {
        for i in 0..100 {
            manager.add_instance(
                mesh,
                Instance::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
            );
        }
    }

    group.bench_function("calculate_savings_1000_instances_10_meshes", |b| {
        b.iter(|| {
            manager.calculate_draw_call_savings();
            bb(manager.draw_calls_saved())
        })
    });

    // Clear operations
    group.bench_function("clear_1000_instances", |b| {
        b.iter_with_setup(
            || {
                let mut m = InstanceManager::new();
                for mesh in 0..10 {
                    for i in 0..100 {
                        m.add_instance(
                            mesh,
                            Instance::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
                        );
                    }
                }
                m
            },
            |mut m| {
                m.clear();
                bb(m.total_instances())
            },
        )
    });

    group.finish();
}

fn bench_instance_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("instance_patterns");

    // Grid pattern
    for &(rows, cols) in &[(10, 10), (32, 32), (100, 100)] {
        let total = rows * cols;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("grid", format!("{}x{}", rows, cols)),
            &(rows, cols),
            |b, &(rows, cols)| {
                b.iter(|| bb(InstancePatternBuilder::new().grid(rows, cols, 2.0).build()))
            },
        );
    }

    // Circle pattern
    for &count in &[8, 32, 128, 512] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("circle", count), &count, |b, &count| {
            b.iter(|| bb(InstancePatternBuilder::new().circle(count, 10.0).build()))
        });
    }

    // Random scatter (with RNG overhead)
    for &count in &[100, 1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("random_scatter", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    bb(InstancePatternBuilder::new()
                        .random_scatter(count, 50.0)
                        .build())
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Overlay Benchmarks
// ============================================================================

fn bench_overlay_params(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlay_params");

    // Parameter creation
    group.bench_function("new", |b| {
        b.iter(|| bb(OverlayParams::new(black_box(0.5), black_box(0.1))))
    });

    group.bench_function("fade_to_black", |b| {
        b.iter(|| bb(OverlayParams::fade_to_black()))
    });

    group.bench_function("cinematic", |b| b.iter(|| bb(OverlayParams::cinematic())));

    group.bench_function("none", |b| b.iter(|| bb(OverlayParams::none())));

    // Parameter interpolation (common for transitions)
    let start = OverlayParams::none();
    let end = OverlayParams::fade_to_black();

    group.bench_function("interpolate_fade", |b| {
        b.iter(|| {
            let t = black_box(0.5f32);
            bb(OverlayParams::new(
                start.fade + (end.fade - start.fade) * t,
                start.letterbox + (end.letterbox - start.letterbox) * t,
            ))
        })
    });

    group.finish();
}

// ============================================================================
// Combined Scenario Benchmarks
// ============================================================================

fn bench_combined_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_scenarios");

    // Typical frame: camera update + instance transforms
    group.bench_function("typical_frame_100_instances", |b| {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = Camera::default();
        controller.process_keyboard_w(true);
        let instances: Vec<Instance> = (0..100)
            .map(|i| Instance::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE))
            .collect();

        b.iter(|| {
            controller.update_camera(&mut camera, 0.016);
            let vp = camera.vp();
            let raw: Vec<InstanceRaw> = instances.iter().map(|i| i.to_raw()).collect();
            bb((vp, raw.len()))
        })
    });

    // Spawn wave: generate primitives + instance pattern
    group.bench_function("spawn_wave_32x32_grid", |b| {
        b.iter(|| {
            let (cube_verts, cube_indices) = cube();
            let instances = InstancePatternBuilder::new().grid(32, 32, 2.0).build();
            bb((cube_verts.len(), cube_indices.len(), instances.len()))
        })
    });

    // Scene setup: multiple primitive types
    group.bench_function("scene_setup_mixed", |b| {
        b.iter(|| {
            let (cube_v, cube_i) = cube();
            let (plane_v, plane_i) = plane();
            let (sphere_v, sphere_i) = sphere(16, 16, 1.0);

            let mut manager = InstanceManager::new();
            manager.add_instances(1, InstancePatternBuilder::new().grid(10, 10, 2.0).build());
            manager.add_instances(2, InstancePatternBuilder::new().circle(16, 20.0).build());
            manager.add_instance(3, Instance::identity()); // ground plane

            manager.calculate_draw_call_savings();

            bb((
                cube_v.len() + plane_v.len() + sphere_v.len(),
                cube_i.len() + plane_i.len() + sphere_i.len(),
                manager.total_instances(),
                manager.draw_calls_saved(),
            ))
        })
    });

    // Large forest: 10,000 tree instances with random scatter
    group.bench_function("large_forest_10k_trees", |b| {
        b.iter(|| {
            let instances = InstancePatternBuilder::new()
                .random_scatter(10000, 500.0)
                .build();

            let mut batch = InstanceBatch::new(1);
            for inst in instances {
                batch.add_instance(inst);
            }

            let raw = batch.to_raw_data();
            bb(raw.len())
        })
    });

    // Cinematic camera: orbit mode with smooth transitions
    group.bench_function("cinematic_camera_sequence", |b| {
        let mut controller = CameraController::new(5.0, 0.01);
        let mut camera = Camera::default();
        controller.toggle_mode(&mut camera); // Switch to orbit
        controller.process_mouse_button_right(true);

        b.iter(|| {
            // Simulate 60 frames of cinematic camera movement
            for i in 0..60 {
                let t = i as f32 / 60.0;
                controller.process_mouse_delta(&mut camera, Vec2::new(t * 2.0, t));
                controller.update_camera(&mut camera, 0.016);
                bb(camera.vp());
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_camera_operations,
    bench_camera_controller,
    bench_primitive_generation,
    bench_instance_transforms,
    bench_instance_batching,
    bench_instance_manager,
    bench_instance_patterns,
    bench_overlay_params,
    bench_combined_scenarios,
);

criterion_main!(benches);
