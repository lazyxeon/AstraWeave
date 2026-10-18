// Integration tests for GOAP learning system
// Phase 3: Learning & Persistence

#[cfg(feature = "planner_advanced")]
mod learning_integration_tests {
    use astraweave_ai::goap::persistence::{HistoryPersistence, PersistenceFormat};
    use astraweave_ai::goap::{config, learning::*, ActionHistory};
    use tempfile::TempDir;

    #[test]
    fn test_learning_improves_over_time() {
        let config = config::GOAPConfig::default();
        let mut manager = LearningManager::new(config);
        let mut history = ActionHistory::new();

        // Simulate 20 action executions with improving success rate
        for i in 0..20 {
            if i < 10 {
                // First 10: 50% success
                if i % 2 == 0 {
                    history.record_success("attack", 0.1);
                } else {
                    history.record_failure("attack");
                }
            } else {
                // Next 10: 80% success (improvement)
                if i % 5 != 0 {
                    history.record_success("attack", 0.1);
                } else {
                    history.record_failure("attack");
                }
            }
        }

        // Probability should reflect the improvement
        let prob = manager.get_probability("attack", &history);

        // With EWMA smoothing, recent 80% success should dominate
        assert!(
            prob > 0.6,
            "Probability should reflect recent improvement: got {}",
            prob
        );
        assert!(prob < 0.9, "But not be overconfident: got {}", prob);

        println!("Learned probability after 20 executions: {:.2}", prob);
    }

    #[test]
    fn test_persistence_across_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("history.json");

        // Session 1: Build up history
        let mut history = ActionHistory::new();
        for _ in 0..5 {
            history.record_success("attack", 0.1);
        }
        for _ in 0..2 {
            history.record_failure("attack");
        }

        // Save
        HistoryPersistence::save(&history, &file_path, PersistenceFormat::Json)
            .expect("Failed to save history");

        // Session 2: Load and verify
        let loaded_history = HistoryPersistence::load(&file_path, PersistenceFormat::Json)
            .expect("Failed to load history");

        let stats = loaded_history.get_action_stats("attack").unwrap();
        assert_eq!(stats.executions, 7);
        assert_eq!(stats.successes, 5);
        assert_eq!(stats.failures, 2);

        println!("History persisted successfully across sessions");
    }

    #[test]
    fn test_config_driven_smoothing_methods() {
        let mut history = ActionHistory::new();

        // Build history: 7 successes, 3 failures (70% raw rate)
        for _ in 0..7 {
            history.record_success("attack", 0.1);
        }
        for _ in 0..3 {
            history.record_failure("attack");
        }

        // Test EWMA method
        let mut ewma_config = config::GOAPConfig::default();
        ewma_config.learning.smoothing.method = config::SmoothingMethod::Ewma;
        ewma_config.learning.smoothing.ewma_alpha = 0.5; // Quick adaptation

        let mut ewma_manager = LearningManager::new(ewma_config);
        let ewma_prob = ewma_manager.get_probability("attack", &history);

        // Test Bayesian method
        let mut bayesian_config = config::GOAPConfig::default();
        bayesian_config.learning.smoothing.method = config::SmoothingMethod::Bayesian;
        bayesian_config.learning.smoothing.bayesian_prior_successes = 3;
        bayesian_config.learning.smoothing.bayesian_prior_failures = 1;

        let mut bayesian_manager = LearningManager::new(bayesian_config);
        let bayesian_prob = bayesian_manager.get_probability("attack", &history);

        // Both should be reasonable, but may differ
        assert!(ewma_prob > 0.5 && ewma_prob < 0.9);
        assert!(bayesian_prob > 0.5 && bayesian_prob < 0.9);

        println!(
            "EWMA probability: {:.2}, Bayesian probability: {:.2}",
            ewma_prob, bayesian_prob
        );
    }

    #[test]
    fn test_learning_convergence_scenario() {
        let config = config::GOAPConfig::default();
        let mut manager = LearningManager::new(config);
        let mut history = ActionHistory::new();

        let mut probabilities = Vec::new();

        // Simulate 30 executions, tracking probability evolution
        for i in 0..30 {
            // Record execution (90% success rate)
            if i % 10 != 0 {
                history.record_success("heal", 0.05);
            } else {
                history.record_failure("heal");
            }

            // Track probability
            let prob = manager.get_probability("heal", &history);
            probabilities.push(prob);
        }

        // Verify convergence: later probabilities should be stable and close to 90%
        let early_avg = probabilities[..10].iter().sum::<f32>() / 10.0;
        let late_avg = probabilities[20..].iter().sum::<f32>() / 10.0;

        println!(
            "Early avg probability: {:.3}, Late avg: {:.3}",
            early_avg, late_avg
        );

        // Late estimate should be more accurate (closer to 90%)
        assert!(
            (late_avg - 0.9).abs() < (early_avg - 0.9).abs(),
            "Learning should converge towards true rate"
        );

        // Late estimates should also be more stable (lower variance)
        let late_variance: f32 = probabilities[20..]
            .iter()
            .map(|p| (p - late_avg).powi(2))
            .sum::<f32>()
            / 10.0;

        assert!(
            late_variance < 0.01,
            "Late estimates should be stable: variance = {}",
            late_variance
        );
    }

    #[test]
    fn test_multi_session_learning() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("multi_session.json");

        // Session 1: Initial learning
        let mut history_s1 = ActionHistory::new();
        for _ in 0..10 {
            history_s1.record_success("attack", 0.1);
            history_s1.record_success("heal", 0.05);
        }
        for _ in 0..5 {
            history_s1.record_failure("attack");
        }

        HistoryPersistence::save(&history_s1, &file_path, PersistenceFormat::Json).unwrap();

        // Session 2: Load and continue learning
        let mut history_s2 = HistoryPersistence::load(&file_path, PersistenceFormat::Json).unwrap();

        for _ in 0..5 {
            history_s2.record_success("attack", 0.1);
        }

        // Verify cumulative stats
        let attack_stats = history_s2.get_action_stats("attack").unwrap();
        assert_eq!(attack_stats.executions, 20); // 15 from s1 + 5 from s2
        assert_eq!(attack_stats.successes, 15); // 10 from s1 + 5 from s2
        assert_eq!(attack_stats.failures, 5); // All from s1

        let heal_stats = history_s2.get_action_stats("heal").unwrap();
        assert_eq!(heal_stats.executions, 10); // All from s1

        println!("Multi-session learning verified successfully");
    }

    #[test]
    fn test_config_bounds_enforcement() {
        let mut config = config::GOAPConfig::default();
        config.learning.min_success_rate = 0.3;
        config.learning.max_success_rate = 0.8;

        let mut manager = LearningManager::new(config.clone());

        // Create history with very low success rate
        let mut low_history = ActionHistory::new();
        for _ in 0..10 {
            low_history.record_failure("risky_action");
        }
        low_history.record_success("risky_action", 0.1);

        let low_prob = manager.get_probability("risky_action", &low_history);
        assert!(
            low_prob >= config.learning.min_success_rate,
            "Should respect min bound: got {}",
            low_prob
        );

        // Create history with very high success rate
        let mut high_history = ActionHistory::new();
        for _ in 0..20 {
            high_history.record_success("safe_action", 0.05);
        }

        let high_prob = manager.get_probability("safe_action", &high_history);
        assert!(
            high_prob <= config.learning.max_success_rate,
            "Should respect max bound: got {}",
            high_prob
        );

        println!(
            "Config bounds enforced: low={:.2}, high={:.2}",
            low_prob, high_prob
        );
    }

    #[test]
    fn test_pruning_and_noise_reduction() {
        let mut history = ActionHistory::new();

        // Add lots of data for some actions
        for _ in 0..20 {
            history.record_success("common_action", 0.1);
        }

        // Add minimal data for noisy actions
        history.record_success("rare_action_1", 0.1);
        history.record_failure("rare_action_2");

        assert_eq!(history.len(), 3);

        // Prune actions with < 3 executions
        history.prune_noise(3);

        // Only common_action should remain
        assert_eq!(history.len(), 1);
        assert!(history.get_action_stats("common_action").is_some());
        assert!(history.get_action_stats("rare_action_1").is_none());

        println!(
            "Noise reduction successful: {} actions remain",
            history.len()
        );
    }

    #[test]
    fn test_bincode_vs_json_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("history.json");
        let bin_path = temp_dir.path().join("history.bin");

        let mut history = ActionHistory::new();
        for _ in 0..50 {
            history.record_success("action", 0.1);
        }
        for _ in 0..10 {
            history.record_failure("action");
        }

        // Save in both formats
        HistoryPersistence::save(&history, &json_path, PersistenceFormat::Json).unwrap();
        HistoryPersistence::save(&history, &bin_path, PersistenceFormat::Bincode).unwrap();

        // Load from both
        let json_loaded = HistoryPersistence::load(&json_path, PersistenceFormat::Json).unwrap();
        let bin_loaded = HistoryPersistence::load(&bin_path, PersistenceFormat::Bincode).unwrap();

        // Verify both have same data
        let json_stats = json_loaded.get_action_stats("action").unwrap();
        let bin_stats = bin_loaded.get_action_stats("action").unwrap();

        assert_eq!(json_stats.executions, bin_stats.executions);
        assert_eq!(json_stats.successes, bin_stats.successes);
        assert_eq!(json_stats.failures, bin_stats.failures);

        // Check file sizes
        let json_size = std::fs::metadata(&json_path).unwrap().len();
        let bin_size = std::fs::metadata(&bin_path).unwrap().len();

        println!(
            "JSON size: {} bytes, Bincode size: {} bytes",
            json_size, bin_size
        );
        assert!(bin_size < json_size, "Bincode should be more compact");
    }

    #[test]
    fn test_learning_with_config_reload() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Save default config
        let config1 = config::GOAPConfig::default();
        config1.save(&config_path).unwrap();

        // Create manager with loaded config
        let loaded_config = config::GOAPConfig::load(&config_path).unwrap();
        let mut manager = LearningManager::new(loaded_config);

        let mut history = ActionHistory::new();
        for _ in 0..10 {
            history.record_success("test", 0.1);
        }

        let prob1 = manager.get_probability("test", &history);

        // Modify config (change smoothing method)
        let mut config2 = config::GOAPConfig::default();
        config2.learning.smoothing.method = config::SmoothingMethod::Bayesian;
        config2.save(&config_path).unwrap();

        // Reload config
        let reloaded_config = config::GOAPConfig::load(&config_path).unwrap();
        manager.update_config(reloaded_config);

        let prob2 = manager.get_probability("test", &history);

        // Probabilities may differ due to different smoothing methods
        println!(
            "Probability with EWMA: {:.2}, with Bayesian: {:.2}",
            prob1, prob2
        );

        // Both should be valid probabilities
        assert!(prob1 > 0.0 && prob1 < 1.0);
        assert!(prob2 > 0.0 && prob2 < 1.0);
    }

    #[test]
    fn test_complete_learning_cycle() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let history_path = temp_dir.path().join("history.json");

        // Step 1: Setup
        let config = config::GOAPConfig::default();
        config.save(&config_path).unwrap();

        let mut manager = LearningManager::new(config.clone());
        let mut history = ActionHistory::new();

        // Step 2: Simulate gameplay and learning
        for round in 0..5 {
            for i in 0..10 {
                // Success rate improves each round (deterministic for testing)
                let success_threshold = 5 - round; // Round 0: 5/10, Round 4: 1/10
                if i >= success_threshold {
                    history.record_success("adaptive_action", 0.1);
                } else {
                    history.record_failure("adaptive_action");
                }
            }

            // Check learned probability
            let prob = manager.get_probability("adaptive_action", &history);
            println!("Round {}: Learned probability = {:.2}", round + 1, prob);
        }

        // Step 3: Save history
        HistoryPersistence::save(&history, &history_path, PersistenceFormat::Json).unwrap();

        // Step 4: Simulate new session
        let reloaded_config = config::GOAPConfig::load(&config_path).unwrap();
        let mut new_manager = LearningManager::new(reloaded_config);
        let reloaded_history =
            HistoryPersistence::load(&history_path, PersistenceFormat::Json).unwrap();

        // Verify learning persisted
        let final_prob = new_manager.get_probability("adaptive_action", &reloaded_history);
        assert!(
            final_prob > 0.6,
            "Should have learned high success rate: got {}",
            final_prob
        );

        println!(
            "Complete learning cycle verified: final probability = {:.2}",
            final_prob
        );
    }
}
