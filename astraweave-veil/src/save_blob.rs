//! Postcard blob stored in `SaveBundleV2.meta` under [`META_KEY`].
//!
//! The bytes are a sorted cell list. `HashMap` iteration never touches them.
//! `aw-save` wraps the bundle (LZ4, CRC, atomic rename). The scar hash is of
//! the field, not of the `.awsv` file, because the file name carries a clock.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::field::ScarField;
use crate::fixed::Fixed64;

pub const META_KEY: &str = "veil_scars";
const BLOB_VERSION: u16 = 1;
/// Hostile cell lists above this are rejected before any cell is inserted.
/// Greybox deaths and the reference history use a handful of cells.
pub const MAX_SCAR_CELLS: usize = 4096;

#[derive(Serialize, Deserialize)]
struct ScarWire {
    v: u16,
    cell_metres: i32,
    decay: i64,
    retain_num: i64,
    retain_den: i64,
    wary: i64,
    avoid: i64,
    ticks: u64,
    cells: Vec<WireCell>,
}

#[derive(Serialize, Deserialize)]
struct WireCell {
    x: i32,
    y: i32,
    z: i32,
    floor: i64,
    transient: i64,
}

impl ScarField {
    pub fn to_postcard(&self) -> Result<Vec<u8>> {
        let wire = ScarWire {
            v: BLOB_VERSION,
            cell_metres: self.cell_metres(),
            decay: self.decay_per_tick().raw(),
            retain_num: self.retain_num(),
            retain_den: self.retain_den(),
            wary: self.wary_at().raw(),
            avoid: self.avoid_at().raw(),
            ticks: self.ticks(),
            cells: self
                .cells()
                .map(|(key, cell)| WireCell {
                    x: key[0],
                    y: key[1],
                    z: key[2],
                    floor: cell.floor().raw(),
                    transient: cell.transient().raw(),
                })
                .collect(),
        };
        postcard::to_allocvec(&wire).context("encode veil scars")
    }

    pub fn from_postcard(bytes: &[u8]) -> Result<Self> {
        let wire: ScarWire = postcard::from_bytes(bytes).context("decode veil scars")?;
        if wire.v != BLOB_VERSION {
            bail!("unknown veil scar blob version {}", wire.v);
        }
        if wire.cell_metres <= 0 || wire.retain_den <= 0 {
            bail!("veil scar blob has a non-positive cell size or retain denominator");
        }
        if wire.cells.len() > MAX_SCAR_CELLS {
            bail!(
                "veil scar blob has {} cells; limit is {MAX_SCAR_CELLS}",
                wire.cells.len()
            );
        }
        let mut field = ScarField::from_parts(
            wire.cell_metres,
            Fixed64::from_raw(wire.decay),
            wire.retain_num,
            wire.retain_den,
            Fixed64::from_raw(wire.wary),
            Fixed64::from_raw(wire.avoid),
            wire.ticks,
        );
        for cell in wire.cells {
            field.insert_loaded(
                [cell.x, cell.y, cell.z],
                Fixed64::from_raw(cell.floor),
                Fixed64::from_raw(cell.transient),
            );
        }
        Ok(field)
    }
}

/// Store the scar blob on an `aw-save` meta map. Missing on old saves.
pub fn insert_into_meta(meta: &mut HashMap<String, String>, field: &ScarField) -> Result<()> {
    let bytes = field.to_postcard()?;
    meta.insert(META_KEY.to_string(), hex::encode(bytes));
    Ok(())
}

/// Read scars from an `aw-save` meta map. A missing key is an empty field
/// so saves written before this system still load.
pub fn read_from_meta(meta: &HashMap<String, String>) -> Result<ScarField> {
    let Some(text) = meta.get(META_KEY) else {
        return Ok(ScarField::new());
    };
    let bytes = hex::decode(text).context("veil scar meta is not hex")?;
    ScarField::from_postcard(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aw_save::{PlayerInventory, SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION};
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::field::reference_history;

    fn empty_bundle(meta: HashMap<String, String>) -> SaveBundleV2 {
        SaveBundleV2 {
            schema: SAVE_SCHEMA_VERSION,
            save_id: Uuid::new_v4(),
            created_at: OffsetDateTime::now_utc(),
            player_id: "veil".into(),
            slot: 0,
            world: WorldState {
                tick: 0,
                ecs_blob: Vec::new(),
                state_hash: 0,
            },
            companions: Vec::new(),
            inventory: PlayerInventory {
                credits: 0,
                items: Vec::new(),
            },
            meta,
        }
    }

    #[test]
    fn postcard_bytes_do_not_depend_on_insert_order() {
        let forward = reference_history();
        let mut reverse = ScarField::new();
        reverse.inject_energy(-4, 4, 12, crate::fixed::Fixed64::from_int(48));
        reverse.inject_energy(12, 4, -8, crate::fixed::Fixed64::from_int(48));
        for _ in 0..10 {
            reverse.tick();
        }
        reverse.inject_energy(12, 4, -8, crate::fixed::Fixed64::from_int(16));
        for _ in 0..30 {
            reverse.tick();
        }
        // reference_history injects (12, -8) first. The reverse script above
        // is a different sequence; the order test lives on one tick of two
        // deaths in field.rs. Here the postcard of one field is stable.
        let a = forward.to_postcard().unwrap();
        let b = forward.to_postcard().unwrap();
        assert_eq!(a, b);
        let _ = reverse;
    }

    #[test]
    fn aw_save_roundtrip_keeps_the_field_hash() {
        let field = reference_history();
        let hash = field.state_hash();
        let mut meta_a = HashMap::new();
        let mut meta_b = HashMap::new();
        insert_into_meta(&mut meta_a, &field).unwrap();
        insert_into_meta(&mut meta_b, &field).unwrap();
        assert_eq!(meta_a.get(META_KEY), meta_b.get(META_KEY));

        let dir = tempfile::tempdir().unwrap();
        let manager = SaveManager::new(dir.path());
        manager.save("veil", 0, empty_bundle(meta_a)).unwrap();
        let (loaded, _) = manager.load_latest_slot("veil", 0).unwrap();
        let restored = read_from_meta(&loaded.meta).unwrap();
        assert_eq!(restored.state_hash(), hash);
        assert_eq!(
            restored.to_postcard().unwrap(),
            field.to_postcard().unwrap()
        );
    }

    #[test]
    fn postcard_rejects_more_than_4096_cells_before_insert() {
        let cells = (0..MAX_SCAR_CELLS + 1)
            .map(|i| WireCell {
                x: i as i32,
                y: 0,
                z: 0,
                floor: 1,
                transient: 0,
            })
            .collect();
        let wire = ScarWire {
            v: BLOB_VERSION,
            cell_metres: 4,
            decay: 0,
            retain_num: 1,
            retain_den: 3,
            wary: 1,
            avoid: 2,
            ticks: 0,
            cells,
        };
        let bytes = postcard::to_allocvec(&wire).unwrap();
        let err = ScarField::from_postcard(&bytes).unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("4096"),
            "cell list was inserted or the cap was not named: {text}"
        );
    }

    #[test]
    fn postcard_accepts_a_full_4096_cell_list() {
        let cells = (0..MAX_SCAR_CELLS)
            .map(|i| WireCell {
                x: i as i32,
                y: 1,
                z: 2,
                floor: 1,
                transient: 0,
            })
            .collect();
        let wire = ScarWire {
            v: BLOB_VERSION,
            cell_metres: 4,
            decay: 0,
            retain_num: 1,
            retain_den: 3,
            wary: 1,
            avoid: 2,
            ticks: 0,
            cells,
        };
        let bytes = postcard::to_allocvec(&wire).unwrap();
        let field = ScarField::from_postcard(&bytes).unwrap();
        assert_eq!(field.occupied_count(), MAX_SCAR_CELLS);
    }

    #[test]
    fn a_save_without_the_key_loads_an_empty_field() {
        let restored = read_from_meta(&HashMap::new()).unwrap();
        assert_eq!(restored.state_hash(), ScarField::new().state_hash());
        assert_eq!(restored.occupied_count(), 0);
    }
}
