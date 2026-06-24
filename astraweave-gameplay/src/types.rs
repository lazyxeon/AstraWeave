use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DamageType {
    Physical,
    Echo,
    Fire,
    Frost,
    Shock,
    Poison,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ResourceKind {
    Wood,
    Crystal,
    Ore,
    Fiber,
    Essence,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DropTableEntry {
    pub kind: ResourceKind,
    pub min: u32,
    pub max: u32,
    pub weight: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaveConsequence {
    pub drop_multiplier: f32,          // affects harvesting in region
    pub faction_disposition: i32,      // -100..100
    pub weather_shift: Option<String>, // e.g., "windy", "calm", "storm"
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WeaveOpKind {
    ReinforcePath,
    CollapseBridge,
    RedirectWind,
    LowerWater,
    RaisePlatform,
    /// W.2c.3 — freeze the water surface at `a`. **Presentation-only** today: a
    /// valid weave that consumes the weather budget and carries `a` for the render
    /// layer's freeze deformation. The truth-bearing freeze (walkable ice / buoyancy
    /// block) is deferred to the post-surface-arc gameplay-truth-coupling phase.
    FreezeWater,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaveOp {
    pub kind: WeaveOpKind,
    pub a: Vec3,
    pub b: Option<Vec3>,
    pub budget_cost: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaveBudget {
    pub terrain_edits: i32,
    pub weather_ops: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub kind: ResourceKind,
    pub count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // DamageType tests
    #[test]
    fn test_damage_type_variants() {
        let types = [
            DamageType::Physical,
            DamageType::Echo,
            DamageType::Fire,
            DamageType::Frost,
            DamageType::Shock,
            DamageType::Poison,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_damage_type_clone() {
        let dt = DamageType::Fire;
        let cloned = dt;
        assert!(matches!(cloned, DamageType::Fire));
    }

    #[test]
    fn test_damage_type_debug() {
        let dt = DamageType::Shock;
        let debug_str = format!("{:?}", dt);
        assert!(debug_str.contains("Shock"));
    }

    #[test]
    fn test_damage_type_serialization() {
        let dt = DamageType::Frost;
        let json = serde_json::to_string(&dt).unwrap();
        let deserialized: DamageType = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, DamageType::Frost));
    }

    // ResourceKind tests
    #[test]
    fn test_resource_kind_variants() {
        let kinds = [
            ResourceKind::Wood,
            ResourceKind::Crystal,
            ResourceKind::Ore,
            ResourceKind::Fiber,
            ResourceKind::Essence,
        ];
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn test_resource_kind_equality() {
        let r1 = ResourceKind::Crystal;
        let r2 = ResourceKind::Crystal;
        let r3 = ResourceKind::Ore;
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_resource_kind_serialization() {
        let kind = ResourceKind::Essence;
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: ResourceKind = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ResourceKind::Essence);
    }

    // DropTableEntry tests
    #[test]
    fn test_drop_table_entry_creation() {
        let entry = DropTableEntry {
            kind: ResourceKind::Ore,
            min: 1,
            max: 5,
            weight: 0.5,
        };
        assert_eq!(entry.kind, ResourceKind::Ore);
        assert_eq!(entry.min, 1);
        assert_eq!(entry.max, 5);
        assert!((entry.weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_drop_table_entry_serialization() {
        let entry = DropTableEntry {
            kind: ResourceKind::Wood,
            min: 2,
            max: 10,
            weight: 1.0,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: DropTableEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, ResourceKind::Wood);
        assert_eq!(deserialized.min, 2);
        assert_eq!(deserialized.max, 10);
    }

    #[test]
    fn test_drop_table_entry_clone() {
        let entry = DropTableEntry {
            kind: ResourceKind::Crystal,
            min: 0,
            max: 3,
            weight: 0.25,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.kind, entry.kind);
        assert_eq!(cloned.min, entry.min);
    }

    // WeaveConsequence tests
    #[test]
    fn test_weave_consequence_creation() {
        let c = WeaveConsequence {
            drop_multiplier: 1.5,
            faction_disposition: 25,
            weather_shift: Some("storm".to_string()),
        };
        assert!((c.drop_multiplier - 1.5).abs() < f32::EPSILON);
        assert_eq!(c.faction_disposition, 25);
        assert_eq!(c.weather_shift, Some("storm".to_string()));
    }

    #[test]
    fn test_weave_consequence_no_weather() {
        let c = WeaveConsequence {
            drop_multiplier: 0.5,
            faction_disposition: -50,
            weather_shift: None,
        };
        assert!(c.weather_shift.is_none());
    }

    #[test]
    fn test_weave_consequence_serialization() {
        let c = WeaveConsequence {
            drop_multiplier: 2.0,
            faction_disposition: 100,
            weather_shift: Some("calm".to_string()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: WeaveConsequence = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.faction_disposition, 100);
    }

    // WeaveOpKind tests
    #[test]
    fn test_weave_op_kind_variants() {
        let kinds = [
            WeaveOpKind::ReinforcePath,
            WeaveOpKind::CollapseBridge,
            WeaveOpKind::RedirectWind,
            WeaveOpKind::LowerWater,
            WeaveOpKind::RaisePlatform,
        ];
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn test_weave_op_kind_debug() {
        let kind = WeaveOpKind::CollapseBridge;
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("CollapseBridge"));
    }

    // WeaveOp tests
    #[test]
    fn test_weave_op_creation() {
        let op = WeaveOp {
            kind: WeaveOpKind::RaisePlatform,
            a: Vec3::new(1.0, 2.0, 3.0),
            b: Some(Vec3::new(4.0, 5.0, 6.0)),
            budget_cost: 10,
        };
        assert!(matches!(op.kind, WeaveOpKind::RaisePlatform));
        assert_eq!(op.a, Vec3::new(1.0, 2.0, 3.0));
        assert!(op.b.is_some());
        assert_eq!(op.budget_cost, 10);
    }

    #[test]
    fn test_weave_op_single_point() {
        let op = WeaveOp {
            kind: WeaveOpKind::LowerWater,
            a: Vec3::ZERO,
            b: None,
            budget_cost: 5,
        };
        assert!(op.b.is_none());
    }

    #[test]
    fn test_weave_op_serialization() {
        let op = WeaveOp {
            kind: WeaveOpKind::RedirectWind,
            a: Vec3::new(0.0, 0.0, 0.0),
            b: Some(Vec3::new(10.0, 0.0, 10.0)),
            budget_cost: 15,
        };
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: WeaveOp = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.kind, WeaveOpKind::RedirectWind));
        assert_eq!(deserialized.budget_cost, 15);
    }

    // WeaveBudget tests
    #[test]
    fn test_weave_budget_creation() {
        let budget = WeaveBudget {
            terrain_edits: 10,
            weather_ops: 5,
        };
        assert_eq!(budget.terrain_edits, 10);
        assert_eq!(budget.weather_ops, 5);
    }

    #[test]
    fn test_weave_budget_serialization() {
        let budget = WeaveBudget {
            terrain_edits: 100,
            weather_ops: 50,
        };
        let json = serde_json::to_string(&budget).unwrap();
        let deserialized: WeaveBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.terrain_edits, 100);
        assert_eq!(deserialized.weather_ops, 50);
    }

    #[test]
    fn test_weave_budget_zero() {
        let budget = WeaveBudget {
            terrain_edits: 0,
            weather_ops: 0,
        };
        assert_eq!(budget.terrain_edits, 0);
        assert_eq!(budget.weather_ops, 0);
    }

    // InventoryEntry tests
    #[test]
    fn test_inventory_entry_creation() {
        let entry = InventoryEntry {
            kind: ResourceKind::Fiber,
            count: 25,
        };
        assert_eq!(entry.kind, ResourceKind::Fiber);
        assert_eq!(entry.count, 25);
    }

    #[test]
    fn test_inventory_entry_serialization() {
        let entry = InventoryEntry {
            kind: ResourceKind::Crystal,
            count: 100,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: InventoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, ResourceKind::Crystal);
        assert_eq!(deserialized.count, 100);
    }

    #[test]
    fn test_inventory_entry_zero_count() {
        let entry = InventoryEntry {
            kind: ResourceKind::Ore,
            count: 0,
        };
        assert_eq!(entry.count, 0);
    }

    #[test]
    fn test_inventory_entry_clone() {
        let entry = InventoryEntry {
            kind: ResourceKind::Essence,
            count: 42,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.kind, entry.kind);
        assert_eq!(cloned.count, entry.count);
    }
}
