//! Companion route across a span in the Aravath greybox.
//!
//! The window does not draw this body yet. The step is the same script as
//! the `nav_reroute` replay: the short path uses the bridge, and a collapse
//! at tick 30 sends the route onto the ford.

use astraweave_nav::{bridge_and_ford, nav_change_from_kind, NavChangeKind, NavScratch, NavTiles};
use glam::Vec3;

/// Player-facing name for this scene.
pub const SCENE_LABEL: &str = "Aravath";

pub struct BridgeAgent {
    tiles: NavTiles,
    scratch: NavScratch,
    /// Highest waypoint x, in milli-units, on the current route.
    pub route_x_milli: i32,
}

impl BridgeAgent {
    #[must_use]
    pub fn new() -> Self {
        let tiles = bridge_and_ford();
        let mut agent = Self {
            tiles,
            scratch: NavScratch::new(),
            route_x_milli: 0,
        };
        agent.refresh();
        agent
    }

    pub fn step(&mut self, tick: u64) {
        if tick == 30 {
            self.tiles.mark(nav_change_from_kind(
                NavChangeKind::Detach,
                tick,
                0,
                [-1_000, -1_000, 17_000],
                [5_000, 1_000, 31_999],
            ));
        }
        let _committed = self.tiles.commit_tick();
        self.refresh();
    }

    fn refresh(&mut self) {
        let _count = self.tiles.query_path(
            &mut self.scratch,
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 46.0),
        );
        let mut max_x = i32::MIN;
        for point in self
            .scratch
            .waypoints
            .iter()
            .take(self.scratch.waypoint_len as usize)
        {
            max_x = max_x.max((point.x * 1000.0) as i32);
        }
        self.route_x_milli = max_x;
    }
}

impl Default for BridgeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_sends_the_agent_to_the_ford() {
        assert_eq!(SCENE_LABEL, "Aravath");
        let mut agent = BridgeAgent::new();
        assert!(
            agent.route_x_milli < 8_000,
            "bridge route x {}",
            agent.route_x_milli
        );
        for tick in 0..40 {
            agent.step(tick);
        }
        assert!(
            agent.route_x_milli > 16_000,
            "ford route x {}",
            agent.route_x_milli
        );
    }
}
