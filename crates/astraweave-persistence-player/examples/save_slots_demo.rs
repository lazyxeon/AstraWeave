//! Save slot manager demo
//!
//! Run with: cargo run -p astraweave-persistence-player --example save_slots_demo

use astraweave_persistence_player::{PlayerProfile, SaveSlotManager};
use std::path::PathBuf;

fn main() {
    println!("=== AstraWeave Save Slot Manager Demo ===\n");

    let manager = SaveSlotManager::new(10, PathBuf::from("saves/slots"));
    let mut profile = PlayerProfile::default();

    // Make some progress
    println!("🎮 Creating initial profile...");
    profile.name = "Hero".to_string();
    profile.unlock_ability("Dash");
    profile.grant_achievement("First Blood");
    profile.add_playtime(3600); // 1 hour
    profile.record_kill();
    profile.record_kill();

    // Save to slot 0
    println!("\n💾 Saving to slot 0 (Tutorial Level)...");
    manager
        .save_to_slot(
            0,
            vec![1, 2, 3, 4], // Dummy world state
            profile.clone(),
            "Tutorial Level".to_string(),
            Some("Checkpoint 1".to_string()),
        )
        .unwrap();

    // Continue playing, make more progress
    println!("\n🎮 Making more progress...");
    profile.unlock_ability("Double Jump");
    profile.grant_achievement("Level 2 Complete");
    profile.add_playtime(1800); // 30 more minutes
    profile.record_kill();
    profile.record_kill();
    profile.record_kill();

    // Save to slot 1
    println!("💾 Saving to slot 1 (Level 2)...");
    manager
        .save_to_slot(
            1,
            vec![5, 6, 7, 8],
            profile.clone(),
            "Level 2".to_string(),
            Some("Boss Fight".to_string()),
        )
        .unwrap();

    // Create a third save (different checkpoint)
    println!("\n🎮 Making even more progress...");
    profile.unlock_item("Health Potion");
    profile.add_playtime(900); // 15 more minutes

    println!("💾 Saving to slot 2 (Level 3)...");
    manager
        .save_to_slot(
            2,
            vec![9, 10, 11, 12],
            profile.clone(),
            "Level 3 - The Gauntlet".to_string(),
            None,
        )
        .unwrap();

    // List all saves
    println!("\n📂 Available save slots:");
    let slots = manager.list_slots().unwrap();
    for slot in &slots {
        let hours = slot.playtime_seconds / 3600;
        let minutes = (slot.playtime_seconds % 3600) / 60;
        let checkpoint_str = slot
            .checkpoint
            .as_ref()
            .map(|c| format!(" ({})", c))
            .unwrap_or_default();

        println!(
            "   Slot {}: {} - {}{}",
            slot.slot_id, slot.character_name, slot.level_name, checkpoint_str,
        );
        println!("      Playtime: {}h {}m", hours, minutes);
        println!(
            "      Saved: {}",
            astraweave_persistence_player::format_utc_seconds(slot.timestamp)
        );
    }

    // Load from slot 0 (earliest save)
    println!("\n📂 Loading slot 0 (earliest save)...");
    let loaded = manager.load_from_slot(0).unwrap();
    println!("   Level: {}", loaded.metadata.level_name);
    println!("   Checkpoint: {:?}", loaded.metadata.checkpoint);
    println!("   Character: {}", loaded.player_profile.name);
    println!(
        "   Playtime: {} hours",
        loaded.player_profile.stats.playtime_seconds / 3600
    );
    println!(
        "   Abilities: {:?}",
        loaded.player_profile.unlocks.abilities
    );
    println!("   Kills: {}", loaded.player_profile.stats.enemies_defeated);

    // Check next available slot
    println!("\n🔍 Checking slot availability...");
    match manager.next_available_slot() {
        Some(slot_id) => println!("   Next available slot: {}", slot_id),
        None => println!("   All slots full!"),
    }

    // Delete slot 1
    println!("\n🗑️  Deleting slot 1...");
    manager.delete_slot(1).unwrap();

    // List again
    println!("\n📂 Available save slots (after deletion):");
    let slots = manager.list_slots().unwrap();
    for slot in &slots {
        println!(
            "   Slot {}: {} - {}",
            slot.slot_id, slot.character_name, slot.level_name
        );
    }

    // Check next available slot again
    match manager.next_available_slot() {
        Some(slot_id) => println!("\n✅ Next available slot: {}", slot_id),
        None => println!("\n⚠️  All slots still full!"),
    }

    println!("\n✅ Save slot management demo complete!");
    println!("📁 Check saves/slots/ directory to see saved files");
}
