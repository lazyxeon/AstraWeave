use egui::Ui;
use std::collections::VecDeque;
use tracing::info;

/// Single render pass record captured by the frame debugger.
#[derive(Debug, Clone)]
pub struct RenderPassRecord {
    pub name: String,
    pub draw_calls: u32,
    pub triangles: u32,
    pub time_ms: f32,
    pub color: egui::Color32,
}

/// Snapshot of one full frame, including all render passes.
#[derive(Debug, Clone, Default)]
pub struct FrameSnapshot {
    pub frame_number: u64,
    pub total_gpu_ms: f32,
    pub total_draw_calls: u32,
    pub total_triangles: u32,
    pub passes: Vec<RenderPassRecord>,
}

/// Frame Debugger panel — shows per-pass GPU timing and draw call breakdown.
pub struct FrameDebuggerPanel {
    /// Ring buffer of recent frame snapshots
    history: VecDeque<FrameSnapshot>,
    max_history: usize,
    /// Currently inspected frame (index into history)
    selected_frame: usize,
    /// Whether live capture is running
    capturing: bool,
    /// Frame counter used to assign numbers
    frame_counter: u64,
    /// Show the per-pass bar chart
    show_bar_chart: bool,
}

impl Default for FrameDebuggerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDebuggerPanel {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(120),
            max_history: 120,
            selected_frame: 0,
            capturing: true,
            frame_counter: 0,
            show_bar_chart: true,
        }
    }

    /// Push a new frame snapshot (call once per frame while capturing).
    pub fn push_snapshot(&mut self, snapshot: FrameSnapshot) {
        if !self.capturing {
            return;
        }
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);
        // Auto-select latest
        self.selected_frame = self.history.len().saturating_sub(1);
    }

    /// Build a simulated snapshot from known AstraWeave render pipeline passes.
    /// In production this would read actual GPU timestamps; here we use the
    /// measured subsystem timing to give a reasonable breakdown.
    pub fn build_snapshot(&mut self, render_ms: f32, entity_count: usize, terrain_active: bool) {
        self.frame_counter += 1;

        // Approximate per-pass breakdown (proportional to measured total)
        let skybox_ms = render_ms * 0.03;
        let grid_ms = render_ms * 0.04;
        let terrain_ms = if terrain_active {
            render_ms * 0.25
        } else {
            0.0
        };
        let scatter_ms = if terrain_active {
            render_ms * 0.10
        } else {
            0.0
        };
        let water_ms = if terrain_active {
            render_ms * 0.06
        } else {
            0.0
        };
        let entity_ms = render_ms * 0.30;
        let physics_ms = render_ms * 0.05;
        let gizmo_ms = render_ms * 0.04;
        let weather_ms = render_ms * 0.05;
        let ui_gizmo_ms = render_ms
            - skybox_ms
            - grid_ms
            - terrain_ms
            - scatter_ms
            - water_ms
            - entity_ms
            - physics_ms
            - gizmo_ms
            - weather_ms;

        let entity_tris = (entity_count as u32) * 12; // cubes = 12 tris each
        let terrain_tris = if terrain_active { 32768 } else { 0 };

        let mut passes = vec![
            RenderPassRecord {
                name: "Skybox".into(),
                draw_calls: 1,
                triangles: 2,
                time_ms: skybox_ms,
                color: egui::Color32::from_rgb(80, 130, 200),
            },
            RenderPassRecord {
                name: "Grid".into(),
                draw_calls: 1,
                triangles: 2,
                time_ms: grid_ms,
                color: egui::Color32::from_rgb(120, 120, 120),
            },
        ];

        if terrain_active {
            passes.push(RenderPassRecord {
                name: "Terrain".into(),
                draw_calls: 4,
                triangles: terrain_tris,
                time_ms: terrain_ms,
                color: egui::Color32::from_rgb(90, 160, 60),
            });
            passes.push(RenderPassRecord {
                name: "Scatter".into(),
                draw_calls: 8,
                triangles: 4096,
                time_ms: scatter_ms,
                color: egui::Color32::from_rgb(50, 140, 50),
            });
            passes.push(RenderPassRecord {
                name: "Water".into(),
                draw_calls: 1,
                triangles: 2,
                time_ms: water_ms,
                color: egui::Color32::from_rgb(60, 120, 200),
            });
        }

        passes.push(RenderPassRecord {
            name: "Entities".into(),
            draw_calls: entity_count as u32,
            triangles: entity_tris,
            time_ms: entity_ms,
            color: egui::Color32::from_rgb(200, 160, 60),
        });

        passes.push(RenderPassRecord {
            name: "Physics Debug".into(),
            draw_calls: 1,
            triangles: 256,
            time_ms: physics_ms,
            color: egui::Color32::from_rgb(200, 80, 80),
        });

        passes.push(RenderPassRecord {
            name: "Gizmos".into(),
            draw_calls: 3,
            triangles: 512,
            time_ms: gizmo_ms,
            color: egui::Color32::from_rgb(220, 200, 60),
        });

        passes.push(RenderPassRecord {
            name: "Weather".into(),
            draw_calls: 1,
            triangles: 1024,
            time_ms: weather_ms,
            color: egui::Color32::from_rgb(140, 140, 200),
        });

        passes.push(RenderPassRecord {
            name: "UI Gizmos".into(),
            draw_calls: 2,
            triangles: 64,
            time_ms: ui_gizmo_ms.max(0.0),
            color: egui::Color32::from_rgb(180, 100, 180),
        });

        let total_draw_calls: u32 = passes.iter().map(|p| p.draw_calls).sum();
        let total_triangles: u32 = passes.iter().map(|p| p.triangles).sum();

        self.push_snapshot(FrameSnapshot {
            frame_number: self.frame_counter,
            total_gpu_ms: render_ms,
            total_draw_calls,
            total_triangles,
            passes,
        });
    }

    /// Render the frame debugger UI.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Frame Debugger");
        ui.separator();

        // Controls
        ui.horizontal(|ui| {
            if self.capturing {
                if ui.button("Pause").clicked() {
                    info!("frame_debugger: capture paused");
                    self.capturing = false;
                }
            } else if ui.button("Resume").clicked() {
                info!("frame_debugger: capture resumed");
                self.capturing = true;
            }

            if ui.button("Clear").clicked() {
                self.history.clear();
                self.selected_frame = 0;
            }

            ui.checkbox(&mut self.show_bar_chart, "Bar Chart");

            ui.label(format!("Frames: {}", self.history.len()));
        });

        ui.separator();

        if self.history.is_empty() {
            ui.label("No frames captured yet.");
            return;
        }

        // Frame selector slider
        let max_idx = self.history.len().saturating_sub(1);
        let mut sel = self.selected_frame.min(max_idx);
        ui.horizontal(|ui| {
            ui.label("Frame:");
            ui.add(egui::Slider::new(&mut sel, 0..=max_idx).show_value(true));
        });
        self.selected_frame = sel;

        let snap = self.history[self.selected_frame].clone();

        ui.separator();

        // Summary
        ui.horizontal(|ui| {
            ui.strong(format!("Frame #{}", snap.frame_number));
            ui.separator();
            ui.label(format!("GPU: {:.2} ms", snap.total_gpu_ms));
            ui.separator();
            ui.label(format!("Draw Calls: {}", snap.total_draw_calls));
            ui.separator();
            ui.label(format!("Triangles: {}", snap.total_triangles));
        });

        ui.separator();

        // Per-pass table
        egui::Grid::new("frame_debugger_passes")
            .num_columns(5)
            .striped(true)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Pass");
                ui.strong("Time (ms)");
                ui.strong("Draw Calls");
                ui.strong("Triangles");
                ui.strong("% of Frame");
                ui.end_row();

                for pass in &snap.passes {
                    let pct = if snap.total_gpu_ms > 0.0 {
                        (pass.time_ms / snap.total_gpu_ms) * 100.0
                    } else {
                        0.0
                    };
                    ui.colored_label(pass.color, &pass.name);
                    ui.label(format!("{:.3}", pass.time_ms));
                    ui.label(format!("{}", pass.draw_calls));
                    ui.label(format!("{}", pass.triangles));
                    ui.label(format!("{:.1}%", pct));
                    ui.end_row();
                }
            });

        // Bar chart visualisation
        if self.show_bar_chart {
            ui.separator();
            ui.label("Pass Timing Breakdown:");
            let avail_width = ui.available_width().max(100.0);
            let bar_height = 18.0;
            let total = snap.total_gpu_ms.max(0.001);

            for pass in &snap.passes {
                let fraction = (pass.time_ms / total).clamp(0.0, 1.0);
                let bar_width = fraction * avail_width;
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(bar_width.max(2.0), bar_height),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 2.0, pass.color);
                    ui.label(format!("{}: {:.2} ms", pass.name, pass.time_ms));
                });
            }
        }
    }
}
