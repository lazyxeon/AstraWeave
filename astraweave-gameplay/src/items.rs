use crate::{DamageType, ResourceKind};
use serde::{Deserialize, Serialize};

pub type ItemId = u32;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ItemKind {
    Weapon { base_damage: i32, dtype: DamageType },
    Armor { defense: i32 },
    Consumable { heal: i32 },
    Material { r#type: ResourceKind },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EchoMod {
    pub name: String,
    pub power_mult: f32, // multiplies base damage / power
    pub dtype_override: Option<DamageType>,
    pub special: Option<String>, // freeform tag for special behaviours
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub kind: ItemKind,
    pub echo: Option<EchoMod>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub resources: Vec<(ResourceKind, u32)>,
}

impl Inventory {
    pub fn add_resource(&mut self, kind: ResourceKind, n: u32) {
        if let Some((_, c)) = self.resources.iter_mut().find(|(k, _)| *k == kind) {
            *c += n;
        } else {
            self.resources.push((kind, n));
        }
    }

    pub fn remove_resource(&mut self, kind: ResourceKind, n: u32) -> bool {
        if let Some((_, c)) = self.resources.iter_mut().find(|(k, _)| *k == kind) {
            if *c >= n {
                *c -= n;
                return true;
            }
        }
        false
    }
}

pub fn infuse(item: &mut Item, echo: EchoMod) {
    item.echo = Some(echo);
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EchoModDef {
    pub name: String,
    pub rarity: Rarity,
    pub power_mult: f32,
    pub dtype_override: Option<DamageType>,
    pub special: Option<String>,
}

pub fn load_echo_defs(toml_txt: &str) -> anyhow::Result<Vec<EchoModDef>> {
    #[derive(Deserialize)]
    struct File {
        echoes: Vec<EchoModDef>,
    }
    let f: File = toml::from_str(toml_txt)?;
    Ok(f.echoes)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::{test_panic, TestValue};

    #[test]
    fn test_item_kind_weapon() {
        let weapon = ItemKind::Weapon {
            base_damage: 25,
            dtype: DamageType::Physical,
        };
        if let ItemKind::Weapon { base_damage, dtype } = weapon {
            assert_eq!(base_damage, 25);
            assert!(matches!(dtype, DamageType::Physical));
        } else {
            test_panic(format_args!("Expected Weapon"));
        }
    }

    #[test]
    fn test_item_kind_armor() {
        let armor = ItemKind::Armor { defense: 15 };
        if let ItemKind::Armor { defense } = armor {
            assert_eq!(defense, 15);
        } else {
            test_panic(format_args!("Expected Armor"));
        }
    }

    #[test]
    fn test_item_kind_consumable() {
        let potion = ItemKind::Consumable { heal: 50 };
        if let ItemKind::Consumable { heal } = potion {
            assert_eq!(heal, 50);
        } else {
            test_panic(format_args!("Expected Consumable"));
        }
    }

    #[test]
    fn test_item_kind_material() {
        let material = ItemKind::Material {
            r#type: ResourceKind::Crystal,
        };
        if let ItemKind::Material { r#type } = material {
            assert_eq!(r#type, ResourceKind::Crystal);
        } else {
            test_panic(format_args!("Expected Material"));
        }
    }

    #[test]
    fn test_echo_mod_creation() {
        let echo = EchoMod {
            name: "Fire Infusion".to_string(),
            power_mult: 1.5,
            dtype_override: Some(DamageType::Fire),
            special: Some("burn".to_string()),
        };
        assert_eq!(echo.name, "Fire Infusion");
        assert!((echo.power_mult - 1.5).abs() < f32::EPSILON);
        assert!(matches!(echo.dtype_override, Some(DamageType::Fire)));
    }

    #[test]
    fn test_echo_mod_no_override() {
        let echo = EchoMod {
            name: "Basic".to_string(),
            power_mult: 1.0,
            dtype_override: None,
            special: None,
        };
        assert!(echo.dtype_override.is_none());
        assert!(echo.special.is_none());
    }

    #[test]
    fn test_item_creation() {
        let item = Item {
            id: 42,
            name: "Iron Sword".to_string(),
            kind: ItemKind::Weapon {
                base_damage: 20,
                dtype: DamageType::Physical,
            },
            echo: None,
        };
        assert_eq!(item.id, 42);
        assert_eq!(item.name, "Iron Sword");
        assert!(item.echo.is_none());
    }

    #[test]
    fn test_inventory_default() {
        let inv = Inventory::default();
        assert!(inv.items.is_empty());
        assert!(inv.resources.is_empty());
    }

    #[test]
    fn test_inventory_add_resource_new() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        assert_eq!(inv.resources.len(), 1);
        assert_eq!(inv.resources[0], (ResourceKind::Wood, 10));
    }

    #[test]
    fn test_inventory_add_resource_existing() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        inv.add_resource(ResourceKind::Wood, 5);
        assert_eq!(inv.resources.len(), 1);
        assert_eq!(inv.resources[0], (ResourceKind::Wood, 15));
    }

    #[test]
    fn test_inventory_add_different_resources() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        inv.add_resource(ResourceKind::Crystal, 5);
        inv.add_resource(ResourceKind::Ore, 20);
        assert_eq!(inv.resources.len(), 3);
    }

    #[test]
    fn test_inventory_remove_resource_success() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        let result = inv.remove_resource(ResourceKind::Wood, 5);
        assert!(result);
        assert_eq!(inv.resources[0], (ResourceKind::Wood, 5));
    }

    #[test]
    fn test_inventory_remove_resource_exact_amount() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Crystal, 10);
        let result = inv.remove_resource(ResourceKind::Crystal, 10);
        assert!(result);
        assert_eq!(inv.resources[0], (ResourceKind::Crystal, 0));
    }

    #[test]
    fn test_inventory_remove_resource_insufficient() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 5);
        let result = inv.remove_resource(ResourceKind::Wood, 10);
        assert!(!result);
        assert_eq!(inv.resources[0], (ResourceKind::Wood, 5));
    }

    #[test]
    fn test_inventory_remove_resource_not_found() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        let result = inv.remove_resource(ResourceKind::Crystal, 5);
        assert!(!result);
    }

    #[test]
    fn test_infuse_item() {
        let mut item = Item {
            id: 1,
            name: "Sword".to_string(),
            kind: ItemKind::Weapon {
                base_damage: 10,
                dtype: DamageType::Physical,
            },
            echo: None,
        };
        let echo = EchoMod {
            name: "Frost".to_string(),
            power_mult: 1.2,
            dtype_override: Some(DamageType::Frost),
            special: None,
        };
        infuse(&mut item, echo);
        assert!(item.echo.is_some());
        assert_eq!(item.echo.as_ref().test_unwrap().name, "Frost");
    }

    #[test]
    fn test_infuse_replaces_existing_echo() {
        let mut item = Item {
            id: 1,
            name: "Sword".to_string(),
            kind: ItemKind::Weapon {
                base_damage: 10,
                dtype: DamageType::Physical,
            },
            echo: Some(EchoMod {
                name: "Old".to_string(),
                power_mult: 1.0,
                dtype_override: None,
                special: None,
            }),
        };
        let new_echo = EchoMod {
            name: "New".to_string(),
            power_mult: 2.0,
            dtype_override: None,
            special: None,
        };
        infuse(&mut item, new_echo);
        assert_eq!(item.echo.as_ref().test_unwrap().name, "New");
    }

    #[test]
    fn test_rarity_ordering() {
        assert_eq!(Rarity::Common, Rarity::Common);
        assert_ne!(Rarity::Common, Rarity::Legendary);
    }

    #[test]
    fn test_load_echo_defs_valid_toml() {
        let toml = r#"
[[echoes]]
name = "Fire Storm"
rarity = "Epic"
power_mult = 1.5
dtype_override = "Fire"
special = "burn"

[[echoes]]
name = "Ice Shard"
rarity = "Rare"
power_mult = 1.3
"#;
        let defs = load_echo_defs(toml).test_expect("parse toml");
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "Fire Storm");
        assert_eq!(defs[0].rarity, Rarity::Epic);
        assert_eq!(defs[1].name, "Ice Shard");
        assert_eq!(defs[1].rarity, Rarity::Rare);
    }

    #[test]
    fn test_load_echo_defs_empty() {
        let toml = "echoes = []";
        let defs = load_echo_defs(toml).test_expect("parse toml");
        assert!(defs.is_empty());
    }

    #[test]
    fn test_load_echo_defs_invalid_toml() {
        let toml = "this is not valid toml {{{";
        let result = load_echo_defs(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_item_serialization() {
        let item = Item {
            id: 100,
            name: "Test Item".to_string(),
            kind: ItemKind::Consumable { heal: 25 },
            echo: None,
        };
        let json = serde_json::to_string(&item).test_expect("serialize");
        let deserialized: Item = serde_json::from_str(&json).test_expect("deserialize");
        assert_eq!(deserialized.id, 100);
        assert_eq!(deserialized.name, "Test Item");
    }

    #[test]
    fn test_inventory_serialization() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 50);
        let json = serde_json::to_string(&inv).test_expect("serialize");
        let deserialized: Inventory = serde_json::from_str(&json).test_expect("deserialize");
        assert_eq!(deserialized.resources.len(), 1);
        assert_eq!(deserialized.resources[0], (ResourceKind::Wood, 50));
    }

    #[test]
    fn test_echo_mod_serialization() {
        let echo = EchoMod {
            name: "Test Echo".to_string(),
            power_mult: 1.75,
            dtype_override: Some(DamageType::Shock),
            special: Some("stun".to_string()),
        };
        let json = serde_json::to_string(&echo).test_expect("serialize");
        let deserialized: EchoMod = serde_json::from_str(&json).test_expect("deserialize");
        assert_eq!(deserialized.name, "Test Echo");
        assert!(matches!(
            deserialized.dtype_override,
            Some(DamageType::Shock)
        ));
    }

    #[test]
    fn test_load_echo_defs_all_rarities() {
        let toml = r#"
[[echoes]]
name = "Common Echo"
rarity = "Common"
power_mult = 1.0

[[echoes]]
name = "Uncommon Echo"
rarity = "Uncommon"
power_mult = 1.1

[[echoes]]
name = "Rare Echo"
rarity = "Rare"
power_mult = 1.2

[[echoes]]
name = "Epic Echo"
rarity = "Epic"
power_mult = 1.4

[[echoes]]
name = "Legendary Echo"
rarity = "Legendary"
power_mult = 2.0
"#;
        let defs = load_echo_defs(toml).test_expect("parse toml");
        assert_eq!(defs.len(), 5);
        assert_eq!(defs[0].rarity, Rarity::Common);
        assert_eq!(defs[1].rarity, Rarity::Uncommon);
        assert_eq!(defs[2].rarity, Rarity::Rare);
        assert_eq!(defs[3].rarity, Rarity::Epic);
        assert_eq!(defs[4].rarity, Rarity::Legendary);
    }
}
