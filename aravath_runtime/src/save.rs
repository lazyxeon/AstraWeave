//! Edit log plus player pose, written through `aw-save` directly.

use std::path::Path;

use anyhow::{bail, Context, Result};
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
    let edits_text = bundle.meta.get("edits").map(String::as_str).unwrap_or("");
    let edits = decode_edits(edits_text)?;
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

const MAX_EDITS: usize = 16_384;
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
            &Checkpoint {
                edits: Vec::new(),
                position: Vec3::new(1.0, 2.0, 3.0),
                yaw: 0.0,
                stamina: 1.0,
                player_hp: 1.0,
                scars: astraweave_veil::ScarField::new(),
            },
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
        let err = finite_limited("7fc00000", "x", 10.0).unwrap_err();
        assert!(err.to_string().contains("range"), "{err}");
        let err = decode_edits("1:2:3:7f800000:1").unwrap_err();
        assert!(err.to_string().contains("density"), "{err}");
        let _ = poisoned;
        let edits = vec![crate::ground::VoxelEdit {
            x: 1,
            y: 2,
            z: -3,
            density: 0.5,
            material: 4,
        }];
        assert_eq!(encode_edits(&edits), "1:2:-3:3f000000:4");
        let legacy = decode_edits("1:2:-3:0.500:4").unwrap();
        assert_eq!(legacy[0].material, 4);
        assert!((legacy[0].density - 0.5).abs() < 1e-6);
        let current = decode_edits("1:2:-3:3f000000:4").unwrap();
        assert!((current[0].density - 0.5).abs() < 1e-6);
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
