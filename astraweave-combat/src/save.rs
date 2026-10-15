//! Save meta for sub-unit health, poise, and the unused thread-strain hook.
//!
//! Keys match `aw_save::SaveBundleV2.meta`: `combat_tables` is the table hash,
//! `combat_vitals` is the little-endian record list. A hash mismatch refuses
//! to treat the blob as current content.

use std::collections::BTreeMap;

use crate::CombatError;

pub const META_TABLES: &str = "combat_tables";
pub const META_VITALS: &str = "combat_vitals";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThreadStrainSave {
    pub capacity: i64,
    pub reserve: i64,
    pub coherence: i64,
    pub strain: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VitalsRecord {
    pub entity: u64,
    pub hp: i64,
    pub hp_max: i64,
    pub poise: i64,
    pub poise_max: i64,
    pub strain: ThreadStrainSave,
}

pub trait MetaSink {
    fn insert_meta(&mut self, key: &str, value: String);
    fn get_meta(&self, key: &str) -> Option<&str>;
}

impl MetaSink for BTreeMap<String, String> {
    fn insert_meta(&mut self, key: &str, value: String) {
        self.insert(key.to_string(), value);
    }

    fn get_meta(&self, key: &str) -> Option<&str> {
        self.get(key).map(String::as_str)
    }
}

impl MetaSink for std::collections::HashMap<String, String> {
    fn insert_meta(&mut self, key: &str, value: String) {
        self.insert(key.to_string(), value);
    }

    fn get_meta(&self, key: &str) -> Option<&str> {
        self.get(key).map(String::as_str)
    }
}

pub fn check_table_hash(saved: &str, running_hex: &str) -> Result<(), CombatError> {
    if saved == running_hex {
        Ok(())
    } else {
        Err(CombatError::HashMismatch)
    }
}

pub fn encode_vitals(records: &mut [VitalsRecord]) -> String {
    records.sort_by_key(|r| r.entity);
    let mut bytes = Vec::with_capacity(4 + records.len() * 8 * 9);
    bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
    for rec in records.iter() {
        bytes.extend_from_slice(&rec.entity.to_le_bytes());
        bytes.extend_from_slice(&rec.hp.to_le_bytes());
        bytes.extend_from_slice(&rec.hp_max.to_le_bytes());
        bytes.extend_from_slice(&rec.poise.to_le_bytes());
        bytes.extend_from_slice(&rec.poise_max.to_le_bytes());
        bytes.extend_from_slice(&rec.strain.capacity.to_le_bytes());
        bytes.extend_from_slice(&rec.strain.reserve.to_le_bytes());
        bytes.extend_from_slice(&rec.strain.coherence.to_le_bytes());
        bytes.extend_from_slice(&rec.strain.strain.to_le_bytes());
    }
    hex_encode(&bytes)
}

pub fn decode_vitals(text: &str) -> Result<Vec<VitalsRecord>, CombatError> {
    let bytes = hex_decode(text)?;
    if bytes.len() < 4 {
        return Err(CombatError::BadSave);
    }
    let mut len_buf = [0u8; 4];
    len_buf.copy_from_slice(&bytes[..4]);
    let len = u32::from_le_bytes(len_buf) as usize;
    let body = &bytes[4..];
    let stride = 8 * 9;
    if body.len() != len * stride {
        return Err(CombatError::BadSave);
    }
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let s = &body[i * stride..(i + 1) * stride];
        out.push(VitalsRecord {
            entity: read_i64_as_u64(s, 0),
            hp: read_i64(s, 8),
            hp_max: read_i64(s, 16),
            poise: read_i64(s, 24),
            poise_max: read_i64(s, 32),
            strain: ThreadStrainSave {
                capacity: read_i64(s, 40),
                reserve: read_i64(s, 48),
                coherence: read_i64(s, 56),
                strain: read_i64(s, 64),
            },
        });
    }
    Ok(out)
}

pub fn write_combat_meta(
    meta: &mut impl MetaSink,
    table_hash_hex: &str,
    records: &mut [VitalsRecord],
) {
    meta.insert_meta(META_TABLES, table_hash_hex.to_string());
    meta.insert_meta(META_VITALS, encode_vitals(records));
}

pub fn read_combat_meta(
    meta: &impl MetaSink,
    running_hash_hex: &str,
) -> Result<Vec<VitalsRecord>, CombatError> {
    let saved = meta.get_meta(META_TABLES).ok_or(CombatError::BadSave)?;
    check_table_hash(saved, running_hash_hex)?;
    let vitals = meta.get_meta(META_VITALS).ok_or(CombatError::BadSave)?;
    decode_vitals(vitals)
}

fn read_i64(bytes: &[u8], at: usize) -> i64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[at..at + 8]);
    i64::from_le_bytes(buf)
}

fn read_i64_as_u64(bytes: &[u8], at: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(buf)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Result<Vec<u8>, CombatError> {
    if text.len() % 2 != 0 {
        return Err(CombatError::BadSave);
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(b: u8) -> Result<u8, CombatError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(CombatError::BadSave),
    }
}
