//! Scripted traversal course for the Aravath greybox.
//!
//! The voxel island is unchanged. This course is a deterministic probe script
//! the sim can run without a GPU: a step, a vault, a mantle, a climb that
//! prefers a handhold, a non-climbable puzzle volume, a pool, a slide, a
//! crawl, a wall-run, and a glide. Geometry flags live on the probes.

use astraweave_physics::traversal::{
    step, ProbeFrame, TraversalBody, TraversalInput, TraversalMode, CLIMB_SPEED,
};
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CourseBeat {
    Step,
    Vault,
    Mantle,
    ClimbMarkup,
    PuzzleWall,
    Swim,
    Slide,
    Crawl,
    WallRun,
    Glide,
    Ledge,
    Shimmy,
}

pub fn run_course() -> Vec<CourseBeat> {
    let mut seen = Vec::new();
    let mut body = TraversalBody::default();

    let mut step_probe = ProbeFrame::plane(0.0);
    step_probe.step_height = Some(0.25);
    step(
        &mut body,
        TraversalInput {
            move_y: 1.0,
            ..TraversalInput::default()
        },
        &step_probe,
        0,
    );
    if (body.pos.y - 0.25).abs() < 1e-2 {
        seen.push(CourseBeat::Step);
    }

    body = TraversalBody::default();
    let mut vault = ProbeFrame::plane(0.0);
    vault.vault_to = Some(Vec3::new(1.0, 0.6, 0.0));
    step(
        &mut body,
        TraversalInput {
            move_y: 1.0,
            ..TraversalInput::default()
        },
        &vault,
        1,
    );
    if body.mode == TraversalMode::Vault {
        seen.push(CourseBeat::Vault);
    }

    body = TraversalBody::default();
    let mut mantle = ProbeFrame::plane(0.0);
    mantle.mantle_to = Some(Vec3::new(0.5, 1.5, 0.0));
    step(
        &mut body,
        TraversalInput {
            move_y: 1.0,
            ..TraversalInput::default()
        },
        &mantle,
        2,
    );
    if body.mode == TraversalMode::Mantle {
        seen.push(CourseBeat::Mantle);
    }

    body = TraversalBody::default();
    let mut climb = ProbeFrame::plane(0.0);
    climb.climb_point = Some(Vec3::new(0.0, 1.0, 0.4));
    climb.markup_point = Some(Vec3::new(0.0, 1.6, 0.0));
    climb.climb_normal = Some(Vec3::new(0.0, 0.1, 1.0));
    step(
        &mut body,
        TraversalInput {
            climb: true,
            ..TraversalInput::default()
        },
        &climb,
        3,
    );
    if body.mode == TraversalMode::Climb && (body.pos.y - 1.6).abs() < 1e-3 {
        seen.push(CourseBeat::ClimbMarkup);
    }
    climb.ledge_point = Some(Vec3::new(0.2, 2.0, 0.0));
    step(&mut body, TraversalInput::default(), &climb, 11);
    if body.mode == TraversalMode::LedgeHang {
        seen.push(CourseBeat::Ledge);
    }
    step(
        &mut body,
        TraversalInput {
            move_x: 1.0,
            ..TraversalInput::default()
        },
        &climb,
        12,
    );
    if body.mode == TraversalMode::Shimmy {
        seen.push(CourseBeat::Shimmy);
    }
    let _ = CLIMB_SPEED;

    body = TraversalBody::default();
    climb.climbability = astraweave_physics::traversal::Climbability::NonClimbable;
    climb.markup_point = None;
    step(
        &mut body,
        TraversalInput {
            climb: true,
            ..TraversalInput::default()
        },
        &climb,
        4,
    );
    if body.mode != TraversalMode::Climb {
        seen.push(CourseBeat::PuzzleWall);
    }

    body = TraversalBody::default();
    let mut pool = ProbeFrame::plane(0.0);
    pool.water_surface = Some(1.4);
    step(&mut body, TraversalInput::default(), &pool, 5);
    step(&mut body, TraversalInput::default(), &pool, 6);
    if body.mode == TraversalMode::Swim {
        seen.push(CourseBeat::Swim);
    }

    body = TraversalBody::default();
    step(
        &mut body,
        TraversalInput {
            move_y: 1.0,
            sprint: true,
            crouch: true,
            ..TraversalInput::default()
        },
        &ProbeFrame::plane(0.0),
        7,
    );
    if body.mode == TraversalMode::Slide {
        seen.push(CourseBeat::Slide);
    }

    body = TraversalBody::default();
    step(
        &mut body,
        TraversalInput {
            prone: true,
            ..TraversalInput::default()
        },
        &ProbeFrame::plane(0.0),
        8,
    );
    if body.mode == TraversalMode::Prone {
        seen.push(CourseBeat::Crawl);
    }

    body = TraversalBody::default();
    body.mode = TraversalMode::Air;
    body.vel = Vec3::new(6.0, 1.0, 0.0);
    body.pos.y = 2.0;
    let mut wall = ProbeFrame::plane(0.0);
    wall.support = false;
    wall.wall_normal = Some(Vec3::new(1.0, 0.0, 0.0));
    step(
        &mut body,
        TraversalInput {
            move_y: 1.0,
            ..TraversalInput::default()
        },
        &wall,
        9,
    );
    if body.mode == TraversalMode::WallRun {
        seen.push(CourseBeat::WallRun);
    }

    body = TraversalBody::default();
    body.mode = TraversalMode::Air;
    body.pos.y = 6.0;
    let mut air = ProbeFrame::plane(0.0);
    air.support = false;
    step(
        &mut body,
        TraversalInput {
            glide: true,
            move_y: 1.0,
            ..TraversalInput::default()
        },
        &air,
        10,
    );
    if body.mode == TraversalMode::Glide {
        seen.push(CourseBeat::Glide);
    }

    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn course_hits_every_beat() {
        let beats = run_course();
        let expected = [
            CourseBeat::Step,
            CourseBeat::Vault,
            CourseBeat::Mantle,
            CourseBeat::ClimbMarkup,
            CourseBeat::Ledge,
            CourseBeat::Shimmy,
            CourseBeat::PuzzleWall,
            CourseBeat::Swim,
            CourseBeat::Slide,
            CourseBeat::Crawl,
            CourseBeat::WallRun,
            CourseBeat::Glide,
        ];
        assert_eq!(beats, expected);
    }
}
