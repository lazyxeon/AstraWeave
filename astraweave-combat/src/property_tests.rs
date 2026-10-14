//! Proven properties, each driven by one fixed proptest seed.
//!
//! `PROPTEST_SEED` is `0xC0BA_7E57`. Failure files are off, so a rerun draws
//! the same cases and does not depend on a regressions directory.

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use rand::RngCore;

use super::*;
use crate::resolve::{AttackerView, BlockGate, DefenderView, ImmuneGate, Modifier, ModifierKind};

const SEED: u64 = 0xC0BA_7E57;
const SCALE: i128 = 1 << 32;

fn fixed_config() -> Config {
    Config {
        cases: 48,
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(SEED),
        ..Config::default()
    }
}

fn run<S: Strategy>(strategy: S, test: impl Fn(S::Value) -> Result<(), TestCaseError>) {
    let mut runner = TestRunner::new(fixed_config());
    runner.run(&strategy, test).unwrap();
}

fn swing(k: i64, weapon: i64) -> CombatTables {
    let text = format!(
        r#"
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
        evasion_cap_bp: 4000,
        attacker_bonus_cap_bp: 10000,
        max_flat_whole: 1000000,
        stagger_ticks: 12,
        block_omnidirectional: true,
    ),
    schools: [(
        id: 1, name: "physical", presentation: physical,
        k_by_tier: [(tier: 0, k: {k})], cap_bp: 7500, periodic_rating: none,
    )],
    curves: [],
    abilities: [(
        id: 1, name: "swing", school: 1,
        weapon_min: {weapon}, weapon_max: {weapon}, weapon_bp: 10000,
        might_num: 0, might_den: 1, school_num: 0, school_den: 1,
        blight_num: 0, blight_den: 1, flat: 0, poise: 0,
        variance_min_bp: 10000, variance_max_bp: 10000,
        bleed: false, periodic: false, always_crit: false,
    )],
    heals: [],
)
"#
    );
    load_ron(&text).unwrap()
}

fn bare_atk<'a>(mods: &'a [Modifier]) -> AttackerView<'a> {
    AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: mods,
    }
}

fn bare_def<'a>(hp: i64, armor: i64, mods: &'a [Modifier]) -> DefenderView<'a> {
    DefenderView {
        hp: Fixed64::from_int(hp),
        hp_max: Fixed64::from_int(hp.max(1)),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    }
}

fn hit(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    def: &DefenderView<'_>,
    seed: u64,
) -> HitOutcome {
    let mut rng = hit_rng(seed, 1, 0, 1);
    resolve_hit(tables, atk, def, 1, &mut rng).unwrap()
}

#[test]
fn armor_equal_k_is_half_for_whole_units() {
    run(0i64..4_000, |damage| {
        let tables = swing(400, damage);
        let mods = [];
        let outcome = hit(
            &tables,
            &bare_atk(&mods),
            &bare_def(10_000, 400, &mods),
            SEED,
        );
        let expect = Fixed64::from_int(damage).raw() / 2;
        prop_assert_eq!(outcome.hp_delta.raw(), expect);
        Ok(())
    });
}

#[test]
fn ehp_is_linear_within_one_unit_below_the_cap() {
    run(0i64..=1_200, |armor| {
        let tables = swing(400, 1);
        let mods = [];
        let def = bare_def(1_000, armor, &mods);
        let ehp = effective_health(&tables, 0, &def, 1, StatKnowledge::self_sheet()).unwrap();
        let ideal = 1_000i128 * SCALE * (armor as i128 + 400) / 400;
        let gap = (i128::from(ehp.value.raw()) - ideal).unsigned_abs();
        prop_assert!(gap <= SCALE as u128);
        Ok(())
    });
}

#[test]
fn cap_is_continuous_at_three_k() {
    run(1_200i64..3_000, |armor| {
        let tables = swing(400, 1);
        let mods = [];
        let at = |a: i64| {
            let def = bare_def(1_000, a, &mods);
            effective_health(&tables, 0, &def, 1, StatKnowledge::self_sheet())
                .unwrap()
                .value
        };
        prop_assert_eq!(at(armor), Fixed64::from_int(4_000));
        prop_assert_eq!(at(1_200), at(armor));
        Ok(())
    });
}

#[test]
fn percent_stage_never_zeroes_a_positive_hit() {
    run(prop::collection::vec(0u32..9_001, 0..8), |reductions| {
        let tables = swing(400, 20);
        let mods: Vec<Modifier> = reductions
            .iter()
            .enumerate()
            .map(|(i, bp)| Modifier {
                id: i as u32 + 1,
                kind: ModifierKind::DefenderReduceBp,
                bp_or_flat: *bp,
            })
            .collect();
        let none = [];
        let outcome = hit(&tables, &bare_atk(&none), &bare_def(500, 0, &mods), SEED);
        prop_assert!(outcome.hp_delta.raw() > 0);
        Ok(())
    });
}

#[test]
fn flat_block_never_crosses_the_floor() {
    run(0i64..1_000_000, |flat| {
        let tables = swing(400, 100);
        let mods = [];
        let mut def = bare_def(500, 0, &mods);
        def.block = BlockGate {
            active: true,
            in_arc: true,
        };
        def.bulwark_flat = flat;
        let outcome = hit(&tables, &bare_atk(&mods), &def, SEED);
        let post = Fixed64::from_int(100).raw() as i128;
        let floor = post * 500 / 10_000;
        prop_assert!(i128::from(outcome.hp_delta.raw()) >= floor);
        Ok(())
    });
}

#[test]
fn overflow_sets_the_saturated_flag() {
    run(1usize..8, |bonus_count| {
        let tables = swing(400, 2_000_000_000);
        let mods: Vec<Modifier> = (0..bonus_count)
            .map(|i| Modifier {
                id: i as u32 + 1,
                kind: ModifierKind::AttackerBonusBp,
                bp_or_flat: 10_000,
            })
            .collect();
        let none = [];
        let outcome = hit(&tables, &bare_atk(&mods), &bare_def(1, 0, &none), SEED);
        prop_assert!(outcome.flags.contains(HitFlags::SATURATED));
        prop_assert_eq!(outcome.hp_delta.raw(), i64::MAX);
        Ok(())
    });
}

#[test]
fn modifier_id_order_does_not_change_the_result() {
    run(
        prop::collection::vec((1u32..40, 0u32..9_001), 1..6),
        |pairs| {
            let mut uniq = Vec::new();
            for (id, bp) in pairs {
                if uniq.iter().any(|(seen, _)| *seen == id) {
                    continue;
                }
                uniq.push((id, bp));
            }
            prop_assume!(!uniq.is_empty());
            let tables = swing(400, 40);
            let forward: Vec<Modifier> = uniq
                .iter()
                .map(|(id, bp)| Modifier {
                    id: *id,
                    kind: ModifierKind::DefenderReduceBp,
                    bp_or_flat: *bp,
                })
                .collect();
            let mut backward = forward.clone();
            backward.reverse();
            let none = [];
            let a = hit(
                &tables,
                &bare_atk(&none),
                &bare_def(800, 10, &forward),
                SEED,
            );
            let b = hit(
                &tables,
                &bare_atk(&none),
                &bare_def(800, 10, &backward),
                SEED,
            );
            prop_assert_eq!(a.hp_delta, b.hp_delta);
            let _ = forward;
            Ok(())
        },
    );
}

#[test]
fn gated_hits_still_draw_exactly_four_words() {
    run((0u64..10_000, 0u8..3), |(seed, gate)| {
        let tables = swing(400, 15);
        let mods = [];
        let mut def = bare_def(100, 0, &mods);
        match gate {
            0 => def.hp = Fixed64::ZERO,
            1 => def.immune.iframe_ticks = 4,
            _ => def.immune.parry_open = true,
        }
        let mut live = hit_rng(seed, 3, 0, 1);
        let mut twin = hit_rng(seed, 3, 0, 1);
        let outcome = resolve_hit(&tables, &bare_atk(&mods), &def, 1, &mut live).unwrap();
        prop_assert_eq!(outcome.hp_delta, Fixed64::ZERO);
        for _ in 0..4 {
            let _ = twin.next_u32();
        }
        prop_assert_eq!(live.next_u32(), twin.next_u32());
        Ok(())
    });
}
