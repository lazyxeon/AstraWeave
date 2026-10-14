//! Edit log plus player pose, written through `aw-save` directly.

use std::path::Path;

use anyhow::{Context, Result};
use aw_save::{PlayerInventory, SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION};
use glam::Vec3;
use time::OffsetDateTime;
use uuid::Uuid;

use astraweave_veil::{insert_into_meta, read_from_meta, ScarField};

use crate::ground::VoxelEdit;

pub struct Checkpoint {
    pub edits: Vec<VoxelEdit>,
    pub position: Vec3,
    pub yaw: f32,
    pub stamina: f32,
    pub player_hp: f32,
    pub scars: ScarField,
}

pub fn save_checkpoint(root: &Path, checkpoint: &Checkpoint) -> Result<()> {
    let manager = SaveManager::new(root);
    let mut meta = std::collections::HashMap::new();
    meta.insert("edits".into(), encode_edits(&checkpoint.edits));
    insert_into_meta(&mut meta, &checkpoint.scars).context("encode veil scars")?;
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
    let bundle = SaveBundleV2 {
        schema: SAVE_SCHEMA_VERSION,
        save_id: Uuid::new_v4(),
        created_at: OffsetDateTime::now_utc(),
        player_id: "aravath".into(),
        slot: 0,
        world: WorldState {
            tick: 0,
            ecs_blob: meta.get("edits").cloned().unwrap_or_default().into_bytes(),
            state_hash: checkpoint.edits.len() as u64,
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
    let edits = decode_edits(bundle.meta.get("edits").map(String::as_str).unwrap_or(""));
    let player = bundle
        .meta
        .get("player")
        .context("save is missing the player")?;
    let mut parts = player.split(',');
    let position = Vec3::new(
        decode_f32(parts.next().context("x")?).context("x")?,
        decode_f32(parts.next().context("y")?).context("y")?,
        decode_f32(parts.next().context("z")?).context("z")?,
    );
    let yaw = decode_f32(parts.next().context("yaw")?).context("yaw")?;
    let stamina = decode_f32(parts.next().context("stamina")?).context("stamina")?;
    let player_hp = match parts.next() {
        Some(text) => decode_f32(text).context("player hp")?,
        None => 100.0,
    };
    let scars = read_from_meta(&bundle.meta).context("decode veil scars")?;
    Ok(Checkpoint {
        edits,
        position,
        yaw,
        stamina,
        player_hp,
        scars,
    })
}

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

fn decode_edits(text: &str) -> Vec<VoxelEdit> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split(';')
        .filter_map(|row| {
            let mut parts = row.split(':');
            Some(VoxelEdit {
                x: parts.next()?.parse().ok()?,
                y: parts.next()?.parse().ok()?,
                z: parts.next()?.parse().ok()?,
                density: decode_f32(parts.next()?).ok()?,
                material: parts.next()?.parse().ok()?,
            })
        })
        .collect()
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
        save_checkpoint(
            dir.path(),
            &Checkpoint {
                edits: ground.edits.clone(),
                position,
                yaw: 0.4,
                stamina: 77.0,
                player_hp: 100.0,
                scars: astraweave_veil::ScarField::new(),
            },
        )
        .unwrap();
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
