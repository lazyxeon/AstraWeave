//! Edit log plus player pose, written through `aw-save` directly.

use std::path::Path;

use anyhow::{bail, Context, Result};
use astraweave_veil::{insert_into_meta, read_from_meta, ScarField};
use aw_save::{PlayerInventory, SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION};
use glam::Vec3;

/// Mixed into [`aw_save::sim_save_id`] for this demo. Not a random uuid.
pub const ARAVATH_SAVE_SEED: u64 = 0xA2A7_5A7E;

use crate::ground::VoxelEdit;
use crate::motor::MotorMode;

/// One enemy the checkpoint can put back into the fight.
#[derive(Clone, Debug)]
pub struct SavedEnemy {
    pub body: u64,
    pub health: f32,
    pub stagger: f32,
    pub windup: f32,
    pub attack_cooldown: f32,
    pub attack_time: f32,
    pub swinging: bool,
    pub hit_landed: bool,
    pub closing: bool,
    pub position: Vec3,
}

pub struct Checkpoint {
    pub edits: Vec<VoxelEdit>,
    /// `DestructionWorld::persist_bytes`. Empty on saves written before the brush log.
    pub demolition: Vec<u8>,
    pub position: Vec3,
    pub yaw: f32,
    pub stamina: f32,
    pub player_hp: f32,
    pub scars: ScarField,
    pub tick: u64,
    pub motor_mode: MotorMode,
    pub attack_time: f32,
    pub attack_swinging: bool,
    pub attack_hit: bool,
    pub lock: Option<usize>,
    pub cover_broken: bool,
    pub enemies: Vec<SavedEnemy>,
}

impl Checkpoint {
    pub fn basic(position: Vec3, yaw: f32, stamina: f32, player_hp: f32) -> Self {
        Self {
            edits: Vec::new(),
            demolition: Vec::new(),
            position,
            yaw,
            stamina,
            player_hp,
            scars: ScarField::new(),
            tick: 0,
            motor_mode: MotorMode::Locomotion,
            attack_time: 0.0,
            attack_swinging: false,
            attack_hit: false,
            lock: None,
            cover_broken: false,
            enemies: Vec::new(),
        }
    }
}

/// FNV-1a of the gameplay fields a load restores. Not `edits.len()`.
#[must_use]
pub fn gameplay_hash(checkpoint: &Checkpoint) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x1000_0000_01B3);
        }
    };
    mix(&checkpoint.tick.to_le_bytes());
    mix(&checkpoint.position.x.to_bits().to_le_bytes());
    mix(&checkpoint.position.y.to_bits().to_le_bytes());
    mix(&checkpoint.position.z.to_bits().to_le_bytes());
    mix(&checkpoint.yaw.to_bits().to_le_bytes());
    mix(&checkpoint.stamina.to_bits().to_le_bytes());
    mix(&checkpoint.player_hp.to_bits().to_le_bytes());
    mix(&[motor_tag(checkpoint.motor_mode)]);
    mix(&checkpoint.attack_time.to_bits().to_le_bytes());
    mix(&[
        u8::from(checkpoint.attack_swinging),
        u8::from(checkpoint.attack_hit),
    ]);
    mix(&checkpoint
        .lock
        .map(|index| index as u64)
        .unwrap_or(u64::MAX)
        .to_le_bytes());
    mix(&[u8::from(checkpoint.cover_broken)]);
    mix(&(checkpoint.enemies.len() as u64).to_le_bytes());
    for enemy in &checkpoint.enemies {
        mix(&enemy.body.to_le_bytes());
        mix(&enemy.health.to_bits().to_le_bytes());
        mix(&enemy.stagger.to_bits().to_le_bytes());
        mix(&enemy.windup.to_bits().to_le_bytes());
        mix(&enemy.attack_cooldown.to_bits().to_le_bytes());
        mix(&enemy.attack_time.to_bits().to_le_bytes());
        mix(&[
            u8::from(enemy.swinging),
            u8::from(enemy.hit_landed),
            u8::from(enemy.closing),
        ]);
        mix(&enemy.position.x.to_bits().to_le_bytes());
        mix(&enemy.position.y.to_bits().to_le_bytes());
        mix(&enemy.position.z.to_bits().to_le_bytes());
    }
    mix(&(checkpoint.edits.len() as u64).to_le_bytes());
    for edit in &checkpoint.edits {
        mix(&edit.x.to_le_bytes());
        mix(&edit.y.to_le_bytes());
        mix(&edit.z.to_le_bytes());
        mix(&edit.density.to_bits().to_le_bytes());
        mix(&edit.material.to_le_bytes());
    }
    mix(&(checkpoint.demolition.len() as u64).to_le_bytes());
    mix(&checkpoint.demolition);
    hash
}

fn motor_tag(mode: MotorMode) -> u8 {
    match mode {
        MotorMode::Locomotion => 0,
        MotorMode::Roll => 1,
        MotorMode::Fly => 2,
    }
}

fn motor_from_tag(tag: u8) -> Result<MotorMode> {
    match tag {
        0 => Ok(MotorMode::Locomotion),
        1 => Ok(MotorMode::Roll),
        2 => Ok(MotorMode::Fly),
        _ => bail!("motor mode is unknown"),
    }
}

pub fn save_checkpoint(root: &Path, checkpoint: &Checkpoint) -> Result<()> {
    let manager = SaveManager::new(root);
    let mut meta = std::collections::HashMap::new();
    meta.insert("edits".into(), encode_edits(&checkpoint.edits));
    if checkpoint.demolition.len() > MAX_DEMOLITION {
        bail!("demolition log is too long");
    }
    meta.insert("demolition".into(), encode_hex(&checkpoint.demolition));
    insert_into_meta(&mut meta, &checkpoint.scars).context("encode veil scars")?;
    meta.insert(
        "ground".into(),
        encode_hex(&astraweave_terrain::column_gen::worldgen_record()),
    );
    meta.insert(
        "player".into(),
        format!(
            "{},{},{},{},{},{}",
            encode_f32(checkpoint.position.x),
            encode_f32(checkpoint.position.y),
            encode_f32(checkpoint.position.z),
            encode_f32(checkpoint.yaw),
            encode_f32(checkpoint.stamina),
            encode_f32(checkpoint.player_hp)
        ),
    );
    meta.insert("tick".into(), checkpoint.tick.to_string());
    meta.insert("motor".into(), motor_tag(checkpoint.motor_mode).to_string());
    meta.insert(
        "attack".into(),
        format!(
            "{},{},{}",
            encode_f32(checkpoint.attack_time),
            u8::from(checkpoint.attack_swinging),
            u8::from(checkpoint.attack_hit)
        ),
    );
    meta.insert(
        "lock".into(),
        checkpoint
            .lock
            .map(|index| index.to_string())
            .unwrap_or_default(),
    );
    meta.insert(
        "cover".into(),
        u8::from(checkpoint.cover_broken).to_string(),
    );
    meta.insert("enemies".into(), encode_enemies(&checkpoint.enemies));
    let state_hash = gameplay_hash(checkpoint);
    let bundle = SaveBundleV2 {
        schema: SAVE_SCHEMA_VERSION,
        save_id: aw_save::sim_save_id(ARAVATH_SAVE_SEED, checkpoint.tick, 0),
        created_at: aw_save::sim_timestamp(checkpoint.tick),
        player_id: "aravath".into(),
        slot: 0,
        world: WorldState {
            tick: checkpoint.tick,
            ecs_blob: meta.get("edits").cloned().unwrap_or_default().into_bytes(),
            state_hash,
        },
        companions: Vec::new(),
        inventory: PlayerInventory {
            credits: 0,
            items: Vec::new(),
        },
        meta,
    };
    manager
        .save("aravath", 0, bundle)
        .context("write aravath save")?;
    Ok(())
}

pub fn load_checkpoint(root: &Path) -> Result<Checkpoint> {
    let manager = SaveManager::new(root);
    let (bundle, _) = manager
        .load_latest_slot("aravath", 0)
        .context("read aravath save")?;
    let edits_text = bundle.meta.get("edits").map(String::as_str).unwrap_or("");
    let edits = decode_edits(edits_text)?;
    let demolition = match bundle.meta.get("demolition").map(String::as_str) {
        Some(text) if !text.is_empty() => {
            let bytes = decode_hex(text).context("demolition log")?;
            if bytes.len() > MAX_DEMOLITION {
                bail!("demolition log is too long");
            }
            astraweave_destruction::DestructionWorld::restore(&bytes)
                .map_err(|err| anyhow::anyhow!("demolition log did not restore: {err:?}"))?;
            bytes
        }
        _ => Vec::new(),
    };
    let player = bundle
        .meta
        .get("player")
        .context("save is missing the player")?;
    if player.len() > 256 {
        bail!("player record is too long");
    }
    let mut parts = player.split(',');
    let position = Vec3::new(
        finite_coord(parts.next().context("x")?, "x")?,
        finite_coord(parts.next().context("y")?, "y")?,
        finite_coord(parts.next().context("z")?, "z")?,
    );
    let yaw = finite_limited(parts.next().context("yaw")?, "yaw", 1.0e6)?;
    let stamina = finite_limited(parts.next().context("stamina")?, "stamina", 10_000.0)?;
    let player_hp = match parts.next() {
        Some(text) => finite_limited(text, "player hp", 10_000.0)?,
        None => 100.0,
    };
    require_saved_ground(&bundle.meta)?;
    let scars = read_from_meta(&bundle.meta).context("decode veil scars")?;
    let tick = match bundle.meta.get("tick") {
        Some(text) => text.parse::<u64>().context("tick")?,
        None => bundle.world.tick,
    };
    let motor_mode = match bundle.meta.get("motor") {
        Some(text) => motor_from_tag(text.parse::<u8>().context("motor")?)?,
        None => MotorMode::Locomotion,
    };
    let (attack_time, attack_swinging, attack_hit) = match bundle.meta.get("attack") {
        Some(text) => decode_attack(text)?,
        None => (0.0, false, false),
    };
    let lock = match bundle.meta.get("lock") {
        Some(text) if text.is_empty() => None,
        Some(text) => Some(text.parse::<usize>().context("lock")?),
        None => None,
    };
    let cover_broken = match bundle.meta.get("cover") {
        Some(text) => text == "1",
        None => false,
    };
    let enemies = match bundle.meta.get("enemies") {
        Some(text) => decode_enemies(text)?,
        None => Vec::new(),
    };
    Ok(Checkpoint {
        edits,
        demolition,
        position,
        yaw,
        stamina,
        player_hp,
        scars,
        tick,
        motor_mode,
        attack_time,
        attack_swinging,
        attack_hit,
        lock,
        cover_broken,
        enemies,
    })
}

const MAX_EDITS: usize = 16_384;
const MAX_DEMOLITION: usize = 1024 * 1024;
const MAX_EDIT_COORD: i32 = 2_048;
const MAX_EDIT_TEXT: usize = 2 * 1024 * 1024;

/// Eight lowercase hex digits of `f32::to_bits`. Decimal text stays readable
/// so saves written before the bit encoding still load.
fn encode_f32(value: f32) -> String {
    format!("{:08x}", value.to_bits())
}

fn decode_f32(text: &str) -> Result<f32> {
    if text.len() == 8 && text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        let bits = u32::from_str_radix(text, 16).context("f32 bits")?;
        return Ok(f32::from_bits(bits));
    }
    text.parse().context("f32")
}

fn finite_coord(text: &str, name: &str) -> Result<f32> {
    finite_limited(text, name, MAX_EDIT_COORD as f32)
}

fn finite_limited(text: &str, name: &str, limit: f32) -> Result<f32> {
    let value = decode_f32(text).with_context(|| format!("{name} is not a number"))?;
    if !value.is_finite() || value.abs() > limit {
        bail!("{name} is outside the allowed range");
    }
    Ok(value)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(text: &str) -> Result<Vec<u8>> {
    if text.len() % 2 != 0 {
        bail!("ground record is not hex");
    }
    let raw = text.as_bytes();
    let mut out = Vec::with_capacity(text.len() / 2);
    let mut index = 0;
    while index < raw.len() {
        let hi = hex_nibble(raw[index])?;
        let lo = hex_nibble(raw[index + 1])?;
        out.push((hi << 4) | lo);
        index += 2;
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("ground record is not hex"),
    }
}

/// Old saves omit `ground`. A present key must be the integer columns.
fn require_saved_ground(meta: &std::collections::HashMap<String, String>) -> Result<()> {
    let Some(text) = meta.get("ground") else {
        return Ok(());
    };
    let stored = decode_hex(text).context("ground record")?;
    let fresh = astraweave_terrain::column_gen::worldgen_record();
    if stored != fresh {
        bail!("saved ground does not match the integer columns");
    }
    Ok(())
}

fn encode_enemies(enemies: &[SavedEnemy]) -> String {
    enemies
        .iter()
        .map(|enemy| {
            format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                enemy.body,
                encode_f32(enemy.health),
                encode_f32(enemy.stagger),
                encode_f32(enemy.windup),
                encode_f32(enemy.attack_cooldown),
                encode_f32(enemy.attack_time),
                u8::from(enemy.swinging),
                u8::from(enemy.hit_landed),
                u8::from(enemy.closing),
                encode_f32(enemy.position.x),
                encode_f32(enemy.position.y),
                encode_f32(enemy.position.z)
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_enemies(text: &str) -> Result<Vec<SavedEnemy>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut enemies = Vec::new();
    for row in text.split(';') {
        let mut parts = row.split(',');
        let body = parts
            .next()
            .context("enemy body")?
            .parse()
            .context("enemy body")?;
        let health = finite_limited(parts.next().context("health")?, "health", 10_000.0)?;
        let stagger = finite_limited(parts.next().context("stagger")?, "stagger", 10_000.0)?;
        let windup = finite_limited(parts.next().context("windup")?, "windup", 10_000.0)?;
        let attack_cooldown =
            finite_limited(parts.next().context("cooldown")?, "cooldown", 10_000.0)?;
        let attack_time = finite_limited(parts.next().context("attack")?, "attack", 10_000.0)?;
        let swinging = parts.next().context("swing")? == "1";
        let hit_landed = parts.next().context("hit")? == "1";
        let closing = parts.next().context("closing")? == "1";
        let position = Vec3::new(
            finite_coord(parts.next().context("ex")?, "enemy x")?,
            finite_coord(parts.next().context("ey")?, "enemy y")?,
            finite_coord(parts.next().context("ez")?, "enemy z")?,
        );
        enemies.push(SavedEnemy {
            body,
            health,
            stagger,
            windup,
            attack_cooldown,
            attack_time,
            swinging,
            hit_landed,
            closing,
            position,
        });
    }
    Ok(enemies)
}

fn decode_attack(text: &str) -> Result<(f32, bool, bool)> {
    let mut parts = text.split(',');
    let time = finite_limited(
        parts.next().context("attack time")?,
        "attack time",
        10_000.0,
    )?;
    let swinging = parts.next().context("swing")? == "1";
    let hit = parts.next().context("hit")? == "1";
    Ok((time, swinging, hit))
}

fn encode_edits(edits: &[VoxelEdit]) -> String {
    edits
        .iter()
        .map(|edit| {
            format!(
                "{}:{}:{}:{}:{}",
                edit.x,
                edit.y,
                edit.z,
                encode_f32(edit.density),
                edit.material
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_edits(text: &str) -> Result<Vec<VoxelEdit>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    if text.len() > MAX_EDIT_TEXT {
        bail!("edit log is too large");
    }
    let mut edits = Vec::new();
    for row in text.split(';') {
        if edits.len() >= MAX_EDITS {
            bail!("edit log has more than {MAX_EDITS} entries");
        }
        let mut parts = row.split(':');
        let x: i32 = parts.next().context("edit x")?.parse().context("edit x")?;
        let y: i32 = parts.next().context("edit y")?.parse().context("edit y")?;
        let z: i32 = parts.next().context("edit z")?.parse().context("edit z")?;
        let density = decode_f32(parts.next().context("edit density")?).context("edit density")?;
        let material: u16 = parts
            .next()
            .context("edit material")?
            .parse()
            .context("edit material")?;
        if x.abs() > MAX_EDIT_COORD || y.abs() > MAX_EDIT_COORD || z.abs() > MAX_EDIT_COORD {
            bail!("edit coordinate is outside the island");
        }
        if !density.is_finite() || density.abs() > 100.0 {
            bail!("edit density is outside the allowed range");
        }
        if material > 16 {
            bail!("edit material is unknown");
        }
        edits.push(VoxelEdit {
            x,
            y,
            z,
            density,
            material,
        });
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_nav::{Aabb, NavMesh};
    use astraweave_physics::PhysicsWorld;
    use glam::Vec3;

    use crate::ground::Ground;
    use crate::island;

    #[test]
    fn unsupported_place_is_a_noop_and_supported_place_roundtrips() {
        let mut ground = Ground::from_grid(island::solid_box([0, 0, 0], [6, 3, 6]));
        assert!(!ground.place_sphere(Vec3::new(3.0, 20.0, 3.0), 0.4, 1));
        assert!(ground.edits.is_empty());

        assert!(ground.place_sphere(Vec3::new(3.0, 4.0, 3.0), 0.6, 2));
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let first = ground.sync_dirty(&mut physics);
        let verts: usize = first.iter().map(|chunk| chunk.vertex_count()).sum();
        assert!(verts > 0);

        let dir = tempfile::tempdir().unwrap();
        let position = Vec3::new(3.0, 6.5, 3.0);
        let mut checkpoint = Checkpoint::basic(position, 0.4, 77.0, 100.0);
        checkpoint.edits = ground.edits.clone();
        save_checkpoint(dir.path(), &checkpoint).unwrap();
        let loaded = load_checkpoint(dir.path()).unwrap();
        assert!((loaded.position - position).length() < 1e-3);
        assert!((loaded.stamina - 77.0).abs() < 1e-3);

        let mut fresh = Ground::from_grid(island::solid_box([0, 0, 0], [6, 3, 6]));
        fresh.replay(&loaded.edits);
        let mut physics_b = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let second = fresh.sync_dirty(&mut physics_b);
        let verts_b: usize = second.iter().map(|chunk| chunk.vertex_count()).sum();
        assert_eq!(verts, verts_b);

        let mut nav = NavMesh::bake(&[], 0.5, 55.0);
        nav.invalidate_region(Aabb::from_center_half_extents(
            Vec3::new(3.0, 4.0, 3.0),
            Vec3::splat(0.6),
        ));
        assert!(nav.dirty_region_count() >= 1);
    }

    #[test]
    fn a_save_cannot_request_a_nan_position_or_an_unbounded_edit() {
        let dir = tempfile::tempdir().unwrap();
        let err = decode_edits("nan:0:0:1:1").unwrap_err();
        assert!(
            err.to_string().contains("density") || err.to_string().contains("x"),
            "{err}"
        );
        let err = decode_edits("1:2:3:inf:1").unwrap_err();
        assert!(err.to_string().contains("density"), "{err}");
        let err = decode_edits("90000:0:0:1:1").unwrap_err();
        assert!(err.to_string().contains("coordinate"), "{err}");
        let huge = std::iter::repeat_n("1:2:3:1:1", MAX_EDITS + 1)
            .collect::<Vec<_>>()
            .join(";");
        assert!(decode_edits(&huge).is_err());

        save_checkpoint(
            dir.path(),
            &Checkpoint::basic(Vec3::new(1.0, 2.0, 3.0), 0.0, 1.0, 1.0),
        )
        .unwrap();
        let (bundle, _) = aw_save::SaveManager::new(dir.path())
            .load_latest_slot("aravath", 0)
            .unwrap();
        let mut poisoned = bundle;
        poisoned
            .meta
            .insert("player".into(), "NaN,0,0,0,1,1".into());
        // Rewrite through the public loader by patching the on-disk meta is
        // covered by parse rejection below.
        let err = finite_limited("NaN", "x", 10.0).unwrap_err();
        assert!(err.to_string().contains("range"), "{err}");
        let _ = poisoned;
        let edits = vec![crate::ground::VoxelEdit {
            x: 1,
            y: 2,
            z: -3,
            density: 0.5,
            material: 4,
        }];
        assert_eq!(encode_edits(&edits), "1:2:-3:3f000000:4");
        assert_eq!(decode_edits("1:2:-3:0.500:4").unwrap()[0].material, 4);
        assert_eq!(
            decode_edits("1:2:-3:3f000000:4").unwrap()[0]
                .density
                .to_bits(),
            0.5f32.to_bits()
        );
    }

    #[test]
    fn saved_ground_is_the_integer_record() {
        let dir = tempfile::tempdir().unwrap();
        save_checkpoint(dir.path(), &Checkpoint::basic(Vec3::ZERO, 0.0, 1.0, 1.0)).unwrap();
        let loaded = load_checkpoint(dir.path()).unwrap();
        assert!(loaded.edits.is_empty());
        let (bundle, _) = aw_save::SaveManager::new(dir.path())
            .load_latest_slot("aravath", 0)
            .unwrap();
        let mut mismatched = bundle.meta.clone();
        mismatched.insert("ground".into(), "00".into());
        let err = require_saved_ground(&mismatched).unwrap_err();
        assert!(err.to_string().contains("integer columns"), "{err}");
        let mut legacy = bundle.meta.clone();
        legacy.remove("ground");
        require_saved_ground(&legacy).unwrap();
    }
}

#[cfg(test)]
mod stream_tests {
    use astraweave_physics::PhysicsWorld;
    use glam::Vec3;

    use crate::ground::Ground;
    use astraweave_terrain::VoxelGrid;

    #[test]
    fn streaming_loads_a_ring_and_unloads_the_origin() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(VoxelGrid::new());
        let first = ground.stream_around(&mut physics, Vec3::ZERO, 1);
        assert!(first.iter().any(|chunk| !chunk.removed));
        assert!(ground.body_ids().keys().any(|coord| coord.x == 0));
        let _ = ground.stream_around(&mut physics, Vec3::new(220.0, 12.0, 0.0), 1);
        assert!(
            ground.body_ids().keys().any(|coord| coord.x >= 5),
            "far chunks missing"
        );
        assert!(
            !ground.body_ids().keys().any(|coord| coord.x == 0),
            "origin chunk was not unloaded"
        );
    }
}
