//! Bar chart widget for categorical and grouped data visualization.

use crate::charts::{calculate_nice_bounds, AxisConfig, ChartStyle};
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// A single bar in a bar chart.
#[derive(Clone, Debug)]
pub struct Bar {
    pub label: String,
    pub value: f64,
    pub color: Color32,
}

/// A group of bars (for grouped/stacked bar charts).
#[derive(Clone, Debug)]
pub struct BarGroup {
    pub category: String,
    pub bars: Vec<Bar>,
}

/// Bar chart display mode.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum BarChartMode {
    /// Bars side-by-side
    Grouped,
    /// Bars stacked on top of each other
    Stacked,
}

struct BarDrawParams {
    bar_x: f32,
    bar_width: f32,
    value: f64,
    color: Color32,
    min_y: f64,
    max_y: f64,
}

/// A bar chart widget for visualizing categorical data.
///
/// # Example
///
/// ```rust,ignore
/// use astract::charts::{BarChart, BarGroup, Bar};
/// use egui::Color32;
///
/// let mut chart = BarChart::new("Entity Counts");
/// chart.add_group(BarGroup {
///     category: "Scene 1".to_string(),
///     bars: vec![
///         Bar { label: "Enemies".to_string(), value: 15.0, color: Color32::RED },
///         Bar { label: "Allies".to_string(), value: 5.0, color: Color32::GREEN },
///     ],
/// });
///
/// // In your egui UI:
/// chart.show(ui);
/// ```
pub struct BarChart {
    title: String,
    groups: Vec<BarGroup>,
    y_axis: AxisConfig,
    style: ChartStyle,
    height: f32,
    mode: BarChartMode,
    bar_width_ratio: f32,
    show_values: bool,
}

impl BarChart {
    /// Create a new bar chart with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            groups: Vec::new(),
            y_axis: AxisConfig::new("Value"),
            style: ChartStyle::default(),
            height: 200.0,
            mode: BarChartMode::Grouped,
            bar_width_ratio: 0.8,
            show_values: true,
        }
    }

    /// Set the chart height in pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set the display mode (Grouped or Stacked).
    pub fn mode(mut self, mode: BarChartMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set bar width ratio (0.0-1.0, relative to available space).
    pub fn bar_width_ratio(mut self, ratio: f32) -> Self {
        self.bar_width_ratio = ratio.clamp(0.1, 1.0);
        self
    }

    /// Show/hide value labels on bars.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Set Y axis configuration.
    pub fn y_axis(mut self, config: AxisConfig) -> Self {
        self.y_axis = config;
        self
    }

    /// Add a group of bars to the chart.
    pub fn add_group(&mut self, group: BarGroup) {
        self.groups.push(group);
    }

    /// Clear all groups.
    pub fn clear(&mut self) {
        self.groups.clear();
    }

    /// Calculate the maximum value across all bars.
    fn calculate_max_value(&self) -> f64 {
        let mut max_value: f64 = 0.0;

        for group in &self.groups {
            match self.mode {
                BarChartMode::Grouped => {
                    for bar in &group.bars {
                        max_value = max_value.max(bar.value);
                    }
                }
                BarChartMode::Stacked => {
                    let group_sum: f64 = group.bars.iter().map(|b| b.value).sum();
                    max_value = max_value.max(group_sum);
                }
            }
        }

        max_value
    }

    /// Render the chart.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), self.height), Sense::hover());

        let chart_rect = response.rect;

        // Background
        painter.rect_filled(chart_rect, 2.0, self.style.background_color);

        // Title
        let title_pos = chart_rect.min + Vec2::new(10.0, 5.0);
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            &self.title,
            egui::FontId::proportional(14.0),
            self.style.text_color,
        );

        if self.groups.is_empty() {
            painter.text(
                chart_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No data",
                egui::FontId::proportional(12.0),
                self.style.text_color,
            );
            return response;
        }

        // Calculate data area
        let margin_left = 50.0;
        let margin_bottom = 40.0;
        let legend_width = if self.style.show_legend { 120.0 } else { 0.0 };
        let data_rect = Rect::from_min_max(
            chart_rect.min + Vec2::new(margin_left, 30.0),
            chart_rect.max - Vec2::new(20.0 + legend_width, margin_bottom),
        );

        // Calculate Y axis bounds
        let max_value = self.calculate_max_value();
        let (min_y, max_y) = if self.y_axis.auto_scale {
            calculate_nice_bounds(0.0, max_value)
        } else {
            (
                self.y_axis.min.unwrap_or(0.0),
                self.y_axis.max.unwrap_or(max_value),
            )
        };

        // Draw grid
        if self.style.show_grid {
            self.draw_grid(&painter, data_rect, min_y, max_y);
        }

        // Draw axes
        if self.style.show_axes {
            self.draw_axes(&painter, data_rect, min_y, max_y);
        }

        // Draw bars
        self.draw_bars(&painter, data_rect, min_y, max_y);

        // Draw category labels
        self.draw_category_labels(&painter, data_rect);

        // Draw legend
        if self.style.show_legend {
            self.draw_legend(&painter, chart_rect);
        }

        // Y axis label
        let y_label_pos = Pos2::new(chart_rect.min.x + 10.0, data_rect.center().y);
        painter.text(
            y_label_pos,
            egui::Align2::LEFT_CENTER,
            &self.y_axis.label,
            egui::FontId::proportional(10.0),
            self.style.text_color,
        );

        response
    }

    fn draw_grid(&self, painter: &egui::Painter, data_rect: Rect, _min_y: f64, _max_y: f64) {
        let num_lines = 5;
        for i in 0..=num_lines {
            let t = i as f32 / num_lines as f32;
            let y = data_rect.max.y - t * data_rect.height();

            painter.line_segment(
                [Pos2::new(data_rect.min.x, y), Pos2::new(data_rect.max.x, y)],
                Stroke::new(1.0, self.style.grid_color),
            );
        }
    }

    fn draw_axes(&self, painter: &egui::Painter, data_rect: Rect, min_y: f64, max_y: f64) {
        // X axis
        painter.line_segment(
            [
                data_rect.min + Vec2::new(0.0, data_rect.height()),
                data_rect.max,
            ],
            Stroke::new(1.5, self.style.axis_color),
        );

        // Y axis
        painter.line_segment(
            [
                data_rect.min,
                data_rect.min + Vec2::new(0.0, data_rect.height()),
            ],
            Stroke::new(1.5, self.style.axis_color),
        );

        // Y tick labels
        let num_ticks = 5;
        for i in 0..=num_ticks {
            let t = i as f64 / num_ticks as f64;
            let y_val = min_y + t * (max_y - min_y);
            let y_screen = data_rect.max.y - (t as f32) * data_rect.height();

            painter.text(
                Pos2::new(data_rect.min.x - 5.0, y_screen),
                egui::Align2::RIGHT_CENTER,
                format!("{:.1}", y_val),
                egui::FontId::proportional(9.0),
                self.style.text_color,
            );
        }
    }

    fn draw_bars(&self, painter: &egui::Painter, data_rect: Rect, min_y: f64, max_y: f64) {
        let num_groups = self.groups.len() as f32;
        let group_width = data_rect.width() / num_groups;

        for (group_idx, group) in self.groups.iter().enumerate() {
            let group_x = data_rect.min.x + group_idx as f32 * group_width;

            match self.mode {
                BarChartMode::Grouped => {
                    let num_bars = group.bars.len() as f32;
                    let bar_width = (group_width * self.bar_width_ratio) / num_bars;

                    for (bar_idx, bar) in group.bars.iter().enumerate() {
                        let bar_x = group_x + bar_idx as f32 * bar_width;
                        self.draw_single_bar(
                            painter,
                            data_rect,
                            BarDrawParams {
                                bar_x,
                                bar_width,
                                value: bar.value,
                                color: bar.color,
                                min_y,
                                max_y,
                            },
                        );
                    }
                }
                BarChartMode::Stacked => {
                    let bar_width = group_width * self.bar_width_ratio;
                    let bar_x = group_x + (group_width - bar_width) / 2.0;
                    let mut y_offset = 0.0;

                    for bar in &group.bars {
                        let bar_height_ratio = ((bar.value / (max_y - min_y)) as f32).min(1.0);
                        let bar_height = bar_height_ratio * data_rect.height();

                        let bar_rect = Rect::from_min_size(
                            Pos2::new(bar_x, data_rect.max.y - y_offset - bar_height),
                            Vec2::new(bar_width, bar_height),
                        );

                        painter.rect_filled(bar_rect, 2.0, bar.color);
                        painter.rect_stroke(
                            bar_rect,
                            2.0,
                            (1.0, Color32::BLACK),
                            egui::StrokeKind::Middle,
                        );

                        y_offset += bar_height;
                    }
                }
            }
        }
    }

    fn draw_single_bar(&self, painter: &egui::Painter, data_rect: Rect, params: BarDrawParams) {
        let value_ratio = ((params.value - params.min_y) / (params.max_y - params.min_y)) as f32;
        let bar_height = value_ratio.clamp(0.0, 1.0) * data_rect.height();

        let bar_rect = Rect::from_min_size(
            Pos2::new(params.bar_x, data_rect.max.y - bar_height),
            Vec2::new(params.bar_width, bar_height),
        );

        painter.rect_filled(bar_rect, 2.0, params.color);
        painter.rect_stroke(
            bar_rect,
            2.0,
            (1.0, Color32::BLACK),
            egui::StrokeKind::Middle,
        );

        // Value label
        if self.show_values && bar_height > 15.0 {
            painter.text(
                Pos2::new(params.bar_x + params.bar_width / 2.0, bar_rect.min.y - 5.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{:.1}", params.value),
                egui::FontId::proportional(9.0),
                self.style.text_color,
            );
        }
    }

    fn draw_category_labels(&self, painter: &egui::Painter, data_rect: Rect) {
        let num_groups = self.groups.len() as f32;
        let group_width = data_rect.width() / num_groups;

        for (idx, group) in self.groups.iter().enumerate() {
            let label_x = data_rect.min.x + (idx as f32 + 0.5) * group_width;
            let label_y = data_rect.max.y + 5.0;

            painter.text(
                Pos2::new(label_x, label_y),
                egui::Align2::CENTER_TOP,
                &group.category,
                egui::FontId::proportional(9.0),
                self.style.text_color,
            );
        }
    }

    fn draw_legend(&self, painter: &egui::Painter, chart_rect: Rect) {
        // Collect unique bar labels
        let mut labels: Vec<(String, Color32)> = Vec::new();
        for group in &self.groups {
            for bar in &group.bars {
                if !labels.iter().any(|(l, _)| l == &bar.label) {
                    labels.push((bar.label.clone(), bar.color));
                }
            }
        }

        let legend_x = chart_rect.max.x - 110.0;
        let mut legend_y = chart_rect.min.y + 40.0;

        for (label, color) in labels {
            // Color box
            let box_rect =
                Rect::from_min_size(Pos2::new(legend_x, legend_y - 6.0), Vec2::new(12.0, 12.0));
            painter.rect_filled(box_rect, 2.0, color);
            painter.rect_stroke(
                box_rect,
                2.0,
                (1.0, self.style.text_color),
                egui::StrokeKind::Middle,
            );

            // Label
            painter.text(
                Pos2::new(legend_x + 18.0, legend_y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(10.0),
                self.style.text_color,
            );

            legend_y += 18.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_chart_creation() {
        let mut chart = BarChart::new("Test Chart");
        chart.add_group(BarGroup {
            category: "A".to_string(),
            bars: vec![Bar {
                label: "B1".to_string(),
                value: 10.0,
                color: Color32::RED,
            }],
        });

        assert_eq!(chart.groups.len(), 1);
    }

    #[test]
    fn test_max_value_grouped() {
        let mut chart = BarChart::new("Test").mode(BarChartMode::Grouped);
        chart.add_group(BarGroup {
            category: "A".to_string(),
            bars: vec![
                Bar {
                    label: "B1".to_string(),
                    value: 10.0,
                    color: Color32::RED,
                },
                Bar {
                    label: "B2".to_string(),
                    value: 25.0,
                    color: Color32::BLUE,
                },
            ],
        });

        assert_eq!(chart.calculate_max_value(), 25.0);
    }

    #[test]
    fn test_max_value_stacked() {
        let mut chart = BarChart::new("Test").mode(BarChartMode::Stacked);
        chart.add_group(BarGroup {
            category: "A".to_string(),
            bars: vec![
                Bar {
                    label: "B1".to_string(),
                    value: 10.0,
                    color: Color32::RED,
                },
                Bar {
                    label: "B2".to_string(),
                    value: 15.0,
                    color: Color32::BLUE,
                },
            ],
        });

        assert_eq!(chart.calculate_max_value(), 25.0); // Sum
    }
}
