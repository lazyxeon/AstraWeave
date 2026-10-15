//! Aravath sword, weave school, and destructible cover.
//!
//! This is the caller the greybox combat tick should use. The greybox
//! runtime on its own branch still subtracts a flat health value. This
//! function is the wired path: one combat tick, data-driven schools, and
//! a cover impulse in the destruction log.

use astraweave_combat::{
    install, load_ron, CCover, CDefense, COffense, CVitals, CombatClock, CombatPlugin, Fixed64,
    HitEvent, HitQueue, LoadedTables,
};
use astraweave_destruction::command::{CommandKind, DestructionCommand, SpawnBond, SpawnNode};
use astraweave_destruction::DestructionWorld;
use astraweave_ecs::App;

use crate::damage::{apply_element_schools, ElementSchool};

const TABLE: &str = r#"
TableFile(
    schema: 1,
    defaults: (
        armor_cap_bp: 7500,
        pen_cap_bp: 7500,
        crit_cap_bp: 7500,
        crit_base_bp: 15000,
        crit_mult_cap_bp: 20000,
        min_taken_bp: 500,
        glance_bp: 7500,
        glance_chance_bp: 0,
        variance_min_bp: 10000,
        variance_max_bp: 10000,
        evasion_cap_bp: 0,
        attacker_bonus_cap_bp: 10000,
        max_flat_whole: 1000000,
        stagger_ticks: 12,
        block_omnidirectional: true,
    ),
    schools: [
        (id: 1, name: "physical", presentation: physical, k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500, periodic_rating: none),
        (id: 2, name: "weave", presentation: weave, k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500, periodic_rating: none),
    ],
    curves: [],
    abilities: [
        (
            id: 1, name: "sword_light", school: 1,
            weapon_min: 25, weapon_max: 25, weapon_bp: 10000,
            might_num: 0, might_den: 1,
            school_num: 0, school_den: 1, blight_num: 0, blight_den: 1,
            flat: 0, poise: 4,
            variance_min_bp: 10000, variance_max_bp: 10000,
            bleed: false, periodic: false, always_crit: false,
        ),
        (
            id: 3, name: "weave_bolt", school: 2,
            weapon_min: 0, weapon_max: 0, weapon_bp: 10000,
            might_num: 0, might_den: 1,
            school_num: 1, school_den: 1, blight_num: 0, blight_den: 1,
            flat: 0, poise: 0,
            variance_min_bp: 10000, variance_max_bp: 10000,
            bleed: false, periodic: false, always_crit: false,
        ),
    ],
    heals: [],
)
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AravathSlice {
    pub sword_hp: i64,
    pub weave_hp: i64,
    pub cover_intact: bool,
    pub cover_damage: i64,
}

/// World seed and destruction seed used by [`run_aravath_slice`].
pub const ARAVATH_SLICE_SEED: u64 = 0xA11C_E5EE;

/// Sword for 25, a data-driven weave school for 15, and a cover bond that breaks.
pub fn run_aravath_slice() -> AravathSlice {
    let mut app = App::new().add_plugin(CombatPlugin);
    install(
        &mut app.world.get_resource_mut::<LoadedTables>().unwrap().0,
        load_ron(TABLE),
    )
    .expect("install table");
    app.world.insert_resource(CombatClock {
        world_seed: ARAVATH_SLICE_SEED,
        tick: 0,
    });

    let mut destruction = DestructionWorld::new(ARAVATH_SLICE_SEED, 0);
    let half = [Fixed64::from_int(1); 3];
    let post = [Fixed64::from_int(0), Fixed64::from_int(1), Fixed64::ZERO];
    destruction
        .enqueue(DestructionCommand {
            tick: 0,
            seq: 0,
            entity_id: 1,
            kind: CommandKind::SpawnNode(SpawnNode {
                structure: 1,
                mass: Fixed64::from_int(40),
                pos: post,
                half,
                flags: 1,
            }),
        })
        .expect("spawn post");
    destruction
        .enqueue(DestructionCommand {
            tick: 0,
            seq: 0,
            entity_id: 1,
            kind: CommandKind::SpawnNode(SpawnNode {
                structure: 1,
                mass: Fixed64::from_int(40),
                pos: [Fixed64::from_int(2), Fixed64::from_int(1), Fixed64::ZERO],
                half,
                flags: 1,
            }),
        })
        .expect("spawn post");
    destruction
        .enqueue(DestructionCommand {
            tick: 0,
            seq: 0,
            entity_id: 1,
            kind: CommandKind::SpawnBond(SpawnBond {
                a: 1,
                b: 2,
                strength: Fixed64::from_int(10),
                area: Fixed64::from_int(1),
            }),
        })
        .expect("spawn bond");
    app.world.insert_resource(destruction);

    let hero = app.world.spawn();
    let mut offense = COffense::default();
    let mut unused = CDefense::default();
    let weave = [ElementSchool {
        school_id: 2,
        rating: 15,
        resist: 0,
        density_delta: -1,
        decay_whole: 5,
        origin: [0, 1, 0],
        radius: 2,
    }];
    apply_element_schools(&mut offense, &mut unused, &weave).expect("weave schools");
    app.world.insert(hero, offense);

    let cover = app.world.spawn();
    app.world.insert(
        cover,
        CVitals {
            hp: Fixed64::from_int(40),
            hp_max: Fixed64::from_int(40),
            poise: Fixed64::from_int(20),
            poise_max: Fixed64::from_int(20),
            stagger_left: 0,
        },
    );
    app.world.insert(cover, CDefense::default());
    app.world.insert(
        cover,
        CCover {
            structure: 1,
            point: post,
        },
    );

    let woven = app.world.spawn();
    app.world.insert(
        woven,
        CVitals {
            hp: Fixed64::from_int(40),
            hp_max: Fixed64::from_int(40),
            poise: Fixed64::from_int(1),
            poise_max: Fixed64::from_int(1),
            stagger_left: 0,
        },
    );
    app.world.insert(woven, CDefense::default());

    app.world
        .get_resource_mut::<HitQueue>()
        .expect("hit queue")
        .push(HitEvent {
            attacker: hero,
            defender: cover,
            attack_id: 1,
            seq: 0,
            heal: false,
        });
    app.world
        .get_resource_mut::<HitQueue>()
        .expect("hit queue")
        .push(HitEvent {
            attacker: hero,
            defender: woven,
            attack_id: 3,
            seq: 1,
            heal: false,
        });

    astraweave_combat::combat_resolve_system(&mut app.world);
    let decay = weave[0];
    app.world
        .get_resource_mut::<DestructionWorld>()
        .expect("destruction")
        .enqueue_weave(
            0,
            1,
            astraweave_destruction::command::Weave {
                origin: decay.origin,
                radius: decay.radius,
                density_delta: decay.density_delta,
                decay: Fixed64::from_int(decay.decay_whole),
            },
        )
        .expect("weave decay");
    app.world
        .get_resource_mut::<DestructionWorld>()
        .expect("destruction")
        .tick();

    let sword_hp = app
        .world
        .get::<CVitals>(cover)
        .expect("cover vitals")
        .hp
        .raw()
        / Fixed64::SCALE;
    let weave_hp = app
        .world
        .get::<CVitals>(woven)
        .expect("weave vitals")
        .hp
        .raw()
        / Fixed64::SCALE;
    let destruction = app
        .world
        .get_resource::<DestructionWorld>()
        .expect("destruction");
    let bond = destruction
        .graph()
        .bonds
        .iter()
        .find(|bond| bond.id == 1)
        .expect("cover bond");
    AravathSlice {
        sword_hp,
        weave_hp,
        cover_intact: bond.intact,
        cover_damage: bond.damage.raw() / Fixed64::SCALE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sword_weave_and_cover_resolve_together() {
        let slice = run_aravath_slice();
        assert_eq!(slice.sword_hp, 15, "sword 25 from 40");
        assert_eq!(slice.weave_hp, 25, "weave rating 15 from 40");
        assert!(!slice.cover_intact, "cover bond should break");
        assert!(slice.cover_damage >= 25);
    }
}
