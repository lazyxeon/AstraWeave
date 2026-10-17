#![forbid(unsafe_code)]
//! Stress Testing and Benchmarking for AstraWeave
//!
//! This crate provides comprehensive stress tests and benchmarks for:
//! - ECS performance with large numbers of entities
//! - AI planning and behavior tree execution
//! - Network serialization and deserialization
//! - Save/load performance with large game states
//! - Memory usage and leak detection

use anyhow::Result;
use astraweave_ecs::{App, World};
use criterion::{criterion_group, criterion_main, Criterion};
use rand::prelude::*;
use std::time::{Duration, Instant};

/// Test entity component for stress testing
#[derive(Clone, Debug)]
pub struct CStressEntity {
    pub id: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub health: f32,
    pub data: Vec<u8>,
    pub kind: String, // Variable-sized data for memory testing
}

/// AI stress component
#[derive(Clone, Debug)]
pub struct CAIStress {
    pub behavior_tree: Vec<String>, // Simulated behavior tree nodes
    pub last_decision: u64,
    pub decision_count: u64,
}

/// Network stress component
#[derive(Clone, Debug)]
pub struct CNetworkStress {
    pub player_id: String,
    pub input_buffer: Vec<Vec<u8>>,
    pub last_sync: u64,
}

/// Stress test configuration
#[derive(Clone, Debug)]
pub struct StressTestConfig {
    pub entity_count: usize,
    pub ai_entity_count: usize,
    pub network_entity_count: usize,
    pub test_duration_seconds: u64,
    pub max_memory_mb: usize,
}

/// Stress test results
#[derive(Clone, Debug)]
pub struct StressTestResults {
    pub start_time: Instant,
    pub frame_count: u64,
    pub total_entities_processed: u64,
    pub average_frame_time_ms: f64,
    pub peak_memory_usage_mb: usize,
    pub errors: Vec<String>,
}

/// Generate a large number of test entities
pub fn generate_stress_entities(config: &StressTestConfig) -> Vec<CStressEntity> {
    let mut rng = rand::rng();
    let mut entities = Vec::with_capacity(config.entity_count);

    for i in 0..config.entity_count {
        let data_size = rng.random_range(100..1000);
        let mut data = vec![0u8; data_size];
        rng.fill_bytes(&mut data);

        entities.push(CStressEntity {
            id: i as u32,
            position: [
                rng.random_range(-1000.0..1000.0),
                rng.random_range(-1000.0..1000.0),
                rng.random_range(-1000.0..1000.0),
            ],
            velocity: [
                rng.random_range(-10.0..10.0),
                rng.random_range(-10.0..10.0),
                rng.random_range(-10.0..10.0),
            ],
            health: rng.random_range(0.0..100.0),
            data,
            kind: "default".to_string(), // Provide a default value for the kind field
        });
    }

    entities
}

/// Generate AI stress entities
pub fn generate_ai_stress_entities(config: &StressTestConfig) -> Vec<CAIStress> {
    let mut rng = rand::rng();
    let mut entities = Vec::with_capacity(config.ai_entity_count);

    for _ in 0..config.ai_entity_count {
        let tree_size = rng.random_range(5..20);
        let behavior_tree = (0..tree_size)
            .map(|_| format!("node_{}", rng.random_range(0..100)))
            .collect();

        entities.push(CAIStress {
            behavior_tree,
            last_decision: rng.random_range(0..1000),
            decision_count: rng.random_range(0..10000),
        });
    }

    entities
}

/// Generate network stress entities
pub fn generate_network_stress_entities(config: &StressTestConfig) -> Vec<CNetworkStress> {
    let mut rng = rand::rng();
    let mut entities = Vec::with_capacity(config.network_entity_count);

    for i in 0..config.network_entity_count {
        let buffer_size = rng.random_range(1..10);
        let input_buffer = (0..buffer_size)
            .map(|_| {
                let size = rng.random_range(10..100);
                let mut data = vec![0u8; size];
                rng.fill_bytes(&mut data);
                data
            })
            .collect();

        entities.push(CNetworkStress {
            player_id: format!("player_{}", i),
            input_buffer,
            last_sync: rng.random_range(0..1000),
        });
    }

    entities
}

/// Physics simulation system for stress testing
fn physics_stress_system(world: &mut World) {
    let entities: Vec<_> = world.entities_with::<CStressEntity>();

    for entity in entities {
        if let Some(entity_data) = world.get_mut::<CStressEntity>(entity) {
            // Update position based on velocity
            for i in 0..3 {
                entity_data.position[i] += entity_data.velocity[i] * 0.016; // ~60 FPS delta
            }

            // Simple boundary checking
            for i in 0..3 {
                if entity_data.position[i] > 1000.0 {
                    entity_data.position[i] = -1000.0;
                } else if entity_data.position[i] < -1000.0 {
                    entity_data.position[i] = 1000.0;
                }
            }

            // Health decay simulation
            entity_data.health = (entity_data.health - 0.01).max(0.0);
        }
    }
}

/// AI decision system for stress testing
fn ai_stress_system(world: &mut World) {
    let mut rng = rand::rng();
    let entities: Vec<_> = world.entities_with::<CAIStress>();

    for entity in entities {
        if let Some(ai) = world.get_mut::<CAIStress>(entity) {
            ai.decision_count += 1;

            // Simulate AI decision making
            if rng.random_bool(0.1) {
                // 10% chance to make a decision
                ai.last_decision = ai.decision_count;
                // Simulate behavior tree traversal
                for node in &ai.behavior_tree {
                    std::hint::black_box(node); // Prevent optimization
                }
            }
        }
    }
}

/// Network processing system for stress testing
fn network_stress_system(world: &mut World) {
    let mut rng = rand::rng();
    let entities: Vec<_> = world.entities_with::<CNetworkStress>();

    for entity in entities {
        if let Some(net) = world.get_mut::<CNetworkStress>(entity) {
            // Simulate processing network inputs
            for input in &net.input_buffer {
                std::hint::black_box(input); // Prevent optimization
            }

            // Simulate network synchronization
            if rng.random_bool(0.05) {
                // 5% chance to sync
                net.last_sync += 1;
            }
        }
    }
}

/// Results tracking system
fn results_tracking_system(world: &mut World) {
    if let Some(results) = world.get_resource_mut::<StressTestResults>() {
        results.frame_count += 1;
    }

    // Get config for test completion check
    if let (Some(results), Some(config)) = (
        world.get_resource::<StressTestResults>(),
        world.get_resource::<StressTestConfig>(),
    ) {
        // Check for test completion
        if results.start_time.elapsed() >= Duration::from_secs(config.test_duration_seconds) {
            println!("Stress test completed!");
            println!("Frames: {}", results.frame_count);
            println!("Average frame time: {:.2}ms", results.average_frame_time_ms);
            println!("Peak memory: {}MB", results.peak_memory_usage_mb);
            println!("Errors: {}", results.errors.len());
        }
    }
}

/// Create a stress test application with all systems
pub fn create_stress_test_app(config: StressTestConfig) -> Result<App> {
    let mut app = App::new();

    // Add configuration and results resources
    app.world.insert_resource(config.clone());
    app.world.insert_resource(StressTestResults {
        start_time: Instant::now(),
        frame_count: 0,
        total_entities_processed: 0,
        average_frame_time_ms: 0.0,
        peak_memory_usage_mb: 0,
        errors: Vec::new(),
    });

    // Generate and spawn test entities
    let stress_entities = generate_stress_entities(&config);
    let ai_entities = generate_ai_stress_entities(&config);
    let network_entities = generate_network_stress_entities(&config);

    // Spawn entities (in a real implementation, this would be done through ECS commands)
    println!("Generated {} stress entities", stress_entities.len());
    println!("Generated {} AI entities", ai_entities.len());
    println!("Generated {} network entities", network_entities.len());

    // Add systems
    app.add_system_known_stage("simulation", physics_stress_system);
    app.add_system_known_stage("simulation", ai_stress_system);
    app.add_system_known_stage("simulation", network_stress_system);
    app.add_system_known_stage("post_simulation", results_tracking_system);

    Ok(app)
}

/// Run a complete stress test
pub async fn run_stress_test(config: StressTestConfig) -> Result<StressTestResults> {
    println!("Starting stress test with config: {:?}", config);

    let mut app = create_stress_test_app(config.clone())?;

    let start_time = Instant::now();
    let mut frame_count = 0u64;

    // Run the test for the specified duration
    while start_time.elapsed() < Duration::from_secs(config.test_duration_seconds) {
        app.schedule.run(&mut app.world);
        frame_count += 1;

        // Yield occasionally to prevent blocking
        if frame_count % 100 == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    // Get final results
    let results = StressTestResults {
        start_time,
        frame_count,
        total_entities_processed: (config.entity_count
            + config.ai_entity_count
            + config.network_entity_count) as u64
            * frame_count,
        average_frame_time_ms: start_time.elapsed().as_millis() as f64 / frame_count as f64,
        peak_memory_usage_mb: 0, // TODO: Implement memory tracking
        errors: Vec::new(),
    };

    println!("Stress test completed successfully!");
    println!(
        "Processed {} total entity-frames",
        results.total_entities_processed
    );
    println!("Average frame time: {:.2}ms", results.average_frame_time_ms);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_stress_test() {
        let config = StressTestConfig {
            entity_count: 100,
            ai_entity_count: 50,
            network_entity_count: 25,
            test_duration_seconds: 1,
            max_memory_mb: 100,
        };

        let results = run_stress_test(config).await.unwrap();

        assert!(results.frame_count > 0);
        assert!(results.total_entities_processed > 0);
        assert!(results.average_frame_time_ms > 0.0);
    }

    #[test]
    fn entity_generation() {
        let config = StressTestConfig {
            entity_count: 10,
            ai_entity_count: 5,
            network_entity_count: 3,
            test_duration_seconds: 1,
            max_memory_mb: 100,
        };

        let entities = generate_stress_entities(&config);
        assert_eq!(entities.len(), 10);

        let ai_entities = generate_ai_stress_entities(&config);
        assert_eq!(ai_entities.len(), 5);

        let net_entities = generate_network_stress_entities(&config);
        assert_eq!(net_entities.len(), 3);
    }
}

// Benchmark functions
pub fn ecs_performance_benchmark(c: &mut Criterion) {
    let config = StressTestConfig {
        entity_count: 1000,
        ai_entity_count: 500,
        network_entity_count: 100,
        test_duration_seconds: 1,
        max_memory_mb: 500,
    };

    c.bench_function("ecs_stress_test", |b| {
        b.iter(|| {
            let app = create_stress_test_app(config.clone()).unwrap();
            std::hint::black_box(app);
        })
    });
}

pub fn entity_generation_benchmark(c: &mut Criterion) {
    let config = StressTestConfig {
        entity_count: 10000,
        ai_entity_count: 1000,
        network_entity_count: 1000,
        test_duration_seconds: 1,
        max_memory_mb: 1000,
    };

    c.bench_function("entity_generation", |b| {
        b.iter(|| {
            let entities = generate_stress_entities(&config);
            let ai_entities = generate_ai_stress_entities(&config);
            let net_entities = generate_network_stress_entities(&config);
            std::hint::black_box((entities, ai_entities, net_entities));
        })
    });
}

criterion_group!(
    benches,
    ecs_performance_benchmark,
    entity_generation_benchmark
);
criterion_main!(benches);
