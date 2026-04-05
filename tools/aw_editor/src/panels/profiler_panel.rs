use super::Panel;
use egui::Ui;
use std::collections::VecDeque;

/// Subsystem timing data for detailed breakdown
#[derive(Debug, Clone, Default)]
pub struct SubsystemTimings {
    pub render: f32,
    pub physics: f32,
    pub ai: f32,
    pub audio: f32,
    pub scripts: f32,
    pub animation: f32,
    pub ui: f32,
    pub network: f32,
}

impl SubsystemTimings {
    pub fn total(&self) -> f32 {
        self.render
            + self.physics
            + self.ai
            + self.audio
            + self.scripts
            + self.animation
            + self.ui
            + self.network
    }
}

/// GPU profiling metrics
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub draw_calls: u32,
    pub triangles: u32,
    pub vertices: u32,
    pub gpu_time_ms: f32,
    pub vram_used_mb: f32,
    pub vram_total_mb: f32,
    pub textures_bound: u32,
    pub shader_switches: u32,
    pub state_changes: u32,
}

/// Flame graph node for hierarchical profiling
#[derive(Debug, Clone)]
pub struct FlameNode {
    pub name: String,
    pub time_ms: f32,
    pub children: Vec<FlameNode>,
    pub color: egui::Color32,
}

impl FlameNode {
    pub fn new(name: &str, time_ms: f32, color: egui::Color32) -> Self {
        Self {
            name: name.to_string(),
            time_ms,
            children: Vec::new(),
            color,
        }
    }

    pub fn total_time(&self) -> f32 {
        self.time_ms + self.children.iter().map(|c| c.total_time()).sum::<f32>()
    }
}

/// Memory allocation category
#[derive(Debug, Clone)]
pub struct MemoryCategory {
    pub name: String,
    pub used_bytes: usize,
    pub allocated_bytes: usize,
    pub allocation_count: u32,
    pub color: egui::Color32,
}

/// Profiler panel with advanced features
pub struct ProfilerPanel {
    // Basic metrics
    frame_times: VecDeque<f32>,
    fps_samples: VecDeque<f32>,
    memory_samples: VecDeque<usize>,
    max_samples: usize,
    last_update: std::time::Instant,
    update_interval_ms: u64,

    // Visibility toggles
    show_frame_graph: bool,
    show_fps_graph: bool,
    show_memory_graph: bool,
    show_subsystem_breakdown: bool,
    show_gpu_stats: bool,
    show_flame_graph: bool,
    show_memory_inspector: bool,

    // Peak values
    peak_frame_time: f32,
    peak_fps: f32,
    peak_memory_kb: usize,

    // Advanced metrics
    subsystem_timings: SubsystemTimings,
    subsystem_history: VecDeque<SubsystemTimings>,
    gpu_metrics: GpuMetrics,
    gpu_time_history: VecDeque<f32>,
    flame_root: Option<FlameNode>,
    memory_categories: Vec<MemoryCategory>,

    // Tab selection
    selected_tab: ProfilerTab,

    // Display options
    graph_height: f32,
    target_fps: f32,
    pause_profiling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilerTab {
    Overview,
    Subsystems,
    Gpu,
    FlameGraph,
    Memory,
}

impl ProfilerPanel {
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(120),
            fps_samples: VecDeque::with_capacity(120),
            memory_samples: VecDeque::with_capacity(120),
            max_samples: 120,
            last_update: std::time::Instant::now(),
            update_interval_ms: 16,
            show_frame_graph: true,
            show_fps_graph: true,
            show_memory_graph: false,
            show_subsystem_breakdown: true,
            show_gpu_stats: true,
            show_flame_graph: false,
            show_memory_inspector: false,
            peak_frame_time: 0.0,
            peak_fps: 0.0,
            peak_memory_kb: 0,
            subsystem_timings: SubsystemTimings::default(),
            subsystem_history: VecDeque::with_capacity(120),
            gpu_metrics: GpuMetrics::default(),
            gpu_time_history: VecDeque::with_capacity(120),
            flame_root: None,
            memory_categories: Vec::new(),
            selected_tab: ProfilerTab::Overview,
            graph_height: 80.0,
            target_fps: 60.0,
            pause_profiling: false,
        }
    }

    pub fn create_sample_memory_categories() -> Vec<MemoryCategory> {
        vec![
            MemoryCategory {
                name: "Textures".to_string(),
                used_bytes: 128 * 1024 * 1024,
                allocated_bytes: 256 * 1024 * 1024,
                allocation_count: 847,
                color: egui::Color32::from_rgb(255, 100, 100),
            },
            MemoryCategory {
                name: "Meshes".to_string(),
                used_bytes: 64 * 1024 * 1024,
                allocated_bytes: 96 * 1024 * 1024,
                allocation_count: 1203,
                color: egui::Color32::from_rgb(100, 255, 100),
            },
            MemoryCategory {
                name: "Audio".to_string(),
                used_bytes: 32 * 1024 * 1024,
                allocated_bytes: 48 * 1024 * 1024,
                allocation_count: 256,
                color: egui::Color32::from_rgb(100, 100, 255),
            },
            MemoryCategory {
                name: "Scripts".to_string(),
                used_bytes: 16 * 1024 * 1024,
                allocated_bytes: 24 * 1024 * 1024,
                allocation_count: 3420,
                color: egui::Color32::from_rgb(255, 255, 100),
            },
            MemoryCategory {
                name: "ECS".to_string(),
                used_bytes: 48 * 1024 * 1024,
                allocated_bytes: 64 * 1024 * 1024,
                allocation_count: 50000,
                color: egui::Color32::from_rgb(255, 100, 255),
            },
            MemoryCategory {
                name: "Particles".to_string(),
                used_bytes: 8 * 1024 * 1024,
                allocated_bytes: 16 * 1024 * 1024,
                allocation_count: 128,
                color: egui::Color32::from_rgb(100, 255, 255),
            },
        ]
    }

    pub fn create_sample_flame_graph() -> FlameNode {
        let mut root = FlameNode::new("Frame", 16.67, egui::Color32::from_rgb(80, 80, 80));

        let mut update = FlameNode::new("Update", 4.5, egui::Color32::from_rgb(100, 150, 200));
        update.children.push(FlameNode::new(
            "Physics",
            1.8,
            egui::Color32::from_rgb(255, 100, 100),
        ));
        update.children.push(FlameNode::new(
            "AI",
            1.2,
            egui::Color32::from_rgb(100, 255, 100),
        ));
        update.children.push(FlameNode::new(
            "Animation",
            0.8,
            egui::Color32::from_rgb(100, 100, 255),
        ));
        update.children.push(FlameNode::new(
            "Scripts",
            0.7,
            egui::Color32::from_rgb(255, 255, 100),
        ));

        let mut render = FlameNode::new("Render", 10.2, egui::Color32::from_rgb(200, 150, 100));
        let mut draw_scene =
            FlameNode::new("DrawScene", 7.5, egui::Color32::from_rgb(180, 120, 80));
        draw_scene.children.push(FlameNode::new(
            "ShadowPass",
            2.1,
            egui::Color32::from_rgb(150, 100, 60),
        ));
        draw_scene.children.push(FlameNode::new(
            "GBuffer",
            2.8,
            egui::Color32::from_rgb(140, 90, 50),
        ));
        draw_scene.children.push(FlameNode::new(
            "Lighting",
            1.5,
            egui::Color32::from_rgb(130, 80, 40),
        ));
        draw_scene.children.push(FlameNode::new(
            "Particles",
            1.1,
            egui::Color32::from_rgb(120, 70, 30),
        ));
        render.children.push(draw_scene);
        render.children.push(FlameNode::new(
            "PostProcess",
            1.8,
            egui::Color32::from_rgb(160, 110, 70),
        ));
        render.children.push(FlameNode::new(
            "UI",
            0.9,
            egui::Color32::from_rgb(170, 130, 90),
        ));

        root.children.push(update);
        root.children.push(render);
        root.children.push(FlameNode::new(
            "Audio",
            0.5,
            egui::Color32::from_rgb(100, 200, 150),
        ));
        root.children.push(FlameNode::new(
            "Network",
            0.3,
            egui::Color32::from_rgb(200, 100, 150),
        ));

        root
    }

    pub fn push_frame_time(&mut self, frame_time_ms: f32) {
        if self.pause_profiling {
            return;
        }

        if self.frame_times.len() >= self.max_samples {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(frame_time_ms);

        if frame_time_ms > self.peak_frame_time {
            self.peak_frame_time = frame_time_ms;
        }

        let fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };

        if self.fps_samples.len() >= self.max_samples {
            self.fps_samples.pop_front();
        }
        self.fps_samples.push_back(fps);

        if fps > self.peak_fps {
            self.peak_fps = fps;
        }
    }

    pub fn push_memory_sample(&mut self, memory_kb: usize) {
        if self.pause_profiling {
            return;
        }

        if self.memory_samples.len() >= self.max_samples {
            self.memory_samples.pop_front();
        }
        self.memory_samples.push_back(memory_kb);

        if memory_kb > self.peak_memory_kb {
            self.peak_memory_kb = memory_kb;
        }
    }

    pub fn push_subsystem_timings(&mut self, timings: SubsystemTimings) {
        if self.pause_profiling {
            return;
        }

        if self.subsystem_history.len() >= self.max_samples {
            self.subsystem_history.pop_front();
        }
        self.subsystem_history.push_back(timings.clone());
        self.subsystem_timings = timings;
    }

    pub fn push_gpu_metrics(&mut self, metrics: GpuMetrics) {
        if self.pause_profiling {
            return;
        }

        if self.gpu_time_history.len() >= self.max_samples {
            self.gpu_time_history.pop_front();
        }
        self.gpu_time_history.push_back(metrics.gpu_time_ms);
        self.gpu_metrics = metrics;
    }

    pub fn reset_peaks(&mut self) {
        self.peak_frame_time = 0.0;
        self.peak_fps = 0.0;
        self.peak_memory_kb = 0;
    }

    fn avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    fn avg_fps(&self) -> f32 {
        if self.fps_samples.is_empty() {
            return 0.0;
        }
        self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32
    }

    fn min_fps(&self) -> f32 {
        self.fps_samples
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min)
    }

    fn max_fps(&self) -> f32 {
        self.fps_samples.iter().cloned().fold(0.0, f32::max)
    }

    fn draw_graph(
        &self,
        ui: &mut Ui,
        data: &VecDeque<f32>,
        label: &str,
        max_val: f32,
        color: egui::Color32,
        show_budget_line: bool,
    ) {
        let height = self.graph_height;
        let width = ui.available_width().min(400.0);

        ui.label(label);
        let (response, painter) =
            ui.allocate_painter(egui::vec2(width, height), egui::Sense::hover());
        let rect = response.rect;

        painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));

        if data.len() < 2 {
            return;
        }

        let max_display = if max_val > 0.0 { max_val } else { 1.0 };
        let step = width / (self.max_samples as f32 - 1.0);

        let points: Vec<egui::Pos2> = data
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let x = rect.left() + i as f32 * step;
                let normalized = (val / max_display).clamp(0.0, 1.0);
                let y = rect.bottom() - normalized * height;
                egui::pos2(x, y)
            })
            .collect();

        if points.len() >= 2 {
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
        }

        // Draw target FPS budget line
        let budget_ms = 1000.0 / self.target_fps;
        let budget_line_y = rect.bottom() - (budget_ms / max_display).clamp(0.0, 1.0) * height;
        if show_budget_line && budget_line_y > rect.top() && budget_line_y < rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), budget_line_y),
                    egui::pos2(rect.right(), budget_line_y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 100, 100)),
            );
            painter.text(
                egui::pos2(rect.right() - 60.0, budget_line_y - 10.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{:.0} FPS", self.target_fps),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(255, 100, 100),
            );
        }
    }

    fn draw_subsystem_bar(&self, ui: &mut Ui) {
        let height = 24.0;
        let width = ui.available_width().min(400.0);

        let (response, painter) =
            ui.allocate_painter(egui::vec2(width, height), egui::Sense::hover());
        let rect = response.rect;

        let total = self.subsystem_timings.total();
        if total <= 0.0 {
            painter.rect_filled(rect, 2.0, egui::Color32::from_gray(40));
            return;
        }

        let subsystems = [
            (
                "Render",
                self.subsystem_timings.render,
                egui::Color32::from_rgb(200, 100, 100),
            ),
            (
                "Physics",
                self.subsystem_timings.physics,
                egui::Color32::from_rgb(100, 200, 100),
            ),
            (
                "AI",
                self.subsystem_timings.ai,
                egui::Color32::from_rgb(100, 100, 200),
            ),
            (
                "Audio",
                self.subsystem_timings.audio,
                egui::Color32::from_rgb(200, 200, 100),
            ),
            (
                "Scripts",
                self.subsystem_timings.scripts,
                egui::Color32::from_rgb(200, 100, 200),
            ),
            (
                "Anim",
                self.subsystem_timings.animation,
                egui::Color32::from_rgb(100, 200, 200),
            ),
            (
                "UI",
                self.subsystem_timings.ui,
                egui::Color32::from_rgb(200, 150, 100),
            ),
            (
                "Net",
                self.subsystem_timings.network,
                egui::Color32::from_rgb(150, 100, 200),
            ),
        ];

        let mut x = rect.left();
        for (name, time, color) in subsystems.iter() {
            let w = (*time / total) * width;
            if w > 0.5 {
                let sub_rect =
                    egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, height));
                painter.rect_filled(sub_rect, 0.0, *color);

                if w > 30.0 {
                    painter.text(
                        sub_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        name,
                        egui::FontId::proportional(9.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            x += w;
        }
    }

    fn draw_flame_node(
        &self,
        ui: &mut Ui,
        node: &FlameNode,
        x_start: f32,
        x_end: f32,
        y: f32,
        depth: usize,
    ) {
        if depth > 10 {
            return; // Limit recursion depth
        }

        let row_height = 18.0;
        let (response, painter) = ui.allocate_painter(
            egui::vec2(x_end - x_start, row_height),
            egui::Sense::hover(),
        );

        let rect = egui::Rect::from_min_size(
            egui::pos2(x_start, y),
            egui::vec2(x_end - x_start, row_height),
        );

        // Draw background
        let color = if response.hovered() {
            node.color.linear_multiply(1.3)
        } else {
            node.color
        };
        painter.rect_filled(rect, 1.0, color);
        painter.rect_stroke(
            rect,
            1.0,
            egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
            egui::epaint::StrokeKind::Outside,
        );

        // Draw label
        if rect.width() > 40.0 {
            let text = format!("{} ({:.2}ms)", node.name, node.time_ms);
            painter.text(
                egui::pos2(rect.left() + 4.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &text,
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
        }

        // Draw tooltip
        if response.hovered() {
            response.on_hover_ui(|ui| {
                ui.label(format!("{}: {:.3} ms", node.name, node.time_ms));
                ui.label(format!("Total: {:.3} ms", node.total_time()));
            });
        }
    }

    fn show_overview_tab(&mut self, ui: &mut Ui) {
        // Stats grid
        egui::Grid::new("profiler_stats_grid")
            .num_columns(4)
            .spacing([15.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.label(egui::RichText::new("Current").strong());
                ui.label(egui::RichText::new("Average").strong());
                ui.label(egui::RichText::new("Peak").strong());
                ui.end_row();

                let current_ft = self.frame_times.back().copied().unwrap_or(0.0);
                let avg_ft = self.avg_frame_time();
                ui.label("Frame Time:");
                ui.label(format!("{:.2} ms", current_ft));
                ui.label(format!("{:.2} ms", avg_ft));
                ui.label(format!("{:.2} ms", self.peak_frame_time));
                ui.end_row();

                let current_fps = self.fps_samples.back().copied().unwrap_or(0.0);
                let avg_fps = self.avg_fps();
                ui.label("FPS:");
                let fps_color = if current_fps >= self.target_fps {
                    egui::Color32::GREEN
                } else if current_fps >= self.target_fps * 0.75 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.colored_label(fps_color, format!("{:.1}", current_fps));
                ui.label(format!("{:.1}", avg_fps));
                ui.label(format!("{:.1}", self.peak_fps));
                ui.end_row();

                if !self.memory_samples.is_empty() {
                    let current_mem = self.memory_samples.back().copied().unwrap_or(0);
                    let avg_mem: usize =
                        self.memory_samples.iter().sum::<usize>() / self.memory_samples.len();
                    ui.label("Memory:");
                    ui.label(Self::format_bytes(current_mem * 1024));
                    ui.label(Self::format_bytes(avg_mem * 1024));
                    ui.label(Self::format_bytes(self.peak_memory_kb * 1024));
                    ui.end_row();
                }
            });

        ui.add_space(8.0);

        // Graphs
        if self.show_frame_graph && !self.frame_times.is_empty() {
            let max_ft = self
                .frame_times
                .iter()
                .cloned()
                .fold(0.0f32, f32::max)
                .max(33.33);
            self.draw_graph(
                ui,
                &self.frame_times,
                "Frame Time (ms)",
                max_ft,
                egui::Color32::from_rgb(100, 200, 255),
                true,
            );
        }

        if self.show_fps_graph && !self.fps_samples.is_empty() {
            ui.add_space(4.0);
            let max_fps = self.max_fps().max(120.0);
            self.draw_graph(
                ui,
                &self.fps_samples,
                "FPS",
                max_fps,
                egui::Color32::from_rgb(100, 255, 100),
                false,
            );
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Min FPS:");
            ui.label(format!("{:.1}", self.min_fps()));
            ui.separator();
            ui.label("Max FPS:");
            ui.label(format!("{:.1}", self.max_fps()));
            ui.separator();
            ui.label("Samples:");
            ui.label(format!("{}", self.frame_times.len()));
        });
    }

    fn show_subsystems_tab(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Subsystem Breakdown").strong());
        ui.add_space(4.0);

        self.draw_subsystem_bar(ui);
        ui.add_space(8.0);

        egui::Grid::new("subsystem_grid")
            .num_columns(3)
            .spacing([20.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subsystem").strong());
                ui.label(egui::RichText::new("Time (ms)").strong());
                ui.label(egui::RichText::new("% Budget").strong());
                ui.end_row();

                let budget = 1000.0 / self.target_fps;
                let subsystems = [
                    (
                        "Render",
                        self.subsystem_timings.render,
                        egui::Color32::from_rgb(200, 100, 100),
                    ),
                    (
                        "Physics",
                        self.subsystem_timings.physics,
                        egui::Color32::from_rgb(100, 200, 100),
                    ),
                    (
                        "AI",
                        self.subsystem_timings.ai,
                        egui::Color32::from_rgb(100, 100, 200),
                    ),
                    (
                        "Audio",
                        self.subsystem_timings.audio,
                        egui::Color32::from_rgb(200, 200, 100),
                    ),
                    (
                        "Scripts",
                        self.subsystem_timings.scripts,
                        egui::Color32::from_rgb(200, 100, 200),
                    ),
                    (
                        "Animation",
                        self.subsystem_timings.animation,
                        egui::Color32::from_rgb(100, 200, 200),
                    ),
                    (
                        "UI",
                        self.subsystem_timings.ui,
                        egui::Color32::from_rgb(200, 150, 100),
                    ),
                    (
                        "Network",
                        self.subsystem_timings.network,
                        egui::Color32::from_rgb(150, 100, 200),
                    ),
                ];

                for (name, time, color) in subsystems.iter() {
                    let percent = (*time / budget) * 100.0;
                    ui.colored_label(*color, *name);
                    ui.label(format!("{:.2}", time));

                    let percent_color = if percent < 15.0 {
                        egui::Color32::GREEN
                    } else if percent < 30.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(percent_color, format!("{:.1}%", percent));
                    ui.end_row();
                }

                ui.separator();
                ui.end_row();
                ui.label(egui::RichText::new("Total").strong());
                ui.label(format!("{:.2}", self.subsystem_timings.total()));
                ui.label(format!(
                    "{:.1}%",
                    (self.subsystem_timings.total() / budget) * 100.0
                ));
                ui.end_row();
            });
    }

    fn show_gpu_tab(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("GPU Statistics").strong());
        ui.add_space(4.0);

        egui::Grid::new("gpu_stats_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("GPU Time:");
                ui.label(format!("{:.2} ms", self.gpu_metrics.gpu_time_ms));
                ui.end_row();

                ui.label("Draw Calls:");
                ui.label(format!("{}", self.gpu_metrics.draw_calls));
                ui.end_row();

                ui.label("Triangles:");
                ui.label(Self::format_count(self.gpu_metrics.triangles));
                ui.end_row();

                ui.label("Vertices:");
                ui.label(Self::format_count(self.gpu_metrics.vertices));
                ui.end_row();

                ui.label("Textures Bound:");
                ui.label(format!("{}", self.gpu_metrics.textures_bound));
                ui.end_row();

                ui.label("Shader Switches:");
                ui.label(format!("{}", self.gpu_metrics.shader_switches));
                ui.end_row();

                ui.label("State Changes:");
                ui.label(format!("{}", self.gpu_metrics.state_changes));
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("VRAM Usage").strong());

        let vram_percent = if self.gpu_metrics.vram_total_mb > 0.0 {
            self.gpu_metrics.vram_used_mb / self.gpu_metrics.vram_total_mb
        } else {
            0.0
        };

        let bar = egui::ProgressBar::new(vram_percent).text(format!(
            "{:.0} MB / {:.0} MB ({:.1}%)",
            self.gpu_metrics.vram_used_mb,
            self.gpu_metrics.vram_total_mb,
            vram_percent * 100.0
        ));
        ui.add(bar);

        if !self.gpu_time_history.is_empty() {
            ui.add_space(8.0);
            let max_gpu = self
                .gpu_time_history
                .iter()
                .cloned()
                .fold(0.0f32, f32::max)
                .max(16.67);
            self.draw_graph(
                ui,
                &self.gpu_time_history,
                "GPU Time (ms)",
                max_gpu,
                egui::Color32::from_rgb(255, 150, 100),
                true,
            );
        }
    }

    fn show_flame_graph_tab(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Flame Graph").strong());
        ui.label("Hierarchical view of frame time distribution");
        ui.add_space(8.0);

        if self.flame_root.is_none() {
            ui.label("No flame graph data available. Profiling data will appear when subsystems are active.");
            return;
        }

        if let Some(root) = &self.flame_root {
            // Draw flame graph header
            let row_height = 18.0;
            let width = ui.available_width().min(500.0);

            // Helper function to calculate positions recursively
            fn draw_flame_recursive(
                ui: &mut Ui,
                node: &FlameNode,
                x_start: f32,
                total_width: f32,
                total_time: f32,
                y: &mut f32,
                row_height: f32,
            ) {
                if total_time <= 0.0 {
                    return;
                }
                let node_width = (node.time_ms / total_time) * total_width;
                if node_width < 1.0 {
                    return;
                }

                let (response, painter) =
                    ui.allocate_painter(egui::vec2(total_width, row_height), egui::Sense::hover());

                let rect = egui::Rect::from_min_size(
                    egui::pos2(response.rect.left() + x_start, response.rect.top()),
                    egui::vec2(node_width, row_height),
                );

                painter.rect_filled(rect, 1.0, node.color);
                painter.rect_stroke(
                    rect,
                    1.0,
                    egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
                    egui::epaint::StrokeKind::Outside,
                );

                if node_width > 50.0 {
                    let text = format!("{} {:.1}ms", node.name, node.time_ms);
                    painter.text(
                        egui::pos2(rect.left() + 3.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        egui::FontId::proportional(9.0),
                        egui::Color32::WHITE,
                    );
                }

                *y += row_height + 1.0;

                // Draw children
                let mut child_x = x_start;
                for child in &node.children {
                    draw_flame_recursive(
                        ui,
                        child,
                        child_x,
                        total_width,
                        total_time,
                        y,
                        row_height,
                    );
                    child_x += (child.time_ms / total_time) * total_width;
                }
            }

            let total_time = root.total_time();
            let mut y = 0.0;
            draw_flame_recursive(ui, root, 0.0, width, total_time, &mut y, row_height);
        }
    }

    fn show_memory_tab(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Memory Inspector").strong());
        ui.add_space(4.0);

        // Memory overview
        let total_used: usize = self.memory_categories.iter().map(|c| c.used_bytes).sum();
        let total_allocated: usize = self
            .memory_categories
            .iter()
            .map(|c| c.allocated_bytes)
            .sum();

        ui.horizontal(|ui| {
            ui.label("Total Used:");
            ui.label(egui::RichText::new(Self::format_bytes(total_used)).strong());
            ui.separator();
            ui.label("Total Allocated:");
            ui.label(Self::format_bytes(total_allocated));
        });

        ui.add_space(8.0);

        // Category breakdown
        egui::Grid::new("memory_categories_grid")
            .num_columns(5)
            .spacing([15.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Category").strong());
                ui.label(egui::RichText::new("Used").strong());
                ui.label(egui::RichText::new("Allocated").strong());
                ui.label(egui::RichText::new("Util %").strong());
                ui.label(egui::RichText::new("Allocs").strong());
                ui.end_row();

                for cat in &self.memory_categories {
                    let util = if cat.allocated_bytes > 0 {
                        (cat.used_bytes as f32 / cat.allocated_bytes as f32) * 100.0
                    } else {
                        0.0
                    };

                    ui.colored_label(cat.color, &cat.name);
                    ui.label(Self::format_bytes(cat.used_bytes));
                    ui.label(Self::format_bytes(cat.allocated_bytes));
                    ui.label(format!("{:.1}%", util));
                    ui.label(Self::format_count(cat.allocation_count));
                    ui.end_row();
                }
            });

        ui.add_space(8.0);

        // Memory graph
        if self.show_memory_graph && !self.memory_samples.is_empty() {
            let mem_floats: VecDeque<f32> = self.memory_samples.iter().map(|&m| m as f32).collect();
            let max_mem = self.peak_memory_kb as f32;
            self.draw_graph(
                ui,
                &mem_floats,
                "Memory Usage Over Time",
                max_mem.max(1024.0),
                egui::Color32::from_rgb(255, 180, 100),
                false,
            );
        }
    }

    fn format_bytes(bytes: usize) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_count(count: u32) -> String {
        if count >= 1_000_000 {
            format!("{:.2}M", count as f64 / 1_000_000.0)
        } else if count >= 1_000 {
            format!("{:.1}K", count as f64 / 1_000.0)
        } else {
            format!("{}", count)
        }
    }
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for ProfilerPanel {
    fn name(&self) -> &str {
        "Profiler"
    }

    fn show(&mut self, ui: &mut Ui) {
        // Header with controls
        ui.horizontal(|ui| {
            ui.heading("Performance Profiler");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Reset Peaks").clicked() {
                    self.reset_peaks();
                }

                let pause_text = if self.pause_profiling {
                    "> Resume"
                } else {
                    "|| Pause"
                };
                if ui.small_button(pause_text).clicked() {
                    self.pause_profiling = !self.pause_profiling;
                }

                ui.separator();
                ui.label("Target:");
                egui::ComboBox::from_id_salt("target_fps")
                    .width(60.0)
                    .selected_text(format!("{:.0} FPS", self.target_fps))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.target_fps, 30.0, "30 FPS");
                        ui.selectable_value(&mut self.target_fps, 60.0, "60 FPS");
                        ui.selectable_value(&mut self.target_fps, 90.0, "90 FPS");
                        ui.selectable_value(&mut self.target_fps, 120.0, "120 FPS");
                        ui.selectable_value(&mut self.target_fps, 144.0, "144 FPS");
                    });
            });
        });

        if self.pause_profiling {
            ui.colored_label(egui::Color32::YELLOW, "|| Profiling paused");
        }

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_tab == ProfilerTab::Overview, "📈 Overview")
                .clicked()
            {
                self.selected_tab = ProfilerTab::Overview;
            }
            if ui
                .selectable_label(self.selected_tab == ProfilerTab::Subsystems, "Subsystems")
                .clicked()
            {
                self.selected_tab = ProfilerTab::Subsystems;
            }
            if ui
                .selectable_label(self.selected_tab == ProfilerTab::Gpu, "GPU")
                .clicked()
            {
                self.selected_tab = ProfilerTab::Gpu;
            }
            if ui
                .selectable_label(self.selected_tab == ProfilerTab::FlameGraph, "[Fire] Flame")
                .clicked()
            {
                self.selected_tab = ProfilerTab::FlameGraph;
            }
            if ui
                .selectable_label(self.selected_tab == ProfilerTab::Memory, "Memory")
                .clicked()
            {
                self.selected_tab = ProfilerTab::Memory;
            }
        });

        ui.separator();

        // Tab content
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.selected_tab {
                ProfilerTab::Overview => self.show_overview_tab(ui),
                ProfilerTab::Subsystems => self.show_subsystems_tab(ui),
                ProfilerTab::Gpu => self.show_gpu_tab(ui),
                ProfilerTab::FlameGraph => self.show_flame_graph_tab(ui),
                ProfilerTab::Memory => self.show_memory_tab(ui),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_panel_creation() {
        let panel = ProfilerPanel::new();
        assert!(panel.frame_times.is_empty());
        assert_eq!(panel.max_samples, 120);
        assert_eq!(panel.target_fps, 60.0);
        assert!(!panel.pause_profiling);
    }

    #[test]
    fn test_push_frame_time() {
        let mut panel = ProfilerPanel::new();
        panel.push_frame_time(16.67);
        panel.push_frame_time(8.33);
        assert_eq!(panel.frame_times.len(), 2);
        assert_eq!(panel.fps_samples.len(), 2);
        assert!((panel.peak_frame_time - 16.67).abs() < 0.01);
    }

    #[test]
    fn test_avg_calculations() {
        let mut panel = ProfilerPanel::new();
        panel.push_frame_time(10.0);
        panel.push_frame_time(20.0);
        assert!((panel.avg_frame_time() - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_reset_peaks() {
        let mut panel = ProfilerPanel::new();
        panel.push_frame_time(100.0);
        panel.push_memory_sample(50000);
        panel.reset_peaks();
        assert_eq!(panel.peak_frame_time, 0.0);
        assert_eq!(panel.peak_memory_kb, 0);
    }

    #[test]
    fn test_max_samples_limit() {
        let mut panel = ProfilerPanel::new();
        for i in 0..150 {
            panel.push_frame_time(i as f32);
        }
        assert_eq!(panel.frame_times.len(), 120);
    }

    #[test]
    fn test_push_memory_sample() {
        let mut panel = ProfilerPanel::new();
        panel.push_memory_sample(1024);
        panel.push_memory_sample(2048);
        assert_eq!(panel.memory_samples.len(), 2);
        assert_eq!(panel.peak_memory_kb, 2048);
    }

    #[test]
    fn test_memory_peak_logic() {
        let mut panel = ProfilerPanel::new();
        panel.push_memory_sample(100);
        assert_eq!(panel.peak_memory_kb, 100);
        panel.push_memory_sample(50);
        assert_eq!(panel.peak_memory_kb, 100);
        panel.push_memory_sample(200);
        assert_eq!(panel.peak_memory_kb, 200);
    }

    #[test]
    fn test_min_max_fps() {
        let mut panel = ProfilerPanel::new();
        panel.push_frame_time(100.0); // 10 FPS
        panel.push_frame_time(10.0); // 100 FPS

        let min = panel.min_fps();
        let max = panel.max_fps();

        assert!((min - 10.0).abs() < 0.1);
        assert!((max - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_push_frame_time_zero() {
        let mut panel = ProfilerPanel::new();
        panel.push_frame_time(0.0);

        // Should handle gracefully (infinite FPS technically, but logic handles it)
        let last_fps = *panel.fps_samples.back().unwrap();
        assert_eq!(last_fps, 0.0);
    }

    #[test]
    fn test_profiler_defaults() {
        let panel = ProfilerPanel::default();
        assert!(panel.show_frame_graph);
        assert!(panel.show_fps_graph);
        assert!(!panel.show_memory_graph);
        assert_eq!(panel.selected_tab, ProfilerTab::Overview);
    }

    // === New tests for enhanced profiler ===

    #[test]
    fn test_subsystem_timings() {
        let timings = SubsystemTimings {
            render: 5.0,
            physics: 2.0,
            ai: 1.0,
            audio: 0.5,
            scripts: 0.3,
            animation: 0.2,
            ui: 0.1,
            network: 0.1,
        };
        assert!((timings.total() - 9.2).abs() < 0.01);
    }

    #[test]
    fn test_push_subsystem_timings() {
        let mut panel = ProfilerPanel::new();
        let timings = SubsystemTimings {
            render: 8.0,
            physics: 2.0,
            ..Default::default()
        };
        panel.push_subsystem_timings(timings);
        assert_eq!(panel.subsystem_history.len(), 1);
        assert!((panel.subsystem_timings.render - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_gpu_metrics() {
        let mut panel = ProfilerPanel::new();
        let metrics = GpuMetrics {
            draw_calls: 500,
            triangles: 100_000,
            vertices: 50_000,
            gpu_time_ms: 8.5,
            vram_used_mb: 512.0,
            vram_total_mb: 8192.0,
            textures_bound: 32,
            shader_switches: 15,
            state_changes: 120,
        };
        panel.push_gpu_metrics(metrics);
        assert_eq!(panel.gpu_metrics.draw_calls, 500);
        assert_eq!(panel.gpu_time_history.len(), 1);
    }

    #[test]
    fn test_pause_profiling() {
        let mut panel = ProfilerPanel::new();
        panel.pause_profiling = true;

        panel.push_frame_time(16.67);
        assert!(panel.frame_times.is_empty()); // Should not add when paused

        panel.pause_profiling = false;
        panel.push_frame_time(16.67);
        assert_eq!(panel.frame_times.len(), 1); // Should add when resumed
    }

    #[test]
    fn test_flame_node() {
        let node = FlameNode::new("Test", 5.0, egui::Color32::RED);
        assert_eq!(node.name, "Test");
        assert!((node.time_ms - 5.0).abs() < 0.01);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_flame_node_total_time() {
        let mut parent = FlameNode::new("Parent", 5.0, egui::Color32::RED);
        parent
            .children
            .push(FlameNode::new("Child1", 2.0, egui::Color32::GREEN));
        parent
            .children
            .push(FlameNode::new("Child2", 3.0, egui::Color32::BLUE));

        assert!((parent.total_time() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(ProfilerPanel::format_bytes(500), "500 B");
        assert_eq!(ProfilerPanel::format_bytes(1024), "1.00 KB");
        assert_eq!(ProfilerPanel::format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(ProfilerPanel::format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(ProfilerPanel::format_count(500), "500");
        assert_eq!(ProfilerPanel::format_count(1500), "1.5K");
        assert_eq!(ProfilerPanel::format_count(1_500_000), "1.50M");
    }

    #[test]
    fn test_memory_categories_created() {
        let panel = ProfilerPanel::new();
        // Memory categories start empty; use create_sample_memory_categories() for testing
        let categories = ProfilerPanel::create_sample_memory_categories();
        assert!(!categories.is_empty());
        assert!(categories.len() >= 5);
    }

    #[test]
    fn test_sample_flame_graph() {
        let flame = ProfilerPanel::create_sample_flame_graph();
        assert_eq!(flame.name, "Frame");
        assert!(!flame.children.is_empty());
        assert!(flame.total_time() > 0.0);
    }
}
