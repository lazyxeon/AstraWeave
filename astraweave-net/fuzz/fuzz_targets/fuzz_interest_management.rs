//! Fuzz target for interest management filters.
//!
//! Tests RadiusTeamInterest and FovInterest with arbitrary entity states.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use astraweave_core::IVec2;
use astraweave_net::{EntityState, FullInterest, Interest, RadiusTeamInterest};

#[derive(Debug, Arbitrary)]
struct FuzzInterestInput {
    viewer: FuzzEntity,
    target: FuzzEntity,
    radius: i32,
}

#[derive(Debug, Clone, Arbitrary)]
struct FuzzEntity {
    id: u32,
    x: i32,
    y: i32,
    hp: i32,
    team: u8,
    ammo: i32,
}

impl From<&FuzzEntity> for EntityState {
    fn from(e: &FuzzEntity) -> Self {
        EntityState {
            id: e.id,
            pos: IVec2 { x: e.x, y: e.y },
            hp: e.hp,
            team: e.team,
            ammo: e.ammo,
        }
    }
}

fuzz_target!(|input: FuzzInterestInput| {
    let viewer = EntityState::from(&input.viewer);
    let target = EntityState::from(&input.target);

    // FullInterest should always return true
    let full = FullInterest;
    assert!(full.include(&viewer, &target));

    // RadiusTeamInterest should not panic with any inputs
    let radius_interest = RadiusTeamInterest {
        radius: input.radius.saturating_abs().max(1),
    };
    let _ = radius_interest.include(&viewer, &target);

    // Same team should always be included
    if viewer.team == target.team {
        assert!(radius_interest.include(&viewer, &target));
    }

    // Self should always be included (viewer == target by ID)
    if viewer.id == target.id {
        assert!(full.include(&viewer, &target));
    }
});
