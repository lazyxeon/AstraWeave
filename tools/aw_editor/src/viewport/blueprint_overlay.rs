//! Blueprint Overlay — 3D wireframe zone visualization for the viewport
//!
//! Generates debug lines for zone polygon edges, vertex dots, and dashed outlines.
//! Integrates with the existing physics debug line renderer in the viewport pipeline.

use astraweave_physics::DebugLine;
use glam::Vec2;

/// Configuration for a zone to be rendered as a 3D overlay.
#[derive(Debug, Clone)]
pub struct ZoneOverlayData {
    /// Polygon vertices in world XZ coordinates.
    pub vertices: Vec<Vec2>,
    /// Color for this zone (RGB 0..1).
    pub color: [f32; 3],
    /// Whether this zone is currently selected (thicker/brighter).
    pub selected: bool,
    /// Whether this zone is being edited (dashed outline).
    pub editing: bool,
    /// Y height to draw at (terrain sample or default).
    pub y_height: f32,
}

/// Generates debug lines for rendering zone overlays in the 3D viewport.
pub struct BlueprintOverlay;

impl BlueprintOverlay {
    /// Generate all debug lines for a set of zones.
    pub fn generate_lines(zones: &[ZoneOverlayData]) -> Vec<DebugLine> {
        let mut lines = Vec::new();
        for zone in zones {
            if zone.vertices.len() < 2 {
                continue;
            }
            Self::generate_zone_lines(zone, &mut lines);
        }
        lines
    }

    fn generate_zone_lines(zone: &ZoneOverlayData, lines: &mut Vec<DebugLine>) {
        let n = zone.vertices.len();
        let y = zone.y_height;
        let color = if zone.selected {
            [1.0, 1.0, 0.3] // yellow highlight for selected
        } else {
            zone.color
        };

        if zone.editing {
            // Dashed edges
            for i in 0..n {
                let j = (i + 1) % n;
                let a = zone.vertices[i];
                let b = zone.vertices[j];
                Self::dashed_line(
                    [a.x, y, a.y],
                    [b.x, y, b.y],
                    color,
                    2.0, // dash length
                    1.0, // gap length
                    lines,
                );
            }
        } else {
            // Solid edges
            for i in 0..n {
                let j = (i + 1) % n;
                let a = zone.vertices[i];
                let b = zone.vertices[j];
                lines.push(DebugLine::new([a.x, y, a.y], [b.x, y, b.y], color));
            }

            // Close the polygon (connect last to first)
            // Already done by `(i + 1) % n`
        }

        // Vertex indicators (small cross at each vertex)
        let cross_size = if zone.selected { 1.5 } else { 0.8 };
        for v in &zone.vertices {
            // X-axis cross arm
            lines.push(DebugLine::new(
                [v.x - cross_size, y, v.y],
                [v.x + cross_size, y, v.y],
                color,
            ));
            // Z-axis cross arm
            lines.push(DebugLine::new(
                [v.x, y, v.y - cross_size],
                [v.x, y, v.y + cross_size],
                color,
            ));
            // Vertical post (small)
            lines.push(DebugLine::new(
                [v.x, y, v.y],
                [v.x, y + cross_size, v.y],
                color,
            ));
        }

        // Centroid label indicator (vertical line at centroid)
        if n >= 3 {
            let cx = zone.vertices.iter().map(|v| v.x).sum::<f32>() / n as f32;
            let cz = zone.vertices.iter().map(|v| v.y).sum::<f32>() / n as f32;
            let label_color = [1.0, 1.0, 1.0]; // white
            lines.push(DebugLine::new(
                [cx, y, cz],
                [cx, y + 3.0, cz],
                label_color,
            ));
        }
    }

    /// Generate a dashed line between two 3D points.
    fn dashed_line(
        start: [f32; 3],
        end: [f32; 3],
        color: [f32; 3],
        dash_len: f32,
        gap_len: f32,
        lines: &mut Vec<DebugLine>,
    ) {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let total_len = (dx * dx + dy * dy + dz * dz).sqrt();
        if total_len < 0.01 {
            return;
        }

        let dir = [dx / total_len, dy / total_len, dz / total_len];
        let segment = dash_len + gap_len;
        let mut t = 0.0;

        while t < total_len {
            let t_end = (t + dash_len).min(total_len);
            let p0 = [
                start[0] + dir[0] * t,
                start[1] + dir[1] * t,
                start[2] + dir[2] * t,
            ];
            let p1 = [
                start[0] + dir[0] * t_end,
                start[1] + dir[1] * t_end,
                start[2] + dir[2] * t_end,
            ];
            lines.push(DebugLine::new(p0, p1, color));
            t += segment;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_zones() {
        let lines = BlueprintOverlay::generate_lines(&[]);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_single_edge_zone() {
        let zone = ZoneOverlayData {
            vertices: vec![Vec2::ZERO, Vec2::new(10.0, 0.0)],
            color: [0.0, 1.0, 0.0],
            selected: false,
            editing: false,
            y_height: 0.0,
        };
        let lines = BlueprintOverlay::generate_lines(&[zone]);
        // 2 edges (0→1, 1→0 same as close) + 2 vertices * 3 lines each = 2 + 6 = 8
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_triangle_zone_solid() {
        let zone = ZoneOverlayData {
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(5.0, 10.0),
            ],
            color: [0.0, 1.0, 0.0],
            selected: false,
            editing: false,
            y_height: 5.0,
        };
        let lines = BlueprintOverlay::generate_lines(&[zone]);
        // 3 edges + 3 vertices * 3 + 1 centroid line = 3 + 9 + 1 = 13
        assert_eq!(lines.len(), 13);

        // All lines at y_height=5.0
        for line in &lines {
            assert!(line.start[1] >= 5.0);
        }
    }

    #[test]
    fn test_selected_zone_color() {
        let zone = ZoneOverlayData {
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(5.0, 10.0),
            ],
            color: [0.0, 0.5, 0.0],
            selected: true,
            editing: false,
            y_height: 0.0,
        };
        let lines = BlueprintOverlay::generate_lines(&[zone]);
        // Selected zones use yellow [1.0, 1.0, 0.3] for edges and vertices
        let edge = &lines[0];
        assert!((edge.color[0] - 1.0).abs() < 0.01);
        assert!((edge.color[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dashed_line_editing() {
        let zone = ZoneOverlayData {
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(5.0, 10.0),
            ],
            color: [0.0, 0.0, 1.0],
            selected: false,
            editing: true,
            y_height: 0.0,
        };
        let lines_editing = BlueprintOverlay::generate_lines(&[zone.clone()]);

        let mut solid_zone = zone;
        solid_zone.editing = false;
        let lines_solid = BlueprintOverlay::generate_lines(&[solid_zone]);

        // Dashed lines produce more segments than solid
        assert!(lines_editing.len() >= lines_solid.len());
    }

    #[test]
    fn test_multiple_zones() {
        let z1 = ZoneOverlayData {
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
            ],
            color: [0.0, 1.0, 0.0],
            selected: false,
            editing: false,
            y_height: 0.0,
        };
        let z2 = ZoneOverlayData {
            vertices: vec![
                Vec2::new(20.0, 0.0),
                Vec2::new(30.0, 0.0),
                Vec2::new(25.0, 10.0),
            ],
            color: [0.0, 0.0, 1.0],
            selected: false,
            editing: false,
            y_height: 2.0,
        };
        let lines = BlueprintOverlay::generate_lines(&[z1, z2]);
        // Each triangle: 3 edges + 9 vertex + 1 centroid = 13, total 26
        assert_eq!(lines.len(), 26);
    }

    #[test]
    fn test_too_few_vertices_skipped() {
        let zone = ZoneOverlayData {
            vertices: vec![Vec2::ZERO],
            color: [1.0, 0.0, 0.0],
            selected: false,
            editing: false,
            y_height: 0.0,
        };
        let lines = BlueprintOverlay::generate_lines(&[zone]);
        assert!(lines.is_empty());
    }
}
