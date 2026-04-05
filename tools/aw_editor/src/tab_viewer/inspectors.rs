//! Component inspector UI functions.
//!
//! These are the 7 specialized inspector widgets for Light, Collider, RigidBody,
//! Audio, Camera, Script, and Particle components.

use super::PanelEvent;
use egui_dock::egui;

pub(super) fn show_light_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let light_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("point");
    let mut type_idx = match light_type {
        "directional" => 0,
        "point" => 1,
        "spot" => 2,
        _ => 1,
    };
    ui.horizontal(|ui| {
        ui.label("Type:");
        let changed = egui::ComboBox::from_id_salt(format!("light_type_{}", entity_id))
            .selected_text(match type_idx {
                0 => "Directional",
                1 => "Point",
                2 => "Spot",
                _ => "Point",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut type_idx, 0, "Directional")
                    .changed()
                    | ui.selectable_value(&mut type_idx, 1, "Point").changed()
                    | ui.selectable_value(&mut type_idx, 2, "Spot").changed()
            })
            .inner
            .unwrap_or(false);
        if changed {
            let type_str = match type_idx {
                0 => "directional",
                2 => "spot",
                _ => "point",
            };
            let mut new_data = data.clone();
            new_data["type"] = serde_json::json!(type_str);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Light".to_string(),
                data: new_data,
            });
        }
    });

    let mut intensity = data
        .get("intensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Intensity:");
        if ui
            .add(egui::Slider::new(&mut intensity, 0.0..=100.0).logarithmic(true))
            .changed()
        {
            let mut new_data = data.clone();
            new_data["intensity"] = serde_json::json!(intensity);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Light".to_string(),
                data: new_data,
            });
        }
    });

    let r = data.get("color_r").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let g = data.get("color_g").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let b = data.get("color_b").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut color = [r, g, b];
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(&mut color).changed() {
            let mut new_data = data.clone();
            new_data["color_r"] = serde_json::json!(color[0]);
            new_data["color_g"] = serde_json::json!(color[1]);
            new_data["color_b"] = serde_json::json!(color[2]);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Light".to_string(),
                data: new_data,
            });
        }
    });

    if light_type == "point" || light_type == "spot" {
        let mut range = data.get("range").and_then(|v| v.as_f64()).unwrap_or(10.0) as f32;
        ui.horizontal(|ui| {
            ui.label("Range:");
            if ui
                .add(
                    egui::DragValue::new(&mut range)
                        .speed(0.1)
                        .range(0.1..=1000.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["range"] = serde_json::json!(range);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Light".to_string(),
                    data: new_data,
                });
            }
        });
    }
    if light_type == "spot" {
        let mut angle = data
            .get("spot_angle")
            .and_then(|v| v.as_f64())
            .unwrap_or(45.0) as f32;
        ui.horizontal(|ui| {
            ui.label("Spot Angle:");
            if ui
                .add(egui::Slider::new(&mut angle, 1.0..=179.0).suffix("°"))
                .changed()
            {
                let mut new_data = data.clone();
                new_data["spot_angle"] = serde_json::json!(angle);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Light".to_string(),
                    data: new_data,
                });
            }
        });
    }

    let mut cast_shadows = data
        .get("cast_shadows")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    ui.horizontal(|ui| {
        ui.label("Cast Shadows:");
        if ui.checkbox(&mut cast_shadows, "").changed() {
            let mut new_data = data.clone();
            new_data["cast_shadows"] = serde_json::json!(cast_shadows);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Light".to_string(),
                data: new_data,
            });
        }
    });
}

pub(super) fn show_collider_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let shape = data.get("shape").and_then(|v| v.as_str()).unwrap_or("box");
    let mut shape_idx = match shape {
        "box" => 0,
        "sphere" => 1,
        "capsule" => 2,
        "mesh" => 3,
        _ => 0,
    };
    ui.horizontal(|ui| {
        ui.label("Shape:");
        let changed = egui::ComboBox::from_id_salt(format!("collider_shape_{}", entity_id))
            .selected_text(match shape_idx {
                0 => "Box",
                1 => "Sphere",
                2 => "Capsule",
                3 => "Mesh",
                _ => "Box",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut shape_idx, 0, "Box").changed()
                    | ui.selectable_value(&mut shape_idx, 1, "Sphere").changed()
                    | ui.selectable_value(&mut shape_idx, 2, "Capsule").changed()
                    | ui.selectable_value(&mut shape_idx, 3, "Mesh").changed()
            })
            .inner
            .unwrap_or(false);
        if changed {
            let shape_str = match shape_idx {
                1 => "sphere",
                2 => "capsule",
                3 => "mesh",
                _ => "box",
            };
            let mut new_data = data.clone();
            new_data["shape"] = serde_json::json!(shape_str);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Collider".to_string(),
                data: new_data,
            });
        }
    });

    match shape {
        "box" => {
            let size = data
                .get("size")
                .and_then(|v| v.as_array())
                .map(|a| {
                    [
                        a.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        a.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        a.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    ]
                })
                .unwrap_or([1.0, 1.0, 1.0]);
            let mut sx = size[0];
            let mut sy = size[1];
            let mut sz = size[2];
            ui.horizontal(|ui| {
                ui.label("Size:");
                let c = ui
                    .add(
                        egui::DragValue::new(&mut sx)
                            .prefix("X:")
                            .speed(0.1)
                            .range(0.01..=100.0),
                    )
                    .changed()
                    | ui.add(
                        egui::DragValue::new(&mut sy)
                            .prefix("Y:")
                            .speed(0.1)
                            .range(0.01..=100.0),
                    )
                    .changed()
                    | ui.add(
                        egui::DragValue::new(&mut sz)
                            .prefix("Z:")
                            .speed(0.1)
                            .range(0.01..=100.0),
                    )
                    .changed();
                if c {
                    let mut new_data = data.clone();
                    new_data["size"] = serde_json::json!([sx, sy, sz]);
                    events.push(PanelEvent::ComponentDataChanged {
                        entity_id,
                        component_type: "Collider".to_string(),
                        data: new_data,
                    });
                }
            });
        }
        "sphere" => {
            let mut radius = data.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui
                    .add(
                        egui::DragValue::new(&mut radius)
                            .speed(0.05)
                            .range(0.01..=100.0),
                    )
                    .changed()
                {
                    let mut new_data = data.clone();
                    new_data["radius"] = serde_json::json!(radius);
                    events.push(PanelEvent::ComponentDataChanged {
                        entity_id,
                        component_type: "Collider".to_string(),
                        data: new_data,
                    });
                }
            });
        }
        "capsule" => {
            let mut radius = data.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
            let mut half_height = data
                .get("half_height")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui
                    .add(
                        egui::DragValue::new(&mut radius)
                            .speed(0.05)
                            .range(0.01..=100.0),
                    )
                    .changed()
                {
                    let mut new_data = data.clone();
                    new_data["radius"] = serde_json::json!(radius);
                    events.push(PanelEvent::ComponentDataChanged {
                        entity_id,
                        component_type: "Collider".to_string(),
                        data: new_data,
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Half Height:");
                if ui
                    .add(
                        egui::DragValue::new(&mut half_height)
                            .speed(0.05)
                            .range(0.01..=100.0),
                    )
                    .changed()
                {
                    let mut new_data = data.clone();
                    new_data["half_height"] = serde_json::json!(half_height);
                    events.push(PanelEvent::ComponentDataChanged {
                        entity_id,
                        component_type: "Collider".to_string(),
                        data: new_data,
                    });
                }
            });
        }
        "mesh" => {
            ui.label("Uses entity mesh for collision");
        }
        _ => {}
    }

    let mut is_trigger = data
        .get("is_trigger")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label("Is Trigger:");
        if ui.checkbox(&mut is_trigger, "").changed() {
            let mut new_data = data.clone();
            new_data["is_trigger"] = serde_json::json!(is_trigger);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Collider".to_string(),
                data: new_data,
            });
        }
    });
}

pub(super) fn show_rigidbody_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let body_type = data
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("dynamic");
    let mut type_idx = match body_type {
        "static" => 0,
        "kinematic" => 1,
        "dynamic" => 2,
        _ => 2,
    };
    ui.horizontal(|ui| {
        ui.label("Body Type:");
        let changed = egui::ComboBox::from_id_salt(format!("rb_type_{}", entity_id))
            .selected_text(match type_idx {
                0 => "Static",
                1 => "Kinematic",
                2 => "Dynamic",
                _ => "Dynamic",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut type_idx, 0, "Static").changed()
                    | ui.selectable_value(&mut type_idx, 1, "Kinematic").changed()
                    | ui.selectable_value(&mut type_idx, 2, "Dynamic").changed()
            })
            .inner
            .unwrap_or(false);
        if changed {
            let type_str = match type_idx {
                0 => "static",
                1 => "kinematic",
                _ => "dynamic",
            };
            let mut new_data = data.clone();
            new_data["type"] = serde_json::json!(type_str);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "RigidBody".to_string(),
                data: new_data,
            });
        }
    });

    if body_type == "dynamic" {
        let mut mass = data.get("mass").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        ui.horizontal(|ui| {
            ui.label("Mass:");
            if ui
                .add(
                    egui::DragValue::new(&mut mass)
                        .speed(0.1)
                        .range(0.001..=10000.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["mass"] = serde_json::json!(mass);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "RigidBody".to_string(),
                    data: new_data,
                });
            }
        });

        let mut drag = data.get("drag").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        ui.horizontal(|ui| {
            ui.label("Drag:");
            if ui
                .add(
                    egui::DragValue::new(&mut drag)
                        .speed(0.01)
                        .range(0.0..=100.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["drag"] = serde_json::json!(drag);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "RigidBody".to_string(),
                    data: new_data,
                });
            }
        });

        let mut angular_drag = data
            .get("angular_drag")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05) as f32;
        ui.horizontal(|ui| {
            ui.label("Angular Drag:");
            if ui
                .add(
                    egui::DragValue::new(&mut angular_drag)
                        .speed(0.01)
                        .range(0.0..=100.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["angular_drag"] = serde_json::json!(angular_drag);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "RigidBody".to_string(),
                    data: new_data,
                });
            }
        });

        let mut use_gravity = data
            .get("use_gravity")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        ui.horizontal(|ui| {
            ui.label("Use Gravity:");
            if ui.checkbox(&mut use_gravity, "").changed() {
                let mut new_data = data.clone();
                new_data["use_gravity"] = serde_json::json!(use_gravity);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "RigidBody".to_string(),
                    data: new_data,
                });
            }
        });
    }
}

pub(super) fn show_audio_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let clip = data
        .get("clip")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut clip_buf = clip.clone();
    ui.horizontal(|ui| {
        ui.label("Clip:");
        if ui.text_edit_singleline(&mut clip_buf).lost_focus() && clip_buf != clip {
            let mut new_data = data.clone();
            new_data["clip"] = serde_json::json!(clip_buf);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Audio".to_string(),
                data: new_data,
            });
        }
        if ui
            .small_button("\u{1f4c2}")
            .on_hover_text("Browse for audio file")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", &["wav", "ogg", "mp3", "flac"])
                .pick_file()
            {
                let mut new_data = data.clone();
                new_data["clip"] = serde_json::json!(path.to_string_lossy().to_string());
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Audio".to_string(),
                    data: new_data,
                });
            }
        }
    });

    let mut volume = data.get("volume").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Volume:");
        if ui.add(egui::Slider::new(&mut volume, 0.0..=2.0)).changed() {
            let mut new_data = data.clone();
            new_data["volume"] = serde_json::json!(volume);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Audio".to_string(),
                data: new_data,
            });
        }
    });

    let mut spatial = data
        .get("spatial")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    ui.horizontal(|ui| {
        ui.label("Spatial:");
        if ui.checkbox(&mut spatial, "3D audio").changed() {
            let mut new_data = data.clone();
            new_data["spatial"] = serde_json::json!(spatial);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Audio".to_string(),
                data: new_data,
            });
        }
    });

    if spatial {
        let mut min_dist = data
            .get("min_distance")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let mut max_dist = data
            .get("max_distance")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as f32;
        ui.horizontal(|ui| {
            ui.label("Min Distance:");
            if ui
                .add(
                    egui::DragValue::new(&mut min_dist)
                        .speed(0.1)
                        .range(0.1..=100.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["min_distance"] = serde_json::json!(min_dist);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Audio".to_string(),
                    data: new_data,
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label("Max Distance:");
            if ui
                .add(
                    egui::DragValue::new(&mut max_dist)
                        .speed(0.5)
                        .range(1.0..=1000.0),
                )
                .changed()
            {
                let mut new_data = data.clone();
                new_data["max_distance"] = serde_json::json!(max_dist);
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Audio".to_string(),
                    data: new_data,
                });
            }
        });
    }

    let mut looping = data
        .get("looping")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label("Loop:");
        if ui.checkbox(&mut looping, "").changed() {
            let mut new_data = data.clone();
            new_data["looping"] = serde_json::json!(looping);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Audio".to_string(),
                data: new_data,
            });
        }
    });

    let mut play_on_start = data
        .get("play_on_start")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label("Play on Start:");
        if ui.checkbox(&mut play_on_start, "").changed() {
            let mut new_data = data.clone();
            new_data["play_on_start"] = serde_json::json!(play_on_start);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Audio".to_string(),
                data: new_data,
            });
        }
    });
}

pub(super) fn show_camera_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let mut fov = data.get("fov").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
    ui.horizontal(|ui| {
        ui.label("FOV:");
        if ui
            .add(egui::Slider::new(&mut fov, 10.0..=120.0).suffix("\u{00b0}"))
            .changed()
        {
            let mut new_data = data.clone();
            new_data["fov"] = serde_json::json!(fov);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Camera".to_string(),
                data: new_data,
            });
        }
    });

    let mut near = data.get("near").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
    ui.horizontal(|ui| {
        ui.label("Near Clip:");
        if ui
            .add(
                egui::DragValue::new(&mut near)
                    .speed(0.01)
                    .range(0.001..=10.0),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["near"] = serde_json::json!(near);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Camera".to_string(),
                data: new_data,
            });
        }
    });

    let mut far = data.get("far").and_then(|v| v.as_f64()).unwrap_or(1000.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Far Clip:");
        if ui
            .add(
                egui::DragValue::new(&mut far)
                    .speed(1.0)
                    .range(1.0..=100000.0),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["far"] = serde_json::json!(far);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Camera".to_string(),
                data: new_data,
            });
        }
    });

    let projection_str = data
        .get("projection")
        .and_then(|v| v.as_str())
        .unwrap_or("perspective");
    let mut proj_idx = if projection_str == "orthographic" {
        1
    } else {
        0
    };
    ui.horizontal(|ui| {
        ui.label("Projection:");
        let changed = egui::ComboBox::from_id_salt(format!("cam_proj_{}", entity_id))
            .selected_text(if proj_idx == 0 {
                "Perspective"
            } else {
                "Orthographic"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut proj_idx, 0, "Perspective")
                    .changed()
                    | ui.selectable_value(&mut proj_idx, 1, "Orthographic")
                        .changed()
            })
            .inner
            .unwrap_or(false);
        if changed {
            let mut new_data = data.clone();
            new_data["projection"] = serde_json::json!(if proj_idx == 0 {
                "perspective"
            } else {
                "orthographic"
            });
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Camera".to_string(),
                data: new_data,
            });
        }
    });
}

pub(super) fn show_script_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let path = data
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut path_buf = path.clone();
    ui.horizontal(|ui| {
        ui.label("Script:");
        if ui.text_edit_singleline(&mut path_buf).lost_focus() && path_buf != path {
            let mut new_data = data.clone();
            new_data["path"] = serde_json::json!(path_buf);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Script".to_string(),
                data: new_data,
            });
        }
        if ui
            .small_button("\u{1f4c2}")
            .on_hover_text("Browse for script file")
            .clicked()
        {
            if let Some(file_path) = rfd::FileDialog::new()
                .add_filter("Scripts", &["lua", "rhai", "wasm", "rs"])
                .pick_file()
            {
                let mut new_data = data.clone();
                new_data["path"] = serde_json::json!(file_path.to_string_lossy().to_string());
                events.push(PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type: "Script".to_string(),
                    data: new_data,
                });
            }
        }
    });

    let mut enabled = data
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    ui.horizontal(|ui| {
        ui.label("Enabled:");
        if ui.checkbox(&mut enabled, "").changed() {
            let mut new_data = data.clone();
            new_data["enabled"] = serde_json::json!(enabled);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Script".to_string(),
                data: new_data,
            });
        }
    });
}

pub(super) fn show_particle_inspector(
    ui: &mut egui::Ui,
    entity_id: u64,
    data: &serde_json::Value,
    events: &mut Vec<PanelEvent>,
) {
    let mut emission_rate = data
        .get("emission_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Emission Rate:");
        if ui
            .add(
                egui::DragValue::new(&mut emission_rate)
                    .speed(1.0)
                    .range(0.0..=10000.0),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["emission_rate"] = serde_json::json!(emission_rate);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let mut lifetime = data.get("lifetime").and_then(|v| v.as_f64()).unwrap_or(2.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Lifetime:");
        if ui
            .add(
                egui::DragValue::new(&mut lifetime)
                    .speed(0.1)
                    .range(0.01..=60.0)
                    .suffix("s"),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["lifetime"] = serde_json::json!(lifetime);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let mut start_size = data
        .get("start_size")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1) as f32;
    ui.horizontal(|ui| {
        ui.label("Start Size:");
        if ui
            .add(
                egui::DragValue::new(&mut start_size)
                    .speed(0.01)
                    .range(0.001..=10.0),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["start_size"] = serde_json::json!(start_size);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let mut speed = data.get("speed").and_then(|v| v.as_f64()).unwrap_or(5.0) as f32;
    ui.horizontal(|ui| {
        ui.label("Speed:");
        if ui
            .add(
                egui::DragValue::new(&mut speed)
                    .speed(0.1)
                    .range(0.0..=100.0),
            )
            .changed()
        {
            let mut new_data = data.clone();
            new_data["speed"] = serde_json::json!(speed);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let r = data.get("color_r").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let g = data.get("color_g").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let b = data.get("color_b").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let a = data.get("color_a").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut color = [r, g, b, a];
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
            let mut new_data = data.clone();
            new_data["color_r"] = serde_json::json!(color[0]);
            new_data["color_g"] = serde_json::json!(color[1]);
            new_data["color_b"] = serde_json::json!(color[2]);
            new_data["color_a"] = serde_json::json!(color[3]);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let shape_str = data.get("shape").and_then(|v| v.as_str()).unwrap_or("cone");
    let mut shape_idx = match shape_str {
        "sphere" => 0,
        "cone" => 1,
        "box" => 2,
        _ => 1,
    };
    ui.horizontal(|ui| {
        ui.label("Shape:");
        let changed = egui::ComboBox::from_id_salt(format!("particle_shape_{}", entity_id))
            .selected_text(match shape_idx {
                0 => "Sphere",
                1 => "Cone",
                2 => "Box",
                _ => "Cone",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut shape_idx, 0, "Sphere").changed()
                    | ui.selectable_value(&mut shape_idx, 1, "Cone").changed()
                    | ui.selectable_value(&mut shape_idx, 2, "Box").changed()
            })
            .inner
            .unwrap_or(false);
        if changed {
            let s = match shape_idx {
                0 => "sphere",
                2 => "box",
                _ => "cone",
            };
            let mut new_data = data.clone();
            new_data["shape"] = serde_json::json!(s);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });

    let mut looping = data
        .get("looping")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    ui.horizontal(|ui| {
        ui.label("Loop:");
        if ui.checkbox(&mut looping, "").changed() {
            let mut new_data = data.clone();
            new_data["looping"] = serde_json::json!(looping);
            events.push(PanelEvent::ComponentDataChanged {
                entity_id,
                component_type: "Particle".to_string(),
                data: new_data,
            });
        }
    });
}
