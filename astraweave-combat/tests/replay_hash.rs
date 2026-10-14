//! Seeded multi-entity battle. The digest is SHA-256 of little-endian
//! Q32.32 raw bits (the integer form of `to_bits`) after entity ids are sorted.
//!
//! Writes `target/replay-hash.json` for the three-OS compare job.

use std::path::PathBuf;

use astraweave_combat::{
    hit_rng, load_ron, resolve_hit, AttackerView, BlockGate, DefenderView, Fixed64, ImmuneGate,
};
use sha2::{Digest, Sha256};

fn battle_table() -> astraweave_combat::CombatTables {
    load_ron(
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
        evasion_cap_bp: 0,
        attacker_bonus_cap_bp: 10000,
        max_flat_whole: 1000000,
        stagger_ticks: 12,
        block_omnidirectional: true,
    ),
    schools: [(
        id: 1, name: "physical", presentation: physical,
        k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500, periodic_rating: none,
    )],
    curves: [],
    abilities: [(
        id: 1, name: "swing", school: 1,
        weapon_min: 8, weapon_max: 8, weapon_bp: 10000,
        might_num: 1, might_den: 5,
        school_num: 0, school_den: 1, blight_num: 0, blight_den: 1,
        flat: 0, poise: 4,
        variance_min_bp: 10000, variance_max_bp: 10000,
        bleed: false, periodic: false, always_crit: false,
    )],
    heals: [],
)
"#,
    )
    .expect("battle table")
}

struct Fighter {
    id: u64,
    hp: i64,
    poise: i64,
    armor: i64,
    might: i64,
    iframe: u16,
}

fn run_battle() -> Vec<Fighter> {
    let tables = battle_table();
    let mut fighters: Vec<Fighter> = (0..8)
        .map(|i| Fighter {
            id: i,
            hp: Fixed64::from_int(180).raw(),
            poise: Fixed64::from_int(30).raw(),
            armor: 15 * i as i64,
            might: 4 + i as i64 * 3,
            iframe: if i == 0 { 2 } else { 0 },
        })
        .collect();
    for tick in 0..48u64 {
        let order: Vec<usize> = (0..fighters.len()).collect();
        let mut events = Vec::new();
        for attacker in order {
            if fighters[attacker].hp <= 0 {
                continue;
            }
            let mut target = (attacker + 1) % fighters.len();
            let mut steps = 0;
            while fighters[target].hp <= 0 && steps < fighters.len() {
                target = (target + 1) % fighters.len();
                steps += 1;
            }
            if fighters[target].hp <= 0 {
                continue;
            }
            events.push((fighters[attacker].id, attacker, target));
        }
        events.sort_by_key(|(id, _, _)| *id);
        for (ordinal, (_, attacker, target)) in events.iter().copied().enumerate() {
            let mods: [astraweave_combat::Modifier; 0] = [];
            let atk = AttackerView {
                might: fighters[attacker].might,
                ratings: &[],
                blight: 0,
                precision: 0,
                lethality_bonus_bp: 0,
                pen_bp: 0,
                tier: 0,
                modifiers: &mods,
            };
            let mut def = DefenderView {
                hp: Fixed64::from_raw(fighters[target].hp),
                hp_max: Fixed64::from_int(180),
                poise: Fixed64::from_raw(fighters[target].poise),
                poise_max: Fixed64::from_int(30),
                armor: fighters[target].armor,
                resists: &[],
                evasion_bp: 0,
                bulwark_bp: 0,
                bulwark_flat: 0,
                modifiers: &mods,
                block: BlockGate::default(),
                immune: ImmuneGate {
                    iframe_ticks: fighters[target].iframe,
                    parry_open: false,
                },
            };
            if attacker == 3 && tick % 5 == 0 {
                def.block.active = true;
                def.block.in_arc = true;
                def.bulwark_bp = 4_000;
            }
            let mut rng = hit_rng(0xA11C_E5EEu64, tick, ordinal as u64, 1);
            let outcome = resolve_hit(&tables, &atk, &def, 1, &mut rng).expect("hit");
            if fighters[target].iframe > 0 {
                fighters[target].iframe -= 1;
            }
            if outcome.hp_delta.raw() == 0 {
                continue;
            }
            fighters[target].hp = fighters[target].hp.saturating_sub(outcome.hp_delta.raw());
            fighters[target].poise = fighters[target]
                .poise
                .saturating_sub(outcome.poise_delta.raw());
            if fighters[target].poise <= 0 && outcome.poise_delta.raw() > 0 {
                fighters[target].poise = Fixed64::from_int(30).raw();
            }
        }
    }
    fighters.sort_by_key(|f| f.id);
    fighters
}

fn digest(fighters: &[Fighter]) -> String {
    let mut hasher = Sha256::new();
    for fighter in fighters {
        hasher.update(fighter.id.to_le_bytes());
        hasher.update(fighter.hp.to_le_bytes());
        hasher.update(fighter.poise.to_le_bytes());
        hasher.update(fighter.armor.to_le_bytes());
        hasher.update(fighter.might.to_le_bytes());
    }
    let bytes = hasher.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash.json");
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("target");
    path.push("replay-hash.json");
    path
}

#[test]
fn combat_resolve_replay_is_stable_and_written() {
    let first = digest(&run_battle());
    let second = digest(&run_battle());
    assert_eq!(first, second, "same seed diverged inside one process");
    assert_eq!(
        first, "975aae72e3c83a2335be05142eea2bf2ae4413d621fc5009ed6019f89e2a6e52",
        "combat_resolve digest drifted"
    );
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    let body = format!("[\n  {{ \"case\": \"combat_resolve\", \"sha256\": \"{first}\" }}\n]\n");
    std::fs::write(&path, &body).expect("write replay hash");
    println!("combat_resolve {first}");
    println!("wrote {}", path.display());
}
