# Your First AI Companion

This tutorial will guide you through creating your first AI companion in AstraWeave. By the end, you'll have a companion that can perceive its environment, experience emotions, and exhibit autonomous behaviors.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Project Setup](#project-setup)
- [Creating a Basic Companion](#creating-a-basic-companion)
- [Adding Perception](#adding-perception)
- [Adding Emotions](#adding-emotions)
- [Adding Behaviors](#adding-behaviors)
- [Testing Your Companion](#testing-your-companion)
- [Next Steps](#next-steps)

## Prerequisites

Before starting, ensure you have:

- Rust 1.75.0 or later installed
- AstraWeave installed (see [Installation](installation.md))
- A compatible GPU (see [System Requirements](requirements.md))
- Basic Rust knowledge

```admonish tip
New to Rust? Check out [The Rust Book](https://doc.rust-lang.org/book/) for a comprehensive introduction.
```

## Project Setup

### Create a New Project

```bash
# Create a new Rust project
cargo new my_first_companion
cd my_first_companion
```

### Add AstraWeave Dependencies

Edit `Cargo.toml` to add AstraWeave dependencies:

```toml
[package]
name = "my_first_companion"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core AstraWeave crates
astraweave-ai = "0.1.0"
astraweave-render = "0.1.0"
bevy = "0.12"

# Utilities
anyhow = "1.0"
```

```admonish note
Version numbers may change. Check [crates.io](https://crates.io) for the latest versions.
```

### Verify Setup

```bash
# Build to ensure dependencies are resolved
cargo build
```

## Creating a Basic Companion

Let's start with a simple companion that just exists in the world.

### Basic Structure

Edit `src/main.rs`:

```rust
use bevy::prelude::*;
use astraweave_ai::prelude::*;
use anyhow::Result;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AstraWeaveAIPlugin)
        .add_systems(Startup, setup_companion)
        .add_systems(Update, update_companion)
        .run();
    
    Ok(())
}

fn setup_companion(mut commands: Commands) {
    // Create a basic companion
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .build();
    
    commands.spawn(companion);
    
    info!("Spawned companion: Buddy");
}

fn update_companion(
    mut companions: Query<&mut Companion>,
    time: Res<Time>,
) {
    for mut companion in companions.iter_mut() {
        companion.update(time.delta_seconds());
    }
}
```

### Run Your First Companion

```bash
cargo run
```

You should see log output indicating your companion was spawned!

```admonish success
Congratulations! You've created your first AI companion. Now let's make it more interesting.
```

## Adding Perception

Companions need to perceive their environment to interact meaningfully.

### Visual Perception

Add visual perception to detect nearby entities:

```rust
use astraweave_ai::perception::*;

fn setup_companion(mut commands: Commands) {
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .with_perception(PerceptionConfig {
            visual_range: 10.0,
            visual_fov: 120.0,      // 120-degree field of view
            update_rate: 10.0,       // 10 updates per second
            ..default()
        })
        .build();
    
    commands.spawn(companion);
}
```

### Adding Stimuli

Create entities that the companion can perceive:

```rust
fn setup_companion(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the companion
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .with_perception(PerceptionConfig {
            visual_range: 10.0,
            visual_fov: 120.0,
            update_rate: 10.0,
            ..default()
        })
        .build();
    
    commands.spawn((
        companion,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // Spawn a ball that the companion can see
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(0.5)),
            material: materials.add(Color::rgb(0.8, 0.2, 0.2)),
            transform: Transform::from_xyz(3.0, 0.5, 0.0),
            ..default()
        },
        Stimulus::new(StimulusType::Visual, 1.0),
    ));
    
    // Add camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 10.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    
    // Add light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}
```

### Reacting to Perception

Add a system to log what the companion perceives:

```rust
fn update_companion(
    mut companions: Query<(&mut Companion, &Transform)>,
    stimuli: Query<(&Stimulus, &Transform)>,
    time: Res<Time>,
) {
    for (mut companion, companion_transform) in companions.iter_mut() {
        companion.update(time.delta_seconds());
        
        // Check what the companion can see
        if let Some(perception) = companion.perception() {
            for (stimulus, stimulus_transform) in stimuli.iter() {
                let distance = companion_transform
                    .translation
                    .distance(stimulus_transform.translation);
                
                if distance <= perception.visual_range {
                    info!(
                        "{} sees {} stimulus at distance {:.2}",
                        companion.name(),
                        stimulus.stimulus_type,
                        distance
                    );
                }
            }
        }
    }
}
```

## Adding Emotions

Emotions make companions feel alive and responsive.

### Emotion System

Add emotion processing to your companion:

```rust
use astraweave_ai::emotion::*;

fn setup_companion(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .with_perception(PerceptionConfig {
            visual_range: 10.0,
            visual_fov: 120.0,
            update_rate: 10.0,
            ..default()
        })
        .with_emotions(vec![
            EmotionConfig::new("joy", 0.5, 0.95),
            EmotionConfig::new("curiosity", 0.3, 0.98),
            EmotionConfig::new("calm", 0.7, 0.99),
        ])
        .build();
    
    commands.spawn((
        companion,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    
    // ... spawn stimuli and camera as before
}
```

### Emotion Responses

Make emotions respond to stimuli:

```rust
fn process_emotions(
    mut companions: Query<&mut Companion>,
    stimuli: Query<(&Stimulus, &Transform)>,
) {
    for mut companion in companions.iter_mut() {
        if let Some(emotion_system) = companion.emotion_system_mut() {
            // Process nearby stimuli
            for (stimulus, _) in stimuli.iter() {
                match stimulus.stimulus_type {
                    StimulusType::Visual => {
                        emotion_system.increase_emotion("curiosity", 0.1);
                    }
                    StimulusType::Positive => {
                        emotion_system.increase_emotion("joy", 0.2);
                    }
                    StimulusType::Negative => {
                        emotion_system.decrease_emotion("calm", 0.15);
                    }
                    _ => {}
                }
            }
            
            // Log dominant emotion
            if let Some(dominant) = emotion_system.dominant_emotion() {
                info!(
                    "{} feels {} (intensity: {:.2})",
                    companion.name(),
                    dominant.name,
                    dominant.intensity
                );
            }
        }
    }
}

// Add this system to your app
fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AstraWeaveAIPlugin)
        .add_systems(Startup, setup_companion)
        .add_systems(Update, (update_companion, process_emotions))
        .run();
    
    Ok(())
}
```

## Adding Behaviors

Behaviors allow companions to act autonomously based on their internal state.

### Basic Movement Behavior

Add a simple behavior that makes the companion wander:

```rust
use astraweave_ai::behavior::*;

fn setup_companion(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .with_perception(PerceptionConfig::default())
        .with_emotions(vec![
            EmotionConfig::new("joy", 0.5, 0.95),
            EmotionConfig::new("curiosity", 0.3, 0.98),
            EmotionConfig::new("calm", 0.7, 0.99),
        ])
        .with_behavior(BehaviorConfig {
            wander_speed: 2.0,
            wander_radius: 5.0,
            ..default()
        })
        .build();
    
    commands.spawn((
        companion,
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
            material: materials.add(Color::rgb(0.3, 0.5, 0.8)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
    
    // ... rest of scene setup
}
```

### Emotion-Driven Behavior

Make behavior change based on emotions:

```rust
fn emotion_driven_behavior(
    mut companions: Query<(&mut Companion, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut companion, mut transform) in companions.iter_mut() {
        if let Some(emotion_system) = companion.emotion_system() {
            let curiosity = emotion_system
                .get_emotion("curiosity")
                .map(|e| e.intensity)
                .unwrap_or(0.0);
            
            let calm = emotion_system
                .get_emotion("calm")
                .map(|e| e.intensity)
                .unwrap_or(0.0);
            
            // High curiosity = move around more
            if curiosity > 0.6 {
                let speed = 2.0 * curiosity;
                let direction = Vec3::new(
                    (time.elapsed_seconds() * 0.5).sin(),
                    0.0,
                    (time.elapsed_seconds() * 0.5).cos(),
                );
                
                transform.translation += direction * speed * time.delta_seconds();
                
                info!("{} is exploring curiously!", companion.name());
            }
            // High calm = stay still
            else if calm > 0.7 {
                info!("{} is resting calmly.", companion.name());
            }
        }
    }
}

// Update main to include the behavior system
fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AstraWeaveAIPlugin)
        .add_systems(Startup, setup_companion)
        .add_systems(Update, (
            update_companion,
            process_emotions,
            emotion_driven_behavior,
        ))
        .run();
    
    Ok(())
}
```

## Testing Your Companion

### Complete Example

Here's the full code with all features:

```rust
use bevy::prelude::*;
use astraweave_ai::prelude::*;
use anyhow::Result;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My First Companion".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AstraWeaveAIPlugin)
        .add_systems(Startup, setup_companion)
        .add_systems(Update, (
            update_companion,
            process_emotions,
            emotion_driven_behavior,
        ))
        .run();
    
    Ok(())
}

fn setup_companion(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create companion
    let companion = CompanionBuilder::new()
        .with_name("Buddy")
        .with_perception(PerceptionConfig {
            visual_range: 10.0,
            visual_fov: 120.0,
            update_rate: 10.0,
            ..default()
        })
        .with_emotions(vec![
            EmotionConfig::new("joy", 0.5, 0.95),
            EmotionConfig::new("curiosity", 0.3, 0.98),
            EmotionConfig::new("calm", 0.7, 0.99),
        ])
        .with_behavior(BehaviorConfig::default())
        .build();
    
    commands.spawn((
        companion,
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
            material: materials.add(Color::rgb(0.3, 0.5, 0.8)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
    
    // Add ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::new(20.0, 20.0))),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });
    
    // Add interactive objects
    for i in 0..5 {
        let angle = (i as f32 / 5.0) * std::f32::consts::TAU;
        let pos = Vec3::new(angle.cos() * 4.0, 0.5, angle.sin() * 4.0);
        
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Sphere::new(0.5)),
                material: materials.add(Color::rgb(0.8, 0.2, 0.2)),
                transform: Transform::from_translation(pos),
                ..default()
            },
            Stimulus::new(StimulusType::Visual, 1.0),
        ));
    }
    
    // Add camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 8.0, 12.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    
    // Add light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

fn update_companion(
    mut companions: Query<&mut Companion>,
    time: Res<Time>,
) {
    for mut companion in companions.iter_mut() {
        companion.update(time.delta_seconds());
    }
}

fn process_emotions(
    mut companions: Query<(&mut Companion, &Transform)>,
    stimuli: Query<(&Stimulus, &Transform)>,
) {
    for (mut companion, companion_transform) in companions.iter_mut() {
        if let Some(emotion_system) = companion.emotion_system_mut() {
            for (stimulus, stimulus_transform) in stimuli.iter() {
                let distance = companion_transform
                    .translation
                    .distance(stimulus_transform.translation);
                
                if distance <= 5.0 {
                    emotion_system.increase_emotion("curiosity", 0.05);
                    emotion_system.increase_emotion("joy", 0.02);
                }
            }
        }
    }
}

fn emotion_driven_behavior(
    mut companions: Query<(&mut Companion, &mut Transform)>,
    time: Res<Time>,
) {
    for (companion, mut transform) in companions.iter_mut() {
        if let Some(emotion_system) = companion.emotion_system() {
            let curiosity = emotion_system
                .get_emotion("curiosity")
                .map(|e| e.intensity)
                .unwrap_or(0.0);
            
            if curiosity > 0.4 {
                let speed = 1.5 * curiosity;
                let direction = Vec3::new(
                    (time.elapsed_seconds() * 0.8).sin(),
                    0.0,
                    (time.elapsed_seconds() * 0.8).cos(),
                );
                
                transform.translation += direction * speed * time.delta_seconds();
                transform.translation.y = 0.5; // Keep on ground
            }
        }
    }
}
```

### Run and Observe

```bash
cargo run --release
```

Watch your companion:
- Wander around the scene
- React to nearby objects
- Show emotional responses
- Exhibit autonomous behavior

```admonish success
You've created a fully functional AI companion with perception, emotions, and behaviors!
```

## Next Steps

Now that you have a working companion, try:

1. **Add more emotions**: Experiment with fear, excitement, anger
2. **Complex behaviors**: Implement approach/avoid behaviors based on emotions
3. **Social interactions**: Add multiple companions that interact
4. **Save/Load**: Persist companion state between runs
5. **Visual feedback**: Change companion color based on dominant emotion

### Advanced Topics

- [Behavior Trees](../ai/behavior-trees.md)
- [Emotion Blending](../ai/emotions.md)
- [Multi-Agent Systems](../ai/multi-agent.md)
- [Performance Optimization](../optimization/ai-performance.md)

### Example Projects

Check out more examples in the repository:

```bash
# Clone AstraWeave
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave

# Run advanced examples
cargo run --release --example companion_emotions
cargo run --release --example multi_companion
cargo run --release --example behavior_showcase
```

### Join the Community

- **Discord**: [Join our server](https://discord.gg/astraweave)
- **GitHub**: [Open issues or discussions](https://github.com/verdentlabs/astraweave)
- **Forum**: [Community forums](https://community.verdentlabs.com)

```admonish tip
Share your companion creations in our Discord showcase channel!
```

Happy companion building!
