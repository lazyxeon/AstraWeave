//! Combat tables. Schools, curves, and abilities are data.
//!
//! The engine ceilings in this file are not moddable upward. A failed load
//! does not replace a table the caller already holds (`install`).
//!
//! The shipped default names a physical school, a `weave` school, and three
//! mundane hazard schools (fire, frost, poison). It does not ship shock, and
//! it does not use the labels Echo or Veil. Mods may add schools by id.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::fixed::Fixed64;
use crate::CombatError;

pub const MAX_MODIFIERS: usize = 8;
pub const MAX_DEFENDER_REDUCE_BP: u32 = 9_000;
pub const MIN_TAKEN_FLOOR_BP: u32 = 100;
pub const MAX_MITIGATION_CAP_BP: u32 = 9_000;
pub const MAX_PEN_CAP_BP: u32 = 9_000;
pub const MAX_CRIT_CHANCE_BP: u32 = 9_500;
pub const MAX_CRIT_MULT_BP: u32 = 30_000;
pub const MAX_EVASION_CAP_BP: u32 = 4_000;
pub const MAX_ATTACKER_BONUS_BP: u32 = 20_000;
pub const MAX_CURVE_KEYS: usize = 64;
pub const MAX_K_WHOLE: i64 = Fixed64::MAX_WHOLE;
pub const MIN_K_WHOLE: i64 = 1;
/// User modifier ids must stay below the reserved guard and bulwark ids.
pub const USER_MOD_ID_LIMIT: u32 = 0xFFFF_0000;
pub const BULWARK_MOD_ID: u32 = 0xFFFF_FF00;
pub const GUARD_MOD_ID: u32 = 0xFFFF_FF01;

/// How the default content presents a school. Mods may add ids; this enum
/// is the shipped vocabulary, not a closed set of spells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchoolPresentation {
    Physical,
    /// Mundane hazard. Not a spell school.
    Hazard,
    /// Causal-thread rating. Not "Veil" and not "Echo".
    Weave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicRating {
    None,
    Blight,
}

impl Default for PeriodicRating {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierK {
    pub tier: u16,
    pub k: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchoolDef {
    pub id: u32,
    pub name: String,
    pub presentation: SchoolPresentation,
    pub k_by_tier: Vec<TierK>,
    pub cap_bp: u32,
    #[serde(default)]
    pub periodic_rating: PeriodicRating,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurveDef {
    pub id: String,
    pub points: Vec<CurvePoint>,
    #[serde(default = "monotonic_by_default")]
    pub monotonic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityDef {
    pub id: u32,
    pub name: String,
    pub school: u32,
    pub weapon_min: i64,
    pub weapon_max: i64,
    /// Share of the rolled weapon, in basis points. `10_000` is the whole roll.
    pub weapon_bp: u32,
    pub might_num: u32,
    pub might_den: u32,
    pub school_num: u32,
    pub school_den: u32,
    pub blight_num: u32,
    pub blight_den: u32,
    pub flat: i64,
    pub poise: i64,
    pub variance_min_bp: u32,
    pub variance_max_bp: u32,
    #[serde(default)]
    pub bleed: bool,
    #[serde(default)]
    pub periodic: bool,
    #[serde(default)]
    pub always_crit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealDef {
    pub id: u32,
    pub name: String,
    pub mending_num: u32,
    pub mending_den: u32,
    pub flat: i64,
    #[serde(default)]
    pub always_crit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defaults {
    pub armor_cap_bp: u32,
    pub pen_cap_bp: u32,
    pub crit_cap_bp: u32,
    pub crit_base_bp: u32,
    pub crit_mult_cap_bp: u32,
    pub min_taken_bp: u32,
    pub glance_bp: u32,
    pub glance_chance_bp: u32,
    pub variance_min_bp: u32,
    pub variance_max_bp: u32,
    pub evasion_cap_bp: u32,
    pub attacker_bonus_cap_bp: u32,
    pub max_flat_whole: i64,
    pub stagger_ticks: u16,
    pub block_omnidirectional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TableFile {
    schema: u32,
    defaults: Defaults,
    schools: Vec<SchoolDef>,
    #[serde(default)]
    curves: Vec<CurveDef>,
    abilities: Vec<AbilityDef>,
    #[serde(default)]
    heals: Vec<HealDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ModFile {
    mod_id: String,
    schema: u32,
    #[serde(default)]
    schools: Vec<SchoolDef>,
    #[serde(default)]
    curves: Vec<CurveDef>,
    #[serde(default)]
    abilities: Vec<AbilityDef>,
    #[serde(default)]
    heals: Vec<HealDef>,
}

#[derive(Clone, Debug)]
pub struct CombatTables {
    schema: u32,
    defaults: Defaults,
    schools: BTreeMap<u32, SchoolDef>,
    curves: BTreeMap<String, CurveDef>,
    abilities: BTreeMap<u32, AbilityDef>,
    heals: BTreeMap<u32, HealDef>,
    hash: [u8; 32],
}

impl CombatTables {
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn hash_hex(&self) -> String {
        hex_encode(&self.hash)
    }

    pub fn defaults(&self) -> &Defaults {
        &self.defaults
    }

    pub fn school(&self, id: u32) -> Option<&SchoolDef> {
        self.schools.get(&id)
    }

    pub fn ability(&self, id: u32) -> Option<&AbilityDef> {
        self.abilities.get(&id)
    }

    pub fn heal(&self, id: u32) -> Option<&HealDef> {
        self.heals.get(&id)
    }

    pub fn curve(&self, id: &str) -> Option<&CurveDef> {
        self.curves.get(id)
    }

    pub fn schools(&self) -> impl Iterator<Item = &SchoolDef> {
        self.schools.values()
    }

    pub fn k_for(&self, school: &SchoolDef, tier: u16) -> Result<i64, CombatError> {
        school
            .k_by_tier
            .iter()
            .find(|row| row.tier == tier)
            .map(|row| row.k)
            .ok_or(CombatError::UnknownTier)
    }
}

impl CurveDef {
    /// Piecewise linear, truncating division. Clamps outside the end keys.
    pub fn eval(&self, x: i64) -> i64 {
        let pts = &self.points;
        if pts.is_empty() {
            return 0;
        }
        if x <= pts[0].x {
            return pts[0].y;
        }
        if x >= pts[pts.len() - 1].x {
            return pts[pts.len() - 1].y;
        }
        let mut i = 1;
        while i < pts.len() && pts[i].x < x {
            i += 1;
        }
        let right = &pts[i];
        let left = &pts[i - 1];
        let span = right.x - left.x;
        if span == 0 {
            return left.y;
        }
        let num = (x - left.x) as i128 * (right.y - left.y) as i128;
        left.y + (num / span as i128) as i64
    }
}

pub fn load_ron(text: &str) -> Result<CombatTables, CombatError> {
    let file: TableFile = ron::from_str(text).map_err(|e| CombatError::Ron(e.to_string()))?;
    finalize(file)
}

pub fn load_default() -> Result<CombatTables, CombatError> {
    load_ron(DEFAULT_TABLE_RON)
}

/// Apply mod texts in `mod_id` order. A school id that already exists must
/// keep its name. Unknown schema fails the load and leaves `base` unused.
pub fn apply_mods(base: &CombatTables, mod_texts: &[&str]) -> Result<CombatTables, CombatError> {
    let mut mods = Vec::with_capacity(mod_texts.len());
    for text in mod_texts {
        let parsed: ModFile = ron::from_str(text).map_err(|e| CombatError::Ron(e.to_string()))?;
        mods.push(parsed);
    }
    mods.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
    let mut file = TableFile {
        schema: base.schema,
        defaults: base.defaults.clone(),
        schools: base.schools.values().cloned().collect(),
        curves: base.curves.values().cloned().collect(),
        abilities: base.abilities.values().cloned().collect(),
        heals: base.heals.values().cloned().collect(),
    };
    for m in mods {
        if m.schema != 1 {
            return Err(CombatError::BadSchema);
        }
        for school in m.schools {
            if let Some(prev) = file.schools.iter().find(|s| s.id == school.id) {
                if prev.name != school.name {
                    return Err(CombatError::SchoolRename);
                }
            }
            file.schools.retain(|s| s.id != school.id);
            file.schools.push(school);
        }
        for curve in m.curves {
            file.curves.retain(|c| c.id != curve.id);
            file.curves.push(curve);
        }
        for ability in m.abilities {
            file.abilities.retain(|a| a.id != ability.id);
            file.abilities.push(ability);
        }
        for heal in m.heals {
            file.heals.retain(|h| h.id != heal.id);
            file.heals.push(heal);
        }
    }
    finalize(file)
}

pub fn install(
    slot: &mut Option<std::sync::Arc<CombatTables>>,
    incoming: Result<CombatTables, CombatError>,
) -> Result<(), CombatError> {
    match incoming {
        Ok(tables) => {
            *slot = Some(std::sync::Arc::new(tables));
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn finalize(file: TableFile) -> Result<CombatTables, CombatError> {
    if file.schema != 1 {
        return Err(CombatError::BadSchema);
    }
    validate_defaults(&file.defaults)?;
    let mut schools = BTreeMap::new();
    for school in file.schools {
        validate_school(&school)?;
        if schools.insert(school.id, school).is_some() {
            return Err(CombatError::DuplicateId);
        }
    }
    let mut curves = BTreeMap::new();
    for curve in file.curves {
        validate_curve(&curve)?;
        if curves.insert(curve.id.clone(), curve).is_some() {
            return Err(CombatError::DuplicateId);
        }
    }
    let mut abilities = BTreeMap::new();
    for ability in file.abilities {
        validate_ability(&ability, &schools)?;
        if abilities.insert(ability.id, ability).is_some() {
            return Err(CombatError::DuplicateId);
        }
    }
    let mut heals = BTreeMap::new();
    for heal in file.heals {
        if heal.mending_den == 0 {
            return Err(CombatError::BadCoeff);
        }
        if heals.insert(heal.id, heal).is_some() {
            return Err(CombatError::DuplicateId);
        }
    }
    let hash = semantic_hash(
        file.schema,
        &file.defaults,
        &schools,
        &curves,
        &abilities,
        &heals,
    );
    Ok(CombatTables {
        schema: file.schema,
        defaults: file.defaults,
        schools,
        curves,
        abilities,
        heals,
        hash,
    })
}

fn validate_defaults(d: &Defaults) -> Result<(), CombatError> {
    if d.min_taken_bp < MIN_TAKEN_FLOOR_BP || d.min_taken_bp > 10_000 {
        return Err(CombatError::Ceiling("min_taken"));
    }
    if d.armor_cap_bp == 0 || d.armor_cap_bp > MAX_MITIGATION_CAP_BP {
        return Err(CombatError::Ceiling("armor_cap"));
    }
    if d.pen_cap_bp > MAX_PEN_CAP_BP {
        return Err(CombatError::Ceiling("pen_cap"));
    }
    if d.crit_cap_bp > MAX_CRIT_CHANCE_BP {
        return Err(CombatError::Ceiling("crit_cap"));
    }
    if d.crit_base_bp < 10_000 || d.crit_mult_cap_bp > MAX_CRIT_MULT_BP {
        return Err(CombatError::Ceiling("crit_mult"));
    }
    if d.crit_base_bp > d.crit_mult_cap_bp {
        return Err(CombatError::Ceiling("crit_base"));
    }
    if d.evasion_cap_bp > MAX_EVASION_CAP_BP {
        return Err(CombatError::Ceiling("evasion"));
    }
    if d.attacker_bonus_cap_bp > MAX_ATTACKER_BONUS_BP {
        return Err(CombatError::Ceiling("attacker_bonus"));
    }
    if d.variance_max_bp < d.variance_min_bp || d.variance_max_bp > 20_000 {
        return Err(CombatError::Ceiling("variance"));
    }
    if d.glance_bp > 10_000 || d.glance_chance_bp > 10_000 {
        return Err(CombatError::Ceiling("glance"));
    }
    if d.max_flat_whole < 0 || d.max_flat_whole > Fixed64::MAX_WHOLE {
        return Err(CombatError::Ceiling("max_flat"));
    }
    if d.stagger_ticks == 0 {
        return Err(CombatError::Ceiling("stagger"));
    }
    Ok(())
}

fn validate_school(school: &SchoolDef) -> Result<(), CombatError> {
    if school.k_by_tier.is_empty() {
        return Err(CombatError::KOutOfRange);
    }
    let mut seen = BTreeMap::new();
    for row in &school.k_by_tier {
        if row.k < MIN_K_WHOLE || row.k > MAX_K_WHOLE {
            return Err(CombatError::KOutOfRange);
        }
        if seen.insert(row.tier, ()).is_some() {
            return Err(CombatError::DuplicateId);
        }
    }
    if school.cap_bp == 0 || school.cap_bp > MAX_MITIGATION_CAP_BP {
        return Err(CombatError::Ceiling("school_cap"));
    }
    if school.name.eq_ignore_ascii_case("veil") || school.name.eq_ignore_ascii_case("echo") {
        return Err(CombatError::ForbiddenName);
    }
    Ok(())
}

fn validate_curve(curve: &CurveDef) -> Result<(), CombatError> {
    if curve.points.is_empty() || curve.points.len() > MAX_CURVE_KEYS {
        return Err(CombatError::BadCurve);
    }
    for pair in curve.points.windows(2) {
        if pair[1].x < pair[0].x {
            return Err(CombatError::BadCurve);
        }
        if curve.monotonic && pair[1].y < pair[0].y {
            return Err(CombatError::BadCurve);
        }
    }
    Ok(())
}

fn validate_ability(
    ability: &AbilityDef,
    schools: &BTreeMap<u32, SchoolDef>,
) -> Result<(), CombatError> {
    let school = schools
        .get(&ability.school)
        .ok_or(CombatError::UnknownSchool)?;
    if ability.might_den == 0 || ability.school_den == 0 || ability.blight_den == 0 {
        return Err(CombatError::BadCoeff);
    }
    if ability.weapon_max < ability.weapon_min {
        return Err(CombatError::StatOutOfRange);
    }
    if ability.variance_max_bp < ability.variance_min_bp {
        return Err(CombatError::Ceiling("variance"));
    }
    for whole in [
        ability.weapon_min,
        ability.weapon_max,
        ability.flat,
        ability.poise,
    ] {
        if !(0..=Fixed64::MAX_WHOLE).contains(&whole) {
            return Err(CombatError::StatOutOfRange);
        }
    }
    if ability.bleed && (!ability.periodic || school.presentation != SchoolPresentation::Physical) {
        return Err(CombatError::BadBleed);
    }
    Ok(())
}

fn semantic_hash(
    schema: u32,
    defaults: &Defaults,
    schools: &BTreeMap<u32, SchoolDef>,
    curves: &BTreeMap<String, CurveDef>,
    abilities: &BTreeMap<u32, AbilityDef>,
    heals: &BTreeMap<u32, HealDef>,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(schema.to_le_bytes());
    hash_defaults(&mut h, defaults);
    for school in schools.values() {
        h.update(school.id.to_le_bytes());
        h.update((school.name.len() as u32).to_le_bytes());
        h.update(school.name.as_bytes());
        h.update([presentation_tag(school.presentation)]);
        h.update([periodic_tag(school.periodic_rating)]);
        h.update(school.cap_bp.to_le_bytes());
        let mut rows = school.k_by_tier.clone();
        rows.sort_by_key(|r| r.tier);
        h.update((rows.len() as u32).to_le_bytes());
        for row in rows {
            h.update(row.tier.to_le_bytes());
            h.update(row.k.to_le_bytes());
        }
    }
    for curve in curves.values() {
        h.update((curve.id.len() as u32).to_le_bytes());
        h.update(curve.id.as_bytes());
        h.update([u8::from(curve.monotonic)]);
        h.update((curve.points.len() as u32).to_le_bytes());
        for p in &curve.points {
            h.update(p.x.to_le_bytes());
            h.update(p.y.to_le_bytes());
        }
    }
    for ability in abilities.values() {
        hash_ability(&mut h, ability);
    }
    for heal in heals.values() {
        h.update(heal.id.to_le_bytes());
        h.update((heal.name.len() as u32).to_le_bytes());
        h.update(heal.name.as_bytes());
        h.update(heal.mending_num.to_le_bytes());
        h.update(heal.mending_den.to_le_bytes());
        h.update(heal.flat.to_le_bytes());
        h.update([u8::from(heal.always_crit)]);
    }
    h.finalize().into()
}

fn hash_defaults(h: &mut Sha256, d: &Defaults) {
    h.update(d.armor_cap_bp.to_le_bytes());
    h.update(d.pen_cap_bp.to_le_bytes());
    h.update(d.crit_cap_bp.to_le_bytes());
    h.update(d.crit_base_bp.to_le_bytes());
    h.update(d.crit_mult_cap_bp.to_le_bytes());
    h.update(d.min_taken_bp.to_le_bytes());
    h.update(d.glance_bp.to_le_bytes());
    h.update(d.glance_chance_bp.to_le_bytes());
    h.update(d.variance_min_bp.to_le_bytes());
    h.update(d.variance_max_bp.to_le_bytes());
    h.update(d.evasion_cap_bp.to_le_bytes());
    h.update(d.attacker_bonus_cap_bp.to_le_bytes());
    h.update(d.max_flat_whole.to_le_bytes());
    h.update(d.stagger_ticks.to_le_bytes());
    h.update([u8::from(d.block_omnidirectional)]);
}

fn hash_ability(h: &mut Sha256, a: &AbilityDef) {
    h.update(a.id.to_le_bytes());
    h.update((a.name.len() as u32).to_le_bytes());
    h.update(a.name.as_bytes());
    h.update(a.school.to_le_bytes());
    h.update(a.weapon_min.to_le_bytes());
    h.update(a.weapon_max.to_le_bytes());
    h.update(a.weapon_bp.to_le_bytes());
    h.update(a.might_num.to_le_bytes());
    h.update(a.might_den.to_le_bytes());
    h.update(a.school_num.to_le_bytes());
    h.update(a.school_den.to_le_bytes());
    h.update(a.blight_num.to_le_bytes());
    h.update(a.blight_den.to_le_bytes());
    h.update(a.flat.to_le_bytes());
    h.update(a.poise.to_le_bytes());
    h.update(a.variance_min_bp.to_le_bytes());
    h.update(a.variance_max_bp.to_le_bytes());
    h.update([
        u8::from(a.bleed),
        u8::from(a.periodic),
        u8::from(a.always_crit),
    ]);
}

fn presentation_tag(p: SchoolPresentation) -> u8 {
    match p {
        SchoolPresentation::Physical => 1,
        SchoolPresentation::Hazard => 2,
        SchoolPresentation::Weave => 3,
    }
}

fn periodic_tag(p: PeriodicRating) -> u8 {
    match p {
        PeriodicRating::None => 0,
        PeriodicRating::Blight => 1,
    }
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

/// Shipped content. Physical sword and shield numbers match the greybox
/// scale (flat 25 and flat 12). Fire, frost, and poison are hazards.
/// `weave` is the non-physical school. Shock is not listed.
pub const DEFAULT_TABLE_RON: &str = r#"
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
        variance_min_bp: 9500,
        variance_max_bp: 10500,
        evasion_cap_bp: 0,
        attacker_bonus_cap_bp: 10000,
        max_flat_whole: 1000000,
        stagger_ticks: 12,
        block_omnidirectional: true,
    ),
    schools: [
        (
            id: 1,
            name: "physical",
            presentation: physical,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
        (
            id: 2,
            name: "weave",
            presentation: weave,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
        (
            id: 3,
            name: "fire",
            presentation: hazard,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
        (
            id: 4,
            name: "frost",
            presentation: hazard,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
        (
            id: 6,
            name: "poison",
            presentation: hazard,
            k_by_tier: [(tier: 0, k: 100)],
            cap_bp: 7500,
            periodic_rating: blight,
        ),
    ],
    curves: [
        (
            id: "precision_to_crit",
            monotonic: true,
            points: [(x: 0, y: 0), (x: 100, y: 1500), (x: 400, y: 4500)],
        ),
        (
            id: "guard_to_bp",
            monotonic: true,
            points: [(x: 0, y: 0), (x: 100, y: 1000)],
        ),
    ],
    abilities: [
        (
            id: 1,
            name: "sword_light",
            school: 1,
            weapon_min: 25,
            weapon_max: 25,
            weapon_bp: 10000,
            might_num: 0,
            might_den: 1,
            school_num: 0,
            school_den: 1,
            blight_num: 0,
            blight_den: 1,
            flat: 0,
            poise: 10,
            variance_min_bp: 10000,
            variance_max_bp: 10000,
            bleed: false,
            periodic: false,
            always_crit: false,
        ),
        (
            id: 2,
            name: "enemy_swipe",
            school: 1,
            weapon_min: 12,
            weapon_max: 12,
            weapon_bp: 10000,
            might_num: 0,
            might_den: 1,
            school_num: 0,
            school_den: 1,
            blight_num: 0,
            blight_den: 1,
            flat: 0,
            poise: 8,
            variance_min_bp: 10000,
            variance_max_bp: 10000,
            bleed: false,
            periodic: false,
            always_crit: false,
        ),
    ],
    heals: [
        (
            id: 1,
            name: "bandage",
            mending_num: 1,
            mending_den: 1,
            flat: 0,
            always_crit: false,
        ),
    ],
)
"#;

fn monotonic_by_default() -> bool {
    true
}

/// Canonical pipeline fixture. K = 400 so Armor == K is the published case.
pub fn canonical_tables() -> Result<CombatTables, CombatError> {
    let ron = r#"
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
        (
            id: 1,
            name: "physical",
            presentation: physical,
            k_by_tier: [(tier: 0, k: 400)],
            cap_bp: 7500,
            periodic_rating: none,
        ),
    ],
    curves: [],
    abilities: [
        (
            id: 1,
            name: "canonical_swing",
            school: 1,
            weapon_min: 20,
            weapon_max: 20,
            weapon_bp: 10000,
            might_num: 1,
            might_den: 10,
            school_num: 0,
            school_den: 1,
            blight_num: 0,
            blight_den: 1,
            flat: 0,
            poise: 0,
            variance_min_bp: 10000,
            variance_max_bp: 10000,
            bleed: false,
            periodic: false,
            always_crit: false,
        ),
    ],
    heals: [],
)
"#;
    load_ron(ron)
}
