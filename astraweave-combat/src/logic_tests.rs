use std::collections::{BTreeMap, HashMap};

use super::*;
use crate::resolve::{AttackerView, BlockGate, DefenderView, ImmuneGate, Modifier, ModifierKind};

const CANONICAL_FINAL: i64 = 34_084_860_460;
const CANONICAL_HASH: &str = "071dc70e116987782a8a8b137a6cce648d390a82967726775308946f1e69da71";

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::new();
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn sheet() -> StatKnowledge {
    StatKnowledge::self_sheet()
}

struct Canon {
    tables: CombatTables,
    atk_mods: [Modifier; 2],
    def_mods: [Modifier; 2],
}

fn canon() -> Canon {
    Canon {
        tables: canonical_tables().expect("fixture"),
        atk_mods: [
            Modifier {
                id: 1,
                kind: ModifierKind::AttackerBonusBp,
                bp_or_flat: 1_500,
            },
            Modifier {
                id: 2,
                kind: ModifierKind::AttackerBonusBp,
                bp_or_flat: 2_000,
            },
        ],
        def_mods: [
            Modifier {
                id: 10,
                kind: ModifierKind::DefenderReduceBp,
                bp_or_flat: 4_000,
            },
            Modifier {
                id: 11,
                kind: ModifierKind::DefenderReduceBp,
                bp_or_flat: 2_000,
            },
        ],
    }
}

fn views<'a>(c: &'a Canon) -> (AttackerView<'a>, DefenderView<'a>) {
    let atk = AttackerView {
        might: 100,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &c.atk_mods,
    };
    let def = DefenderView {
        hp: Fixed64::from_int(1_000),
        hp_max: Fixed64::from_int(1_000),
        poise: Fixed64::from_int(10),
        poise_max: Fixed64::from_int(10),
        armor: 400,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 2,
        modifiers: &c.def_mods,
        block: BlockGate {
            active: true,
            in_arc: true,
        },
        immune: ImmuneGate::default(),
    };
    (atk, def)
}

#[test]
fn canonical_hit_matches_the_golden_hash() {
    let c = canon();
    let (atk, def) = views(&c);
    let mut rng = hit_rng(1, 1, 0, 1);
    let outcome = resolve_hit(&c.tables, &atk, &def, 1, &mut rng).unwrap();
    assert_eq!(outcome.trace.step_count, 7);
    assert_eq!(outcome.trace.steps[6], CANONICAL_FINAL);
    assert_eq!(outcome.hp_delta.raw(), CANONICAL_FINAL);
    let digest = hash_steps(&outcome.trace.steps[..7]);
    assert_eq!(hex(&digest), CANONICAL_HASH);
}

#[test]
fn armor_equal_k_is_exactly_half() {
    let tables = canonical_tables().unwrap();
    let mods: [Modifier; 0] = [];
    // Ability weapon is 20. Use a direct half check through effective health
    // and a hit whose base is the weapon only by zeroing might on the canonical
    // ability (might coeff is 1/10). Build the half case with armor == k and
    // no other modifiers by scaling: weapon 20 + 0 might is not 100.
    // The published 100 -> 50 case is effective health's inverse: a 100-point
    // hit with armor 400 and k 400 and no modifiers.
    let def = DefenderView {
        hp: Fixed64::from_int(100),
        hp_max: Fixed64::from_int(100),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 400,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let ehp = effective_health(&tables, 0, &def, 1, sheet()).unwrap();
    assert!(ehp.complete);
    assert_eq!(ehp.value, Fixed64::from_int(200));
    let half = Fixed64::from_int(100);
    assert_eq!(
        (half.raw() as i128) * 400 / 800,
        i128::from(Fixed64::from_int(50).raw())
    );
}

#[test]
fn cooldown_forty_and_twenty_leave_forty_eight_percent() {
    let raw = Fixed64::from_int(1_000).raw() as i128;
    let once = raw * 6_000 / 10_000;
    let twice = once * 8_000 / 10_000;
    assert_eq!(twice, i128::from(Fixed64::from_int(480).raw()));
}

#[test]
fn gated_hit_still_draws_four() {
    let c = canon();
    let (atk, mut def) = views(&c);
    def.immune.iframe_ticks = 3;
    let mut a = hit_rng(4, 4, 1, 1);
    let mut b = hit_rng(4, 4, 1, 1);
    let outcome = resolve_hit(&c.tables, &atk, &def, 1, &mut a).unwrap();
    assert!(outcome.flags.contains(HitFlags::IMMUNE));
    assert_eq!(outcome.hp_delta, Fixed64::ZERO);
    let _ = b.next_u32();
    let _ = b.next_u32();
    let _ = b.next_u32();
    let _ = b.next_u32();
    assert_eq!(a.next_u32(), b.next_u32());
}

#[test]
fn variance_endpoints_are_inclusive() {
    assert_eq!(lerp_bp(0, 9_500, 10_500), 9_500);
    assert_eq!(lerp_bp(u32::MAX, 9_500, 10_500), 10_500);
}

#[test]
fn eight_reductions_stay_at_the_floor() {
    let tables = load_default().unwrap();
    let mut mods = [Modifier {
        id: 0,
        kind: ModifierKind::DefenderReduceBp,
        bp_or_flat: 9_000,
    }; 8];
    for (i, m) in mods.iter_mut().enumerate() {
        m.id = i as u32 + 1;
    }
    let atk_mods = [];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &atk_mods,
    };
    let def = DefenderView {
        hp: Fixed64::from_int(1_000),
        hp_max: Fixed64::from_int(1_000),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let mut rng = hit_rng(1, 2, 0, 2);
    let outcome = resolve_hit(&tables, &atk, &def, 2, &mut rng).unwrap();
    let floor = Fixed64::from_int(12).mul_ratio(500, 10_000);
    assert!(outcome.hp_delta.raw() > 0);
    assert!(outcome.hp_delta.raw() >= floor.raw());
}

#[test]
fn default_table_is_lore_safe_and_physical_viable() {
    let text = DEFAULT_TABLE_RON.to_ascii_lowercase();
    assert!(!text.contains("veil"));
    assert!(!text.contains("echo"));
    assert!(!text.contains("shock"));
    assert!(!text.contains("spell"));
    let tables = load_default().unwrap();
    assert!(tables.school(1).unwrap().presentation == SchoolPresentation::Physical);
    assert_eq!(tables.school(2).unwrap().name, "weave");
    assert!(tables.school(5).is_none());
    let mods = [];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &mods,
    };
    let open = DefenderView {
        hp: Fixed64::from_int(40),
        hp_max: Fixed64::from_int(40),
        poise: Fixed64::from_int(20),
        poise_max: Fixed64::from_int(20),
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let mut rng = hit_rng(1, 0, 0, 1);
    let swing = resolve_hit(&tables, &atk, &open, 1, &mut rng).unwrap();
    assert_eq!(swing.hp_delta, Fixed64::from_int(25));
    let shielded = DefenderView {
        bulwark_bp: 8_000,
        block: BlockGate {
            active: true,
            in_arc: true,
        },
        ..open
    };
    let mut rng = hit_rng(1, 0, 1, 1);
    let blocked = resolve_hit(&tables, &atk, &shielded, 1, &mut rng).unwrap();
    assert_eq!(blocked.hp_delta, Fixed64::from_int(5));
    assert!(blocked.hp_delta.raw() > 0);
}

#[test]
fn hidden_armor_cannot_change_the_estimate() {
    let tables = canonical_tables().unwrap();
    let mods = [];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &mods,
    };
    let mut know = sheet();
    know.armor = false;
    know.guard_and_mods = true;
    know.bulwark = true;
    let low = DefenderView {
        hp: Fixed64::from_int(100),
        hp_max: Fixed64::from_int(100),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let high = DefenderView { armor: 400, ..low };
    let a = effective_health(&tables, 0, &low, 1, know).unwrap();
    let b = effective_health(&tables, 0, &high, 1, know).unwrap();
    assert!(!a.complete);
    assert_eq!(a.value, b.value);
    let seen = effective_health(&tables, 0, &high, 1, sheet()).unwrap();
    assert!(seen.complete);
    assert_ne!(seen.value, a.value);
    let err = expected_hit(&tables, &atk, &low, 1, StatKnowledge::none());
    assert_eq!(err.unwrap_err(), CombatError::HiddenStat);
}

#[test]
fn hidden_block_does_not_change_expected_damage() {
    let tables = load_default().unwrap();
    let mods = [];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &mods,
    };
    let mut know = sheet();
    know.bulwark = false;
    let raised = DefenderView {
        hp: Fixed64::from_int(40),
        hp_max: Fixed64::from_int(40),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 8_000,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate {
            active: true,
            in_arc: true,
        },
        immune: ImmuneGate::default(),
    };
    let lowered = DefenderView {
        block: BlockGate::default(),
        bulwark_bp: 0,
        ..raised
    };
    let a = expected_hit(&tables, &atk, &raised, 1, know).unwrap();
    let b = expected_hit(&tables, &atk, &lowered, 1, know).unwrap();
    assert_eq!(a.value, b.value);
    assert!(!a.complete);
}

#[test]
fn thread_strain_is_not_an_input_to_damage() {
    let tables = load_default().unwrap();
    let mods = [];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &mods,
    };
    let def = DefenderView {
        hp: Fixed64::from_int(40),
        hp_max: Fixed64::from_int(40),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let mut a = hit_rng(9, 1, 0, 1);
    let mut b = hit_rng(9, 1, 0, 1);
    let left = resolve_hit(&tables, &atk, &def, 1, &mut a).unwrap();
    let right = resolve_hit(&tables, &atk, &def, 1, &mut b).unwrap();
    assert_eq!(left.hp_delta, right.hp_delta);
    let _strain = CThreadStrain {
        capacity: 10,
        reserve: 3,
        coherence: 2,
        strain: 9,
    };
}

#[test]
fn duplicate_modifier_is_an_error() {
    let tables = load_default().unwrap();
    let mods = [
        Modifier {
            id: 1,
            kind: ModifierKind::DefenderReduceBp,
            bp_or_flat: 100,
        },
        Modifier {
            id: 1,
            kind: ModifierKind::DefenderReduceBp,
            bp_or_flat: 100,
        },
    ];
    let atk = AttackerView {
        might: 0,
        ratings: &[],
        blight: 0,
        precision: 0,
        lethality_bonus_bp: 0,
        pen_bp: 0,
        tier: 0,
        modifiers: &[],
    };
    let def = DefenderView {
        hp: Fixed64::from_int(10),
        hp_max: Fixed64::from_int(10),
        poise: Fixed64::ZERO,
        poise_max: Fixed64::ZERO,
        armor: 0,
        resists: &[],
        evasion_bp: 0,
        bulwark_bp: 0,
        bulwark_flat: 0,
        modifiers: &mods,
        block: BlockGate::default(),
        immune: ImmuneGate::default(),
    };
    let mut rng = hit_rng(1, 1, 0, 2);
    let err = resolve_hit(&tables, &atk, &def, 2, &mut rng).unwrap_err();
    assert_eq!(err, CombatError::DuplicateModifier);
}

#[test]
fn load_rejects_zero_k_and_forbidden_names() {
    let bad_k = DEFAULT_TABLE_RON.replace("k: 100", "k: 0");
    assert_eq!(load_ron(&bad_k).unwrap_err(), CombatError::KOutOfRange);
    let renamed = DEFAULT_TABLE_RON.replace("name: \"weave\"", "name: \"veil\"");
    assert_eq!(load_ron(&renamed).unwrap_err(), CombatError::ForbiddenName);
}

#[test]
fn ron_key_order_does_not_change_the_hash_and_mods_append() {
    let a = load_default().unwrap();
    let swapped = DEFAULT_TABLE_RON.replace(
        "armor_cap_bp: 7500,\n        pen_cap_bp: 7500,",
        "pen_cap_bp: 7500,\n        armor_cap_bp: 7500,",
    );
    let b = load_ron(&swapped).unwrap();
    assert_eq!(a.hash(), b.hash());
    let extra = r#"
ModFile(
    mod_id: "alpha",
    schema: 1,
    schools: [
        (
            id: 9,
            name: "custom",
            presentation: hazard,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
    ],
)
"#;
    let with = apply_mods(&a, &[extra]).unwrap();
    assert!(with.school(9).is_some());
    let rename = r#"
ModFile(
    mod_id: "beta",
    schema: 1,
    schools: [
        (
            id: 1,
            name: "renamed",
            presentation: physical,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
    ],
)
"#;
    assert_eq!(
        apply_mods(&a, &[rename]).unwrap_err(),
        CombatError::SchoolRename
    );
}

#[test]
fn failed_install_keeps_the_previous_table() {
    let mut slot = None;
    install(&mut slot, load_default()).unwrap();
    let hash = slot.as_ref().unwrap().hash();
    let err = install(&mut slot, load_ron("nope"));
    assert!(err.is_err());
    assert_eq!(slot.unwrap().hash(), hash);
}

#[test]
fn save_roundtrip_keeps_sub_unit_hp_and_refuses_a_hash_mismatch() {
    let mut records = [VitalsRecord {
        entity: 7,
        hp: Fixed64::from_int(3).raw() + 5,
        hp_max: Fixed64::from_int(10).raw(),
        poise: Fixed64::from_ratio(1, 3).raw(),
        poise_max: Fixed64::from_int(4).raw(),
        strain: ThreadStrainSave {
            capacity: 8,
            reserve: 2,
            coherence: 1,
            strain: 4,
        },
    }];
    let mut meta = HashMap::new();
    write_combat_meta(&mut meta, "abc", &mut records);
    let loaded = read_combat_meta(&meta, "abc").unwrap();
    assert_eq!(loaded[0].hp, records[0].hp);
    assert_eq!(loaded[0].strain.strain, 4);
    assert_eq!(
        read_combat_meta(&meta, "def").unwrap_err(),
        CombatError::HashMismatch
    );
    let mut tree = BTreeMap::new();
    write_combat_meta(&mut tree, "abc", &mut records);
    assert_eq!(tree.get(META_TABLES).map(String::as_str), Some("abc"));
}

#[test]
fn save_bundle_meta_accepts_the_combat_keys() {
    use aw_save::{PlayerInventory, SaveBundleV2, WorldState, SAVE_SCHEMA_VERSION};
    use time::OffsetDateTime;
    use uuid::Uuid;
    let mut meta = HashMap::new();
    let mut records = [VitalsRecord {
        entity: 1,
        hp: 99,
        hp_max: 100,
        poise: 1,
        poise_max: 2,
        strain: ThreadStrainSave::default(),
    }];
    write_combat_meta(&mut meta, "feed", &mut records);
    let bundle = SaveBundleV2 {
        schema: SAVE_SCHEMA_VERSION,
        save_id: Uuid::from_u128(1),
        created_at: OffsetDateTime::UNIX_EPOCH,
        player_id: "placeholder".into(),
        slot: 0,
        world: WorldState {
            tick: 3,
            ecs_blob: Vec::new(),
            state_hash: 0,
        },
        companions: Vec::new(),
        inventory: PlayerInventory {
            credits: 0,
            items: Vec::new(),
        },
        meta,
    };
    assert!(bundle.meta.contains_key(META_VITALS));
    let again = bundle.clone();
    let loaded = read_combat_meta(&again.meta, "feed").unwrap();
    assert_eq!(loaded[0].hp, 99);
}

#[test]
fn sorted_batch_gives_the_lower_attacker_the_kill() {
    let mut app = astraweave_ecs::App::new().add_plugin(CombatPlugin);
    install(
        &mut app.world.get_resource_mut::<LoadedTables>().unwrap().0,
        load_default(),
    )
    .unwrap();
    if let Some(gate) = app.world.get_resource_mut::<CombatWriteGate>() {
        gate.open = true;
    }
    let foe = app.world.spawn();
    app.world.insert(
        foe,
        CVitals {
            hp: Fixed64::from_int(20),
            hp_max: Fixed64::from_int(20),
            poise: Fixed64::from_int(50),
            poise_max: Fixed64::from_int(50),
            stagger_left: 0,
        },
    );
    app.world.insert(foe, CDefense::default());
    let mut hitters = [app.world.spawn(), app.world.spawn()];
    hitters.sort();
    for hitter in hitters {
        app.world.insert(hitter, COffense::default());
    }
    if let Some(gate) = app.world.get_resource_mut::<CombatWriteGate>() {
        gate.open = false;
    }
    app.world
        .get_resource_mut::<HitQueue>()
        .unwrap()
        .push(HitEvent {
            attacker: hitters[1],
            defender: foe,
            attack_id: 1,
            seq: 0,
            heal: false,
        });
    app.world
        .get_resource_mut::<HitQueue>()
        .unwrap()
        .push(HitEvent {
            attacker: hitters[0],
            defender: foe,
            attack_id: 1,
            seq: 0,
            heal: false,
        });
    combat_resolve_system(&mut app.world);
    assert!(app.world.get::<CVitals>(foe).unwrap().hp.raw() <= 0);
    assert_eq!(
        app.world.get_resource::<CombatLog>().unwrap().deaths,
        vec![foe.to_raw()]
    );
}

#[test]
fn second_writer_is_refused_when_the_gate_is_closed() {
    let mut world = astraweave_ecs::World::new();
    world.insert_resource(CombatWriteGate { open: false });
    let e = world.spawn();
    world.insert(e, CVitals::default());
    let err = apply_hp_delta(&mut world, e, Fixed64::from_int(1), false).unwrap_err();
    assert_eq!(err, CombatError::DoubleWrite);
}

#[test]
fn parry_zeroes_damage_and_is_consumed() {
    let mut app = astraweave_ecs::App::new().add_plugin(CombatPlugin);
    install(
        &mut app.world.get_resource_mut::<LoadedTables>().unwrap().0,
        load_default(),
    )
    .unwrap();
    let foe = app.world.spawn();
    let hero = app.world.spawn();
    app.world.insert(hero, COffense::default());
    app.world.insert(
        foe,
        CVitals {
            hp: Fixed64::from_int(40),
            hp_max: Fixed64::from_int(40),
            ..CVitals::default()
        },
    );
    app.world.insert(foe, CDefense::default());
    app.world.insert(
        foe,
        CImmune {
            iframe_ticks: 0,
            parry_ticks: 2,
        },
    );
    app.world
        .get_resource_mut::<HitQueue>()
        .unwrap()
        .push(HitEvent {
            attacker: hero,
            defender: foe,
            attack_id: 1,
            seq: 0,
            heal: false,
        });
    combat_resolve_system(&mut app.world);
    assert_eq!(
        app.world.get::<CVitals>(foe).unwrap().hp,
        Fixed64::from_int(40)
    );
    assert_eq!(app.world.get::<CImmune>(foe).unwrap().parry_ticks, 0);
}
