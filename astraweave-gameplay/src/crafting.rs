use crate::{Inventory, Item, ItemKind, ResourceKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CraftCost {
    pub kind: ResourceKind,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CraftRecipe {
    pub name: String,
    pub output_item: ItemKind,
    pub costs: Vec<CraftCost>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeBook {
    pub recipes: Vec<CraftRecipe>,
}

impl RecipeBook {
    /// Craft with the sim stream for `(world_seed, tick, SIM_SYSTEM_CRAFT)`.
    ///
    /// Item ids are a pure function of that triple and of how many `u32` draws
    /// this call consumes (one).
    pub fn craft_with_seed(
        &self,
        name: &str,
        inv: &mut Inventory,
        world_seed: u64,
        tick: u64,
    ) -> Option<Item> {
        let mut rng =
            astraweave_core::SimRng::new(world_seed, tick, astraweave_core::SIM_SYSTEM_CRAFT);
        self.craft_seeded(name, inv, &mut rng)
    }

    /// Deterministic craft using seeded RNG.
    ///
    /// # Determinism
    ///
    /// This method guarantees identical item IDs given:
    /// - Same recipe name
    /// - Same inventory state
    /// - Same RNG state
    ///
    /// Use this for gameplay systems requiring determinism (multiplayer, replay).
    pub fn craft_seeded<R: rand::Rng>(
        &self,
        name: &str,
        inv: &mut Inventory,
        rng: &mut R,
    ) -> Option<Item> {
        let r = self.recipes.iter().find(|r| r.name == name)?;
        // check costs
        for c in &r.costs {
            let have = inv
                .resources
                .iter()
                .find(|(k, _)| *k == c.kind)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            if have < c.count {
                return None;
            }
        }
        // pay costs
        for c in &r.costs {
            if !inv.remove_resource(c.kind, c.count) {
                return None;
            }
        }
        // create item with deterministic ID
        let itm = Item {
            id: rng.random::<u32>(),
            name: r.name.clone(),
            kind: r.output_item.clone(),
            echo: None,
        };
        Some(itm)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactionStanding {
    pub name: String,
    pub reputation: i32,
} // -100..100

#[derive(Clone, Debug)]
pub struct CraftBench {
    pub quality: i32, // -2..+3
}

impl CraftBench {
    pub fn success_chance(
        &self,
        player_power: i32,
        faction: Option<&FactionStanding>,
        rarity: Option<&crate::items::Rarity>,
    ) -> f32 {
        let base = 0.75 + (self.quality as f32) * 0.05 + (player_power as f32) * 0.003;
        let fac = faction
            .map(|f| (f.reputation as f32) * 0.001)
            .unwrap_or(0.0);
        let rarity_penalty = match rarity {
            Some(crate::items::Rarity::Epic) => -0.15,
            Some(crate::items::Rarity::Legendary) => -0.30,
            _ => 0.0,
        };
        (base + fac + rarity_penalty).clamp(0.05, 0.98)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DamageType, Rarity};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(12345)
    }

    #[test]
    fn test_craft_cost_creation() {
        let cost = CraftCost {
            kind: ResourceKind::Wood,
            count: 5,
        };
        assert_eq!(cost.kind, ResourceKind::Wood);
        assert_eq!(cost.count, 5);
    }

    #[test]
    fn test_craft_recipe_creation() {
        let recipe = CraftRecipe {
            name: "Iron Sword".to_string(),
            output_item: ItemKind::Weapon {
                base_damage: 20,
                dtype: DamageType::Physical,
            },
            costs: vec![CraftCost {
                kind: ResourceKind::Ore,
                count: 3,
            }],
        };
        assert_eq!(recipe.name, "Iron Sword");
        assert_eq!(recipe.costs.len(), 1);
    }

    #[test]
    fn test_recipe_book_craft_success() {
        use crate::test_support::TestValue;
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Basic Armor".to_string(),
                output_item: ItemKind::Armor { defense: 10 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 5,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 10);

        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Basic Armor", &mut inv, &mut rng);
        assert!(item.is_some());
        let item = item.test_unwrap();
        assert_eq!(item.name, "Basic Armor");
        // Resources should be consumed
        assert_eq!(inv.resources[0], (ResourceKind::Ore, 5));
    }

    #[test]
    fn test_recipe_book_craft_insufficient_resources() {
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Expensive Item".to_string(),
                output_item: ItemKind::Consumable { heal: 100 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Crystal,
                    count: 10,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Crystal, 5); // Not enough

        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Expensive Item", &mut inv, &mut rng);
        assert!(item.is_none());
        // Resources should NOT be consumed
        assert_eq!(inv.resources[0], (ResourceKind::Crystal, 5));
    }

    #[test]
    fn test_recipe_book_craft_recipe_not_found() {
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Iron Sword".to_string(),
                output_item: ItemKind::Weapon {
                    base_damage: 15,
                    dtype: DamageType::Physical,
                },
                costs: vec![],
            }],
        };
        let mut inv = Inventory::default();

        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Nonexistent Recipe", &mut inv, &mut rng);
        assert!(item.is_none());
    }

    #[test]
    fn test_recipe_book_craft_multiple_costs() {
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Complex Item".to_string(),
                output_item: ItemKind::Armor { defense: 25 },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Ore,
                        count: 5,
                    },
                    CraftCost {
                        kind: ResourceKind::Fiber,
                        count: 3,
                    },
                    CraftCost {
                        kind: ResourceKind::Essence,
                        count: 1,
                    },
                ],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 10);
        inv.add_resource(ResourceKind::Fiber, 5);
        inv.add_resource(ResourceKind::Essence, 2);

        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Complex Item", &mut inv, &mut rng);
        assert!(item.is_some());
    }

    #[test]
    fn test_recipe_book_craft_missing_one_resource() {
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Item".to_string(),
                output_item: ItemKind::Consumable { heal: 10 },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Wood,
                        count: 2,
                    },
                    CraftCost {
                        kind: ResourceKind::Crystal,
                        count: 1,
                    },
                ],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 5);
        // Missing Crystal

        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Item", &mut inv, &mut rng);
        assert!(item.is_none());
    }

    #[test]
    fn test_faction_standing_creation() {
        let faction = FactionStanding {
            name: "Merchants Guild".to_string(),
            reputation: 50,
        };
        assert_eq!(faction.name, "Merchants Guild");
        assert_eq!(faction.reputation, 50);
    }

    #[test]
    fn test_craft_bench_creation() {
        let bench = CraftBench { quality: 2 };
        assert_eq!(bench.quality, 2);
    }

    #[test]
    fn test_craft_bench_success_chance_base() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75 + 0 + 0 = 0.75
        let chance = bench.success_chance(0, None, None);
        assert!((chance - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_with_quality() {
        let bench = CraftBench { quality: 3 };
        // base = 0.75 + 3 * 0.05 + 0 = 0.90
        let chance = bench.success_chance(0, None, None);
        assert!((chance - 0.90).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_with_power() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75 + 0 + 100 * 0.003 = 1.05, clamped to 0.98
        let chance = bench.success_chance(100, None, None);
        assert!((chance - 0.98).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_with_faction() {
        let bench = CraftBench { quality: 0 };
        let faction = FactionStanding {
            name: "Smiths".to_string(),
            reputation: 100,
        };
        // base = 0.75 + 0 + 0 = 0.75, fac = 100 * 0.001 = 0.1, total = 0.85
        let chance = bench.success_chance(0, Some(&faction), None);
        assert!((chance - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_negative_faction() {
        let bench = CraftBench { quality: 0 };
        let faction = FactionStanding {
            name: "Enemies".to_string(),
            reputation: -50,
        };
        // base = 0.75, fac = -0.05, total = 0.70
        let chance = bench.success_chance(0, Some(&faction), None);
        assert!((chance - 0.70).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_epic_rarity() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75, rarity_penalty = -0.15, total = 0.60
        let chance = bench.success_chance(0, None, Some(&Rarity::Epic));
        assert!((chance - 0.60).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_legendary_rarity() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75, rarity_penalty = -0.30, total = 0.45
        let chance = bench.success_chance(0, None, Some(&Rarity::Legendary));
        assert!((chance - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_common_rarity() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75, rarity_penalty = 0.0, total = 0.75
        let chance = bench.success_chance(0, None, Some(&Rarity::Common));
        assert!((chance - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_craft_bench_success_chance_combined() {
        let bench = CraftBench { quality: 2 };
        let faction = FactionStanding {
            name: "Allies".to_string(),
            reputation: 50,
        };
        // base = 0.75 + 0.10 + 10 * 0.003 = 0.88
        // fac = 0.05
        // rarity = -0.15
        // total = 0.78
        let chance = bench.success_chance(10, Some(&faction), Some(&Rarity::Epic));
        assert!((chance - 0.78).abs() < 0.01);
    }

    #[test]
    fn test_craft_bench_success_chance_clamp_min() {
        let bench = CraftBench { quality: -2 };
        let faction = FactionStanding {
            name: "Enemies".to_string(),
            reputation: -100,
        };
        // base = 0.75 - 0.10 + 0 = 0.65
        // fac = -0.1
        // rarity = -0.30
        // total = 0.25 -> still above 0.05
        let chance = bench.success_chance(0, Some(&faction), Some(&Rarity::Legendary));
        assert!(chance >= 0.05);
    }

    #[test]
    fn test_craft_bench_success_chance_clamp_max() {
        let bench = CraftBench { quality: 3 };
        let faction = FactionStanding {
            name: "Best Friends".to_string(),
            reputation: 100,
        };
        // Even with max bonuses, should clamp to 0.98
        let chance = bench.success_chance(100, Some(&faction), Some(&Rarity::Common));
        assert!((chance - 0.98).abs() < 0.001);
    }

    #[test]
    fn test_recipe_book_empty() {
        let recipe_book = RecipeBook { recipes: vec![] };
        let mut inv = Inventory::default();
        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("anything", &mut inv, &mut rng);
        assert!(item.is_none());
    }

    #[test]
    fn test_craft_cost_serialization() {
        use crate::test_support::TestValue;
        let cost = CraftCost {
            kind: ResourceKind::Essence,
            count: 3,
        };
        let json = serde_json::to_string(&cost).test_expect("serialize");
        let deserialized: CraftCost = serde_json::from_str(&json).test_expect("deserialize");
        assert_eq!(deserialized.kind, ResourceKind::Essence);
        assert_eq!(deserialized.count, 3);
    }

    #[test]
    fn test_recipe_serialization() {
        use crate::test_support::TestValue;
        let recipe = CraftRecipe {
            name: "Test".to_string(),
            output_item: ItemKind::Consumable { heal: 20 },
            costs: vec![],
        };
        let json = serde_json::to_string(&recipe).test_expect("serialize");
        let deserialized: CraftRecipe = serde_json::from_str(&json).test_expect("deserialize");
        assert_eq!(deserialized.name, "Test");
    }

    #[test]
    fn test_recipe_book_craft_zero_cost() {
        let recipe_book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Free Item".to_string(),
                output_item: ItemKind::Consumable { heal: 5 },
                costs: vec![],
            }],
        };
        let mut inv = Inventory::default();
        let mut rng = seeded_rng();
        let item = recipe_book.craft_seeded("Free Item", &mut inv, &mut rng);
        assert!(item.is_some());
    }

    // ===== Mutation-resistant tests for RecipeBook::craft_seeded =====

    #[test]
    fn craft_legacy_returns_some_on_success() {
        use crate::test_support::TestValue;
        // Catches: replace RecipeBook::craft -> Option<Item> with None
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Potion".to_string(),
                output_item: ItemKind::Consumable { heal: 10 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Fiber,
                    count: 2,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Fiber, 5);
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Potion", &mut inv, &mut rng);
        assert!(
            result.is_some(),
            "craft() must return Some on valid recipe with resources"
        );
        let item = result.test_unwrap();
        assert_eq!(item.name, "Potion");
    }

    #[test]
    fn craft_legacy_finds_correct_recipe_by_name() {
        use crate::test_support::TestValue;
        // Catches: replace == with != in RecipeBook::craft (line 27 name match)
        let book = RecipeBook {
            recipes: vec![
                CraftRecipe {
                    name: "Alpha".to_string(),
                    output_item: ItemKind::Consumable { heal: 1 },
                    costs: vec![],
                },
                CraftRecipe {
                    name: "Beta".to_string(),
                    output_item: ItemKind::Consumable { heal: 2 },
                    costs: vec![],
                },
            ],
        };
        let mut inv = Inventory::default();
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Beta", &mut inv, &mut rng);
        assert!(result.is_some());
        assert_eq!(
            result.test_unwrap().name,
            "Beta",
            "Must find recipe by exact name match"
        );
    }

    #[test]
    fn craft_legacy_checks_resource_kind_equality() {
        // Catches: replace == with != in cost kind check (line 33)
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Sword".to_string(),
                output_item: ItemKind::Weapon {
                    base_damage: 10,
                    dtype: DamageType::Physical,
                },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 3,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10); // Wrong resource
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Sword", &mut inv, &mut rng);
        assert!(
            result.is_none(),
            "Must not craft when resource kind doesn't match"
        );
    }

    #[test]
    fn craft_legacy_exact_resource_count_boundary() {
        // Catches: replace < with ==, >, <= in have < c.count (line 36)
        // With exact count (have == count), < is false, so craft should succeed
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Shield".to_string(),
                output_item: ItemKind::Armor { defense: 5 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 3,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 3); // Exactly enough
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Shield", &mut inv, &mut rng);
        assert!(
            result.is_some(),
            "Exact resource count must allow crafting (have==cost)"
        );
        // Resources should be consumed
        let remaining = inv
            .resources
            .iter()
            .find(|(k, _)| *k == ResourceKind::Ore)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(remaining, 0, "Resources must be fully consumed");
    }

    #[test]
    fn craft_legacy_one_short_fails() {
        // Catches: replace < with <= (have < count vs have <= count)
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Shield".to_string(),
                output_item: ItemKind::Armor { defense: 5 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 3,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 2); // One short
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Shield", &mut inv, &mut rng);
        assert!(result.is_none(), "Must fail when have < cost");
    }

    #[test]
    fn craft_legacy_remove_resource_failure_returns_none() {
        // Catches: delete ! in `if !inv.remove_resource(...)` (line 42)
        // If ! is deleted, the branch executes on successful removal → returns None early
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Gem".to_string(),
                output_item: ItemKind::Consumable { heal: 50 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Crystal,
                    count: 1,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Crystal, 5);
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Gem", &mut inv, &mut rng);
        assert!(
            result.is_some(),
            "Successful remove_resource must not return None"
        );
    }

    #[test]
    fn craft_seeded_exact_resource_boundary() {
        // Catches: replace < with == or <= in craft_seeded (line 81)
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Ring".to_string(),
                output_item: ItemKind::Consumable { heal: 1 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Essence,
                    count: 4,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Essence, 4); // Exact
        let mut rng = seeded_rng();
        let result = book.craft_seeded("Ring", &mut inv, &mut rng);
        assert!(result.is_some(), "Exact count must succeed in craft_seeded");

        // Also test one-short fails
        let mut inv2 = Inventory::default();
        inv2.add_resource(ResourceKind::Essence, 3);
        let mut rng2 = seeded_rng();
        let result2 = book.craft_seeded("Ring", &mut inv2, &mut rng2);
        assert!(result2.is_none(), "One short must fail in craft_seeded");
    }

    #[test]
    fn gameplay_rng_craft_id_is_seed_locked() {
        use crate::test_support::TestValue;
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Blade".into(),
                output_item: ItemKind::Consumable { heal: 1 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 1,
                }],
            }],
        };
        let craft = |tick: u64| {
            let mut inv = Inventory::default();
            inv.add_resource(ResourceKind::Ore, 5);
            book.craft_with_seed("Blade", &mut inv, 0xA11C, tick)
                .test_expect("craft")
                .id
        };
        let id = craft(3);
        assert_eq!(id, craft(3), "same seed and tick must repeat the item id");
        assert_ne!(id, craft(4), "the next tick is a different stream");
        // Cross-platform lock. StdRng(ChaCha12) of mix_sim_seed(0xA11C, 3, SIM_SYSTEM_CRAFT).
        assert_eq!(id, gameplay_rng_locked_craft_id());
    }

    fn gameplay_rng_locked_craft_id() -> u32 {
        2_289_146_845
    }
}
