// astract/src/widgets/performance_budget.rs - Live performance budgeting widget
//
// Integrates with Week 8 Tracy profiling infrastructure to show real-time
// frame budget breakdown (ECS, Physics, Rendering, AI, Audio, UI).
//
// Usage:
//   let mut widget = PerformanceBudgetWidget::new();
//   widget.update_from_frame_time(frame_time_ms);
//   widget.show(ui);

use std::collections::VecDeque;

/// Frame budget model (60 FPS = 16.67ms per frame)
#[derive(Debug, Clone)]
pub struct FrameBudget {
    pub ecs: f32,       // 2.7ms budget
    pub physics: f32,   // 3.0ms budget
    pub rendering: f32, // 8.0ms budget
    pub ai: f32,        // 1.0ms budget
    pub audio: f32,     // 0.5ms budget
    pub ui: f32,        // 0.5ms budget
    pub headroom: f32,  // 1.0ms buffer
}

impl FrameBudget {
    pub const TARGET_FPS: f32 = 60.0;
    pub const FRAME_TIME_MS: f32 = 16.67; // 1000ms / 60fps

    /// Create empty budget (all zeros)
    pub fn zero() -> Self {
        Self {
            ecs: 0.0,
            physics: 0.0,
            rendering: 0.0,
            ai: 0.0,
            audio: 0.0,
            ui: 0.0,
            headroom: 0.0,
        }
    }
}

impl Default for FrameBudget {
    /// Create default budget allocation
    fn default() -> Self {
        Self {
            ecs: 2.7,
            physics: 3.0,
            rendering: 8.0,
            ai: 1.0,
            audio: 0.5,
            ui: 0.5,
            headroom: 1.0,
        }
    }
}

impl FrameBudget {
    /// Total time used (excluding headroom)
    pub fn total_used(&self) -> f32 {
        self.ecs + self.physics + self.rendering + self.ai + self.audio + self.ui
    }

    /// Percentage of frame budget used
    pub fn percent_used(&self) -> f32 {
        (self.total_used() / Self::FRAME_TIME_MS) * 100.0
    }

    /// Check if over budget (including headroom)
    pub fn is_over_budget(&self) -> bool {
        self.total_used() > (Self::FRAME_TIME_MS - self.headroom)
    }

    /// Get per-category budget allocation
    pub fn get_budget(&self, category: &str) -> f32 {
        match category {
            "ECS" => 2.7,
            "Physics" => 3.0,
            "Rendering" => 8.0,
            "AI" => 1.0,
            "Audio" => 0.5,
            "UI" => 0.5,
            _ => 0.0,
        }
    }

    /// Get per-category actual time
    pub fn get_actual(&self, category: &str) -> f32 {
        match category {
            "ECS" => self.ecs,
            "Physics" => self.physics,
            "Rendering" => self.rendering,
            "AI" => self.ai,
            "Audio" => self.audio,
            "UI" => self.ui,
            _ => 0.0,
        }
    }
}

/// Performance budget widget with history tracking
pub struct PerformanceBudgetWidget {
    current: FrameBudget,
    history: VecDeque<FrameBudget>,
    max_history: usize,
    expanded: bool,
}

impl PerformanceBudgetWidget {
    /// Create new widget with default settings
    pub fn new() -> Self {
        Self {
            current: FrameBudget::zero(),
            history: VecDeque::new(),
            max_history: 60, // Last 60 frames (1 second @ 60 FPS)
            expanded: false,
        }
    }

    /// Update from frame timings (manual entry for now, Tracy integration later)
    pub fn update_from_frame_time(&mut self, frame_time_ms: f32) {
        // Estimate breakdown (Week 8 proportions)
        // In production, this would come from Tracy profiling zones
        let total = frame_time_ms;

        self.current = FrameBudget {
            ecs: total * 0.162,       // ~16.2% (2.7ms / 16.67ms)
            physics: total * 0.180,   // ~18.0% (3.0ms / 16.67ms)
            rendering: total * 0.480, // ~48.0% (8.0ms / 16.67ms)
            ai: total * 0.060,        // ~6.0% (1.0ms / 16.67ms)
            audio: total * 0.030,     // ~3.0% (0.5ms / 16.67ms)
            ui: total * 0.030,        // ~3.0% (0.5ms / 16.67ms)
            headroom: 1.0,
        };

        // Store in history
        self.history.push_back(self.current.clone());
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Update from explicit category timings (for testing or Tracy integration)
    pub fn update_from_categories(
        &mut self,
        ecs: f32,
        physics: f32,
        rendering: f32,
        ai: f32,
        audio: f32,
        ui: f32,
    ) {
        self.current = FrameBudget {
            ecs,
            physics,
            rendering,
            ai,
            audio,
            ui,
            headroom: 1.0,
        };

        self.history.push_back(self.current.clone());
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Show widget in egui UI (compact + optional expanded view)
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Use smoothed average over last 30 frames to reduce strobe effect
        let display_budget = self.average(30);

        ui.horizontal(|ui| {
            // Compact view (always visible)
            let percent = display_budget.percent_used();
            let color = if display_budget.is_over_budget() {
                egui::Color32::RED
            } else if percent > 80.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };

            ui.colored_label(color, format!("{:.1}ms", display_budget.total_used()));
            ui.label("/");
            ui.label(format!("{:.1}ms", FrameBudget::FRAME_TIME_MS));

            // Progress bar
            let progress = (percent / 100.0).min(1.0);
            let progress_bar = egui::ProgressBar::new(progress)
                .fill(color)
                .show_percentage();
            ui.add(progress_bar.desired_width(100.0));
        });

        // Expandable breakdown
        let response = ui.collapsing("Budget Breakdown", |ui| {
            egui::Grid::new("budget_grid").striped(true).show(ui, |ui| {
                ui.label("Category");
                ui.label("Time (ms)");
                ui.label("Budget");
                ui.label("% Used");
                ui.end_row();

                self.show_category(ui, "ECS", display_budget.ecs, 2.7);
                self.show_category(ui, "Physics", display_budget.physics, 3.0);
                self.show_category(ui, "Rendering", display_budget.rendering, 8.0);
                self.show_category(ui, "AI", display_budget.ai, 1.0);
                self.show_category(ui, "Audio", display_budget.audio, 0.5);
                self.show_category(ui, "UI", display_budget.ui, 0.5);
            });
        });

        self.expanded = response.body_returned.is_some();
    }

    /// Show single category row in grid
    fn show_category(&self, ui: &mut egui::Ui, name: &str, actual: f32, budget: f32) {
        ui.label(name);
        ui.label(format!("{:.2}", actual));
        ui.label(format!("{:.2}", budget));

        let percent = if budget > 0.0 {
            (actual / budget) * 100.0
        } else {
            0.0
        };

        let color = if percent > 100.0 {
            egui::Color32::RED
        } else if percent > 80.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::GREEN
        };

        ui.colored_label(color, format!("{:.1}%", percent));
        ui.end_row();
    }

    /// Get current frame budget
    pub fn current(&self) -> &FrameBudget {
        &self.current
    }

    /// Get average over last N frames
    pub fn average(&self, n: usize) -> FrameBudget {
        if self.history.is_empty() {
            return FrameBudget::zero();
        }

        let count = self.history.len().min(n);
        let recent: Vec<&FrameBudget> = self.history.iter().rev().take(count).collect();

        let mut sum = FrameBudget::zero();
        for budget in recent {
            sum.ecs += budget.ecs;
            sum.physics += budget.physics;
            sum.rendering += budget.rendering;
            sum.ai += budget.ai;
            sum.audio += budget.audio;
            sum.ui += budget.ui;
        }

        let count_f32 = count as f32;
        FrameBudget {
            ecs: sum.ecs / count_f32,
            physics: sum.physics / count_f32,
            rendering: sum.rendering / count_f32,
            ai: sum.ai / count_f32,
            audio: sum.audio / count_f32,
            ui: sum.ui / count_f32,
            headroom: 1.0,
        }
    }
}

impl Default for PerformanceBudgetWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_budget_calculations() {
        let budget = FrameBudget {
            ecs: 2.5,
            physics: 2.8,
            rendering: 7.2,
            ai: 0.8,
            audio: 0.3,
            ui: 0.4,
            headroom: 1.0,
        };

        assert_eq!(budget.total_used(), 14.0);
        assert!((budget.percent_used() - 84.0).abs() < 0.1); // ~84%
        assert!(!budget.is_over_budget()); // 14.0 < 15.67
    }

    #[test]
    fn test_over_budget_detection() {
        let budget = FrameBudget {
            ecs: 3.0,
            physics: 4.0,
            rendering: 9.0,
            ai: 1.5,
            audio: 0.5,
            ui: 0.5,
            headroom: 1.0,
        };

        assert!(budget.is_over_budget()); // 18.5 > 15.67
    }

    #[test]
    fn test_color_coding_green() {
        let widget = PerformanceBudgetWidget {
            current: FrameBudget {
                ecs: 2.0,
                physics: 2.5,
                rendering: 6.0,
                ai: 0.5,
                audio: 0.2,
                ui: 0.3,
                headroom: 1.0,
            },
            history: VecDeque::new(),
            max_history: 60,
            expanded: false,
        };

        // 11.5ms / 16.67ms = 69% (should be green)
        assert!(widget.current.percent_used() < 80.0);
        assert!(!widget.current.is_over_budget());
    }

    #[test]
    fn test_widget_history() {
        let mut widget = PerformanceBudgetWidget::new();

        widget.update_from_frame_time(14.0);
        widget.update_from_frame_time(15.0);
        widget.update_from_frame_time(16.0);

        assert_eq!(widget.history.len(), 3);

        // Test that history stores values
        assert!(widget.history[0].total_used() > 0.0);
        assert!(widget.history[1].total_used() > 0.0);
        assert!(widget.history[2].total_used() > 0.0);

        // Test average calculation (proportions sum to ~94.2% of input)
        let avg = widget.average(3);
        assert!(avg.total_used() > 10.0 && avg.total_used() < 20.0);
    }

    #[test]
    fn test_category_getters() {
        let budget = FrameBudget::default();

        assert_eq!(budget.get_budget("ECS"), 2.7);
        assert_eq!(budget.get_budget("Physics"), 3.0);
        assert_eq!(budget.get_budget("Rendering"), 8.0);
        assert_eq!(budget.get_budget("AI"), 1.0);
        assert_eq!(budget.get_budget("Audio"), 0.5);
        assert_eq!(budget.get_budget("UI"), 0.5);
        assert_eq!(budget.get_budget("Unknown"), 0.0);
    }
}
