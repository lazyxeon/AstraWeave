use criterion::{criterion_group, criterion_main, Criterion};
use glam::{Mat4, Quat, Vec2, Vec3};
use std::hint::black_box;
use winit::keyboard::KeyCode;

// Import from the library
use aw_editor_lib::gizmo::{
    picking::{GizmoHandle, GizmoPicker, Ray},
    rendering::{GizmoRenderParams, GizmoRenderer},
    rotate::RotateGizmo,
    scale::ScaleGizmo,
    scene_viewport::{CameraController, SceneViewport, Transform},
    state::{AxisConstraint, GizmoMode, GizmoState},
    translate::TranslateGizmo,
};

// ==================== State Machine Benchmarks ====================

fn bench_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_transitions");

    group.bench_function("start_translate", |b| {
        let mut state = GizmoState::new();
        b.iter(|| {
            state.start_translate();
            black_box(&state);
        });
    });

    group.bench_function("start_rotate", |b| {
        let mut state = GizmoState::new();
        b.iter(|| {
            state.start_rotate();
            black_box(&state);
        });
    });

    group.bench_function("start_scale", |b| {
        let mut state = GizmoState::new();
        b.iter(|| {
            state.start_scale(false);
            black_box(&state);
        });
    });

    group.bench_function("handle_key_g", |b| {
        let mut state = GizmoState::new();
        b.iter(|| {
            state.handle_key(black_box(KeyCode::KeyG));
        });
    });

    group.bench_function("handle_key_x", |b| {
        let mut state = GizmoState::new();
        state.start_translate();
        b.iter(|| {
            state.handle_key(black_box(KeyCode::KeyX));
        });
    });

    group.bench_function("update_mouse", |b| {
        let mut state = GizmoState::new();
        b.iter(|| {
            state.update_mouse(black_box(Vec2::new(0.5, 0.5)));
        });
    });

    group.finish();
}

// ==================== Translation Math Benchmarks ====================

fn bench_translation_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("translation_math");

    let mouse_delta = Vec2::new(0.1, 0.05);
    let camera_distance = 10.0;
    let rotation = Quat::IDENTITY;

    group.bench_function("translate_none_constraint", |b| {
        b.iter(|| {
            TranslateGizmo::calculate_translation(
                black_box(mouse_delta),
                black_box(AxisConstraint::None),
                black_box(camera_distance),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.bench_function("translate_x_constraint", |b| {
        b.iter(|| {
            TranslateGizmo::calculate_translation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(camera_distance),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.bench_function("translate_xy_constraint", |b| {
        b.iter(|| {
            TranslateGizmo::calculate_translation(
                black_box(mouse_delta),
                black_box(AxisConstraint::XY),
                black_box(camera_distance),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.bench_function("translate_local_space", |b| {
        let rotated = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        b.iter(|| {
            TranslateGizmo::calculate_translation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(camera_distance),
                black_box(rotated),
                black_box(true),
            )
        });
    });

    group.bench_function("translate_numeric", |b| {
        b.iter(|| {
            TranslateGizmo::calculate_translation_numeric(
                black_box(5.0),
                black_box(AxisConstraint::X),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.finish();
}

// ==================== Rotation Math Benchmarks ====================

fn bench_rotation_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("rotation_math");

    let mouse_delta = Vec2::new(0.1, 0.05);
    let rotation = Quat::IDENTITY;

    group.bench_function("rotate_x_axis", |b| {
        b.iter(|| {
            RotateGizmo::calculate_rotation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(1.0),
                black_box(rotation),
                black_box(false),
                black_box(false),
            )
        });
    });

    group.bench_function("rotate_with_snap", |b| {
        b.iter(|| {
            RotateGizmo::calculate_rotation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(1.0),
                black_box(rotation),
                black_box(true),
                black_box(false),
            )
        });
    });

    group.bench_function("rotate_local_space", |b| {
        let rotated = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        b.iter(|| {
            RotateGizmo::calculate_rotation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(1.0),
                black_box(rotated),
                black_box(false),
                black_box(true),
            )
        });
    });

    group.bench_function("rotate_numeric", |b| {
        b.iter(|| {
            RotateGizmo::calculate_rotation_numeric(
                black_box(45.0),
                black_box(AxisConstraint::X),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.finish();
}

// ==================== Scale Math Benchmarks ====================

fn bench_scale_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_math");

    let mouse_delta = Vec2::new(0.1, 0.05);
    let rotation = Quat::IDENTITY;

    group.bench_function("scale_uniform", |b| {
        b.iter(|| {
            ScaleGizmo::calculate_scale(
                black_box(mouse_delta),
                black_box(AxisConstraint::None),
                black_box(1.0),
                black_box(rotation),
                black_box(true),
                black_box(false),
            )
        });
    });

    group.bench_function("scale_x_axis", |b| {
        b.iter(|| {
            ScaleGizmo::calculate_scale(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(1.0),
                black_box(rotation),
                black_box(false),
                black_box(false),
            )
        });
    });

    group.bench_function("scale_local_space", |b| {
        let rotated = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        b.iter(|| {
            ScaleGizmo::calculate_scale(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(1.0),
                black_box(rotated),
                black_box(false),
                black_box(true),
            )
        });
    });

    group.bench_function("scale_numeric", |b| {
        b.iter(|| {
            ScaleGizmo::calculate_scale_numeric(
                black_box(2.0),
                black_box(AxisConstraint::X),
                black_box(rotation),
                black_box(false),
            )
        });
    });

    group.finish();
}

// ==================== Rendering Benchmarks ====================

fn bench_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering");

    let params = GizmoRenderParams {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        camera_pos: Vec3::new(5.0, 5.0, 5.0),
        view_proj: Mat4::IDENTITY,
        mode: GizmoMode::Translate {
            constraint: AxisConstraint::None,
        },
        constraint: AxisConstraint::None,
        hovered_axis: None,
    };

    group.bench_function("generate_arrow", |b| {
        b.iter(|| GizmoRenderer::generate_arrow(black_box(Vec3::X), black_box(1.0)));
    });

    group.bench_function("generate_circle", |b| {
        b.iter(|| {
            GizmoRenderer::generate_circle(black_box(Vec3::X), black_box(1.0), black_box(32))
        });
    });

    group.bench_function("generate_cube", |b| {
        b.iter(|| {
            GizmoRenderer::generate_scale_cube(black_box(Vec3::X), black_box(1.0), black_box(0.1))
        });
    });

    group.bench_function("render_translation", |b| {
        b.iter(|| GizmoRenderer::render_translation(black_box(&params)));
    });

    group.bench_function("render_rotation", |b| {
        let rotate_params = GizmoRenderParams {
            mode: GizmoMode::Rotate {
                constraint: AxisConstraint::None,
            },
            ..params
        };
        b.iter(|| GizmoRenderer::render_rotation(black_box(&rotate_params)));
    });

    group.bench_function("render_scale", |b| {
        let scale_params = GizmoRenderParams {
            mode: GizmoMode::Scale {
                constraint: AxisConstraint::None,
                uniform: false,
            },
            ..params
        };
        b.iter(|| GizmoRenderer::render_scale(black_box(&scale_params)));
    });

    group.finish();
}

// ==================== Picking Benchmarks ====================

fn bench_picking(c: &mut Criterion) {
    let mut group = c.benchmark_group("picking");

    let picker = GizmoPicker::default();
    let ray = Ray::from_screen(Vec2::new(0.5, 0.5), Mat4::IDENTITY);
    let gizmo_pos = Vec3::ZERO;

    group.bench_function("pick_translate_handle", |b| {
        b.iter(|| picker.pick_translate_handle(black_box(&ray), black_box(gizmo_pos)));
    });

    group.bench_function("pick_rotate_handle", |b| {
        b.iter(|| picker.pick_rotate_handle(black_box(&ray), black_box(gizmo_pos)));
    });

    group.bench_function("pick_scale_handle", |b| {
        b.iter(|| picker.pick_scale_handle(black_box(&ray), black_box(gizmo_pos)));
    });

    group.bench_function("ray_from_screen", |b| {
        let inv_vp = Mat4::IDENTITY;
        b.iter(|| Ray::from_screen(black_box(Vec2::new(0.5, 0.5)), black_box(inv_vp)));
    });

    group.finish();
}

// ==================== Camera Benchmarks ====================

fn bench_camera(c: &mut Criterion) {
    let mut group = c.benchmark_group("camera");

    group.bench_function("orbit", |b| {
        let mut camera = CameraController::default();
        b.iter(|| {
            camera.orbit(black_box(Vec2::new(0.01, 0.01)), black_box(1.0));
        });
    });

    group.bench_function("pan", |b| {
        let mut camera = CameraController::default();
        b.iter(|| {
            camera.pan(black_box(Vec2::new(0.01, 0.01)), black_box(1.0));
        });
    });

    group.bench_function("zoom", |b| {
        let mut camera = CameraController::default();
        b.iter(|| {
            camera.zoom(black_box(0.1), black_box(1.0));
        });
    });

    group.bench_function("view_matrix", |b| {
        let camera = CameraController::default();
        b.iter(|| black_box(camera.view_matrix()));
    });

    group.bench_function("projection_matrix", |b| {
        let camera = CameraController::default();
        b.iter(|| black_box(camera.projection_matrix()));
    });

    group.finish();
}

// ==================== Scene Viewport Benchmarks ====================

fn bench_viewport(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewport");

    group.bench_function("handle_key", |b| {
        let mut viewport = SceneViewport::new();
        b.iter(|| {
            viewport.handle_key(black_box(KeyCode::KeyG));
        });
    });

    group.bench_function("update_mouse", |b| {
        let mut viewport = SceneViewport::new();
        b.iter(|| {
            viewport.update_mouse(black_box(Vec2::new(0.5, 0.5)));
        });
    });

    group.bench_function("handle_mouse_down", |b| {
        let mut viewport = SceneViewport::new();
        b.iter(|| {
            viewport.handle_mouse_down();
        });
    });

    group.bench_function("handle_mouse_up", |b| {
        let mut viewport = SceneViewport::new();
        b.iter(|| {
            viewport.handle_mouse_up();
        });
    });

    group.finish();
}

// ==================== Integrated Workflow Benchmarks ====================

fn bench_full_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_workflow");

    // Benchmark complete translate workflow
    group.bench_function("translate_workflow", |b| {
        let mut state = GizmoState::new();
        let mouse_delta = Vec2::new(0.1, 0.05);
        let camera_distance = 10.0;
        let rotation = Quat::IDENTITY;

        b.iter(|| {
            // 1. Start translate
            state.start_translate();
            // 2. Add X constraint
            state.handle_key(KeyCode::KeyX);
            // 3. Calculate translation
            let translation = TranslateGizmo::calculate_translation(
                black_box(mouse_delta),
                black_box(AxisConstraint::X),
                black_box(camera_distance),
                black_box(rotation),
                black_box(false),
            );
            black_box(translation);
        });
    });

    // Benchmark complete rotate workflow
    group.bench_function("rotate_workflow", |b| {
        let mut state = GizmoState::new();
        let mouse_delta = Vec2::new(0.1, 0.05);
        let rotation = Quat::IDENTITY;

        b.iter(|| {
            // 1. Start rotate
            state.start_rotate();
            // 2. Add Y constraint
            state.handle_key(KeyCode::KeyY);
            // 3. Calculate rotation
            let quat = RotateGizmo::calculate_rotation(
                black_box(mouse_delta),
                black_box(AxisConstraint::Y),
                black_box(1.0),
                black_box(rotation),
                black_box(false),
                black_box(false),
            );
            black_box(quat);
        });
    });

    // Benchmark complete scale workflow
    group.bench_function("scale_workflow", |b| {
        let mut state = GizmoState::new();
        let mouse_delta = Vec2::new(0.1, 0.05);
        let rotation = Quat::IDENTITY;

        b.iter(|| {
            // 1. Start scale
            state.start_scale(false);
            // 2. Add Z constraint
            state.handle_key(KeyCode::KeyZ);
            // 3. Calculate scale
            let scale = ScaleGizmo::calculate_scale(
                black_box(mouse_delta),
                black_box(AxisConstraint::Z),
                black_box(1.0),
                black_box(rotation),
                black_box(false),
                black_box(false),
            );
            black_box(scale);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_state_transitions,
    bench_translation_math,
    bench_rotation_math,
    bench_scale_math,
    bench_rendering,
    bench_picking,
    bench_camera,
    bench_viewport,
    bench_full_workflow,
);
criterion_main!(benches);
