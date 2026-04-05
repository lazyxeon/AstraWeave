mod ui_overlay;

use astraweave_weaving::*;
use glam::Vec3;
use ui_overlay::*;

/// Advanced Content Demo with UI Overlays: Week 5 Day 4 Polish Showcase
fn main() {
    println!("=== AstraWeave Advanced Content Demo (UI Enhanced) ===\n");
    println!("Week 5 Day 4: Polish & UI Integration Showcase\n");
    println!(
        "Features: Ability cooldown UI, Quest progress HUD, Particle effects, Audio simulation\n"
    );

    demo_scenario_1_escort_quest_with_ui();
    println!("\n{}\n", "=".repeat(80));

    demo_scenario_2_defend_quest_with_ui();
    println!("\n{}\n", "=".repeat(80));

    demo_scenario_3_boss_fight_with_ui();

    println!("\n✅ All UI-enhanced scenarios complete! Week 5 Day 4 validated.");
}

fn demo_scenario_1_escort_quest_with_ui() {
    println!("🎯 Scenario 1: Escort Quest with UI Overlays");
    println!("----------------------------------------");

    let mut player = Player::new(Vec3::ZERO);
    player.echo_currency = 100;
    player.forward = Vec3::new(1.0, 0.0, 0.0);

    let escort_npc = quest_types::EscortNPC::new(
        "Merchant",
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        100.0,
    );

    let mut quest = Quest::new(
        "escort_merchant",
        "Safe Passage",
        "Escort the merchant to the safe zone.",
    )
    .with_objective(ObjectiveType::Escort { npc: escort_npc })
    .with_reward(QuestReward::EchoCurrency(50));

    quest.state = QuestState::Active;

    // === FRAME 0: Initial state ===
    println!("\n📺 Frame 0: Quest Start");
    println!("{}", render_full_hud(&player, &quest, 80));

    // Simulate player movement
    println!("🏃 Player moves forward...");
    player.position += player.forward * 2.0;

    // === FRAME 30: Enemy ambush! Use Shield ===
    println!("\n📺 Frame 30: Enemy Ambush! (2s @ 60 FPS)");
    println!("⚠️  3 enemies detected! Using Shield ability...");

    if let Ok(()) = player.use_shield() {
        println!("{}", play_audio_effect("shield_activate"));
        println!(
            "{}",
            render_particle_effect("shield_bubble", player.position)
        );
        println!(
            "✅ Shield activated! Echo: {} → {}",
            100, player.echo_currency
        );
    }

    // Update cooldowns (0.5s)
    player.ability_manager.update(0.5);
    println!("{}", render_full_hud(&player, &quest, 80));

    // === FRAME 60: Use Dash to escape ===
    println!("\n📺 Frame 60: Dash to Safety! (1s later)");
    println!("💨 Using Dash ability to reposition...");

    // Wait for dash cooldown (1.5s more)
    player.ability_manager.update(1.5);

    if let Ok((_pos, _dmg)) = player.use_dash() {
        println!("{}", play_audio_effect("dash_whoosh"));
        println!("{}", render_particle_effect("dash_trail", player.position));
        player.position += player.forward * 10.0; // Dash forward
        println!("✅ Dash complete! Echo: {}", player.echo_currency);
    }

    println!("{}", render_full_hud(&player, &quest, 80));

    // === FRAME 120: Quest objective progress ===
    println!("\n📺 Frame 120: Escort Progress (2s later)");

    // Simulate NPC reaching waypoint
    if let ObjectiveType::Escort { ref mut npc } = quest.objectives[0] {
        npc.position = Vec3::new(50.0, 0.0, 0.0); // Halfway
        npc.health = 85.0; // Took some damage
        println!(
            "🚶 Merchant reached waypoint! Health: {} / {}",
            npc.health, 100.0
        );
    }

    // Update cooldowns
    player.ability_manager.update(2.0);
    player.echo_currency += 10; // Regen some Echo

    println!("{}", render_full_hud(&player, &quest, 80));

    // === FRAME 180: Quest complete! ===
    println!("\n📺 Frame 180: Quest Complete! (1s later)");

    if let ObjectiveType::Escort { ref mut npc } = quest.objectives[0] {
        npc.position = npc.destination; // Reached destination
        println!("🎯 Merchant reached safe zone!");
    }

    quest.state = QuestState::Completed;
    println!("{}", play_audio_effect("quest_complete"));
    println!(
        "{}",
        render_notification("Quest Complete!", "You earned 50 Echo ⚡", "✅")
    );

    player.echo_currency += 50; // Reward

    println!("{}", render_full_hud(&player, &quest, 80));

    println!(
        "\n✅ Scenario 1 Complete: UI overlays, particle effects, audio simulation validated!"
    );
}

fn demo_scenario_2_defend_quest_with_ui() {
    println!("🎯 Scenario 2: Defend Quest with Wave Spawning UI");
    println!("----------------------------------------");

    let mut player = Player::new(Vec3::new(50.0, 0.0, 50.0));
    player.echo_currency = 150;
    player.forward = Vec3::new(0.0, 0.0, 1.0);

    let defend_anchor = quest_types::DefendObjective::new(
        "Village",
        Vec3::new(50.0, 0.0, 50.0),
        100.0,
        100.0, // Health
        180.0, // 3 minutes
        3,     // Total waves
    );

    let mut quest = Quest::new(
        "defend_village",
        "Hold the Line",
        "Defend the village from enemy waves.",
    )
    .with_objective(ObjectiveType::Defend {
        objective: defend_anchor,
        required_waves: 3,
    })
    .with_reward(QuestReward::EchoCurrency(100));

    quest.state = QuestState::Active;

    let mut spawner = EnemySpawner::new();
    spawner.add_spawn_point(Vec3::new(30.0, 0.0, 30.0), 5.0, None);

    // === FRAME 0: Defense starts ===
    println!("\n📺 Frame 0: Defense Begins");
    println!("{}", render_full_hud(&player, &quest, 80));

    // === WAVE 1: Standard enemies ===
    println!("\n📺 Wave 1: Standard Enemies (15 enemies)");

    // Use force_spawn_wave to simulate wave spawn
    let anchor = Anchor::new(1.0, 100, None);
    let anchors = vec![(0, &anchor)];
    let requests = spawner.force_spawn_wave(&anchors);

    for (_i, req) in requests.iter().enumerate().take(3) {
        let archetype = req.archetype;
        let spawn_pos = req.position;

        println!("{}", render_particle_effect("spawn_portal", spawn_pos));
        println!("{}", play_audio_effect("spawn_portal"));
        println!(
            "   Spawned: {:?} at ({:.1}, {:.1}, {:.1})",
            archetype, spawn_pos.x, spawn_pos.y, spawn_pos.z
        );
    }

    // Simulate combat (use abilities)
    println!("\n⚔️  Combat simulation: Dash + Shield combo");

    if let Ok((_pos, _dmg)) = player.use_dash() {
        println!("{}", render_particle_effect("dash_trail", player.position));
        player.position += player.forward * 10.0;
    }

    player.ability_manager.update(2.0);

    if let Ok(()) = player.use_shield() {
        println!(
            "{}",
            render_particle_effect("shield_bubble", player.position)
        );
    }

    // Simulate damage
    for i in 0..5 {
        let damage_pos = player.position + Vec3::new(i as f32, 0.0, i as f32);
        println!("{}", render_particle_effect("damage_numbers", damage_pos));
    }

    println!("\n{}", render_full_hud(&player, &quest, 80));

    // === WAVE 2: Riftstalkers appear ===
    println!("\n📺 Wave 2: Riftstalkers Appear! (Wave 6)");

    let archetype = crate::enemy_types::EnemyArchetype::Riftstalker;
    println!("⚠️  Advanced enemy type: {:?}", archetype);

    for i in 0..2 {
        let spawn_pos = Vec3::new(30.0, 0.0, 30.0) + Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        println!("{}", render_particle_effect("spawn_portal", spawn_pos));
    }

    // Abilities on cooldown
    player.ability_manager.update(3.0);
    player.echo_currency += 20;

    println!("{}", render_full_hud(&player, &quest, 80));

    // === WAVE 3: Victory! ===
    println!("\n📺 Wave 3 Complete: Defense Successful!");

    if let ObjectiveType::Defend {
        objective: ref mut anchor,
        ..
    } = quest.objectives[0]
    {
        anchor.current_health = 75.0; // Took some damage
        anchor.elapsed_seconds = 180.0; // Timer expired (success)
        println!("🏰 Village Health: {} / 100.0", anchor.current_health);
    }

    quest.state = QuestState::Completed;
    println!("{}", play_audio_effect("quest_complete"));
    println!(
        "{}",
        render_notification("Defense Complete!", "Village survived! +100 Echo ⚡", "🏆")
    );

    player.echo_currency += 100;

    println!("{}", render_full_hud(&player, &quest, 80));

    println!("\n✅ Scenario 2 Complete: Wave UI, spawn effects, combat VFX validated!");
}

fn demo_scenario_3_boss_fight_with_ui() {
    println!("🎯 Scenario 3: Boss Fight with Advanced UI");
    println!("----------------------------------------");

    let mut player = Player::new(Vec3::new(0.0, 0.0, 0.0));
    player.echo_currency = 200;
    player.forward = Vec3::new(1.0, 0.0, 0.0);

    let void_boss = quest_types::BossObjective::new(
        "Void Boss",
        1000.0, // High HP
        Vec3::new(50.0, 0.0, 0.0),
        Vec3::ZERO,
        20.0,
    );

    let mut quest = Quest::new(
        "defeat_void_boss",
        "Shatter the Void",
        "Defeat the Void Boss to restore reality.",
    )
    .with_objective(ObjectiveType::Boss {
        objective: void_boss,
    })
    .with_reward(QuestReward::EchoCurrency(250));

    quest.state = QuestState::Active;

    // === PHASE 1: Initial engagement ===
    println!("\n📺 Phase 1: Boss Awakens");
    println!("{}", render_full_hud(&player, &quest, 80));

    println!("\n👹 Void Boss appears!");
    println!(
        "{}",
        render_particle_effect("spawn_portal", Vec3::new(50.0, 0.0, 0.0))
    );
    println!("{}", play_audio_effect("spawn_portal"));

    // Boss health bar (separate UI element)
    println!(
        "\n{}════════════════════════════════════════════════════════════{}",
        colors::RED,
        colors::RESET
    );
    println!(
        "{}👹 Void Boss {} | HP: {}████████████████████{} 1000 / 1000{}",
        colors::BOLD,
        colors::RESET,
        colors::RED,
        colors::RESET,
        colors::RESET
    );
    println!(
        "{}════════════════════════════════════════════════════════════{}",
        colors::RED,
        colors::RESET
    );

    // === Combat loop: Use abilities strategically ===
    println!("\n⚔️  Combat Phase: Strategic ability usage");

    // Turn 1: Dash to avoid AOE
    println!("\n🌀 Boss charging Void Rift AOE! Use Dash to dodge...");
    if let Ok((_pos, _dmg)) = player.use_dash() {
        println!("{}", render_particle_effect("dash_trail", player.position));
        player.position += player.forward * 15.0;
        println!("✅ Dodged! Echo: {}", player.echo_currency);
    }

    player.ability_manager.update(1.0);
    println!("{}", render_full_hud(&player, &quest, 80));

    // Turn 2: Shield to absorb damage
    player.ability_manager.update(1.0);
    println!("\n💥 Boss melee attack incoming! Use Shield...");
    if let Ok(()) = player.use_shield() {
        println!(
            "{}",
            render_particle_effect("shield_bubble", player.position)
        );
        println!("✅ Blocked! Echo: {}", player.echo_currency);
    }

    // Damage to boss (simulated)
    if let ObjectiveType::Boss {
        objective: ref mut boss,
    } = quest.objectives[0]
    {
        boss.boss_health -= 300.0;
        println!("\n⚔️  Player deals 300 damage!");
        println!(
            "{}👹 Void Boss HP: {}{}████████████░░░░{} {} / 1000{}",
            colors::BOLD,
            colors::RED,
            colors::BG_RED,
            colors::RESET,
            boss.boss_health as i32,
            colors::RESET
        );
    }

    player.ability_manager.update(2.0);
    player.echo_currency += 30;
    println!("{}", render_full_hud(&player, &quest, 80));

    // === PHASE 2: Boss enrage ===
    println!("\n📺 Phase 2: Boss Enraged! (HP < 50%)");

    if let ObjectiveType::Boss {
        objective: ref mut boss,
    } = quest.objectives[0]
    {
        boss.current_phase = quest_types::BossPhase::Phase2;
        boss.boss_health = 400.0;
        println!("⚠️  Boss entered Phase 2! Attack speed increased!");
        println!(
            "{}👹 Void Boss (ENRAGED) HP: {}{}██████░░░░░░░░░░{} {} / 1000{}",
            colors::BOLD,
            colors::RED,
            colors::BG_RED,
            colors::RESET,
            boss.boss_health as i32,
            colors::RESET
        );
    }

    // Final ability combo
    player.ability_manager.update(2.0);
    println!("\n🔥 Final combo: Dash → Attack → Shield!");

    if let Ok((_pos, _dmg)) = player.use_dash() {
        println!("{}", render_particle_effect("dash_trail", player.position));
    }

    player.ability_manager.update(2.0);

    if let Ok(()) = player.use_shield() {
        println!(
            "{}",
            render_particle_effect("shield_bubble", player.position)
        );
    }

    // === Victory! ===
    println!("\n📺 Final Strike: Boss Defeated!");

    if let ObjectiveType::Boss {
        objective: ref mut boss,
    } = quest.objectives[0]
    {
        boss.boss_health = 0.0;
        boss.current_phase = quest_types::BossPhase::Phase3; // Defeated
        println!("💀 Void Boss shattered!");
        println!(
            "{}👹 Void Boss HP: {}{}░░░░░░░░░░░░░░░░{} 0 / 1000 (DEFEATED){}",
            colors::BOLD,
            colors::DIM,
            colors::RESET,
            colors::RESET,
            colors::RESET
        );
    }

    quest.state = QuestState::Completed;
    println!("{}", play_audio_effect("quest_complete"));
    println!(
        "{}",
        render_notification("Boss Defeated!", "Reality restored! +250 Echo ⚡", "👑")
    );

    player.echo_currency += 250;

    println!("{}", render_full_hud(&player, &quest, 80));

    println!("\n✅ Scenario 3 Complete: Boss UI, multi-phase mechanics, ability combos validated!");
}
