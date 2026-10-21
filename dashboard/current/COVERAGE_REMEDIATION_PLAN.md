# AstraWeave Coverage Remediation Plan

**Version**: 1.0.0  
**Date**: December 6, 2025  
**Based On**: COVERAGE_GAP_ANALYSIS.md v1.0.0

---

## Overview

This plan provides specific, actionable test additions for each crate below target coverage. Plans are organized by priority tier with effort estimates.

---

## Priority Matrix

| Priority | Criteria | Crates | Effort |
|----------|----------|--------|--------|
| **P0 - Critical** | 0% coverage OR security-critical | 8 crates | 60h |
| **P1 - High** | <30% coverage AND core functionality | 10 crates | 80h |
| **P2 - Medium** | 30-50% coverage OR gameplay-critical | 8 crates | 50h |
| **P3 - Low** | 50-70% AND close to threshold | 4 crates | 20h |

**Total Estimated Effort**: 210 hours (~5 weeks FTE)

---

## P0 - Critical Priority (60h)

### 1. astraweave-dialogue (0% → 80%)
**Current**: 0% (15 lines)  
**Target**: 80%  
**Effort**: 2 hours

```rust
// Tests to add in astraweave-dialogue/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_node_creation() {
        let node = DialogueNode::new("Hello!", vec!["Option A", "Option B"]);
        assert_eq!(node.text(), "Hello!");
        assert_eq!(node.options().len(), 2);
    }

    #[test]
    fn test_dialogue_tree_navigation() {
        let tree = DialogueTree::default();
        let node = tree.current_node();
        assert!(node.is_some());
    }

    #[test]
    fn test_dialogue_choice_selection() {
        let mut tree = DialogueTree::default();
        tree.select_option(0);
        // Verify state change
    }
}
```

### 2. astraweave-npc (0% → 70%)
**Current**: 0% (229 lines)  
**Target**: 70%  
**Effort**: 4 hours

**Tests to add**:
- `test_npc_creation_default`
- `test_npc_state_transitions`
- `test_npc_schedule_management`
- `test_npc_interaction_handling`
- `test_npc_behavior_assignment`
- `test_npc_position_update`
- `test_npc_dialogue_trigger`

### 3. astraweave-secrets (0% → 70%)
**Current**: 0% (51 lines)  
**Target**: 70%  
**Effort**: 3 hours

**Tests to add**:
- `test_secret_storage_and_retrieval`
- `test_secret_encryption_roundtrip`
- `test_secret_key_validation`
- `test_secret_expiration`
- `test_invalid_secret_handling`

### 4. astraweave-ipc (0% → 50%)
**Current**: 0% (41 lines)  
**Target**: 50%  
**Effort**: 6 hours

**Tests to add** (using process mocks):
- `test_ipc_channel_creation`
- `test_ipc_message_serialization`
- `test_ipc_send_receive_roundtrip`
- `test_ipc_timeout_handling`
- `test_ipc_disconnection`

### 5. astraweave-author (0% → 50%)
**Current**: 0% (72 lines) - **BLOCKED by rhai Sync trait**  
**Target**: 50%  
**Effort**: 8 hours (4h fix + 4h tests)

**Prerequisites**:
1. Resolve `rhai::Scope` Sync trait issue
2. May require wrapping in `Arc<Mutex<>>` or using `Send + Sync` alternatives

**Tests to add** (after fix):
- `test_script_compilation`
- `test_script_execution_basic`
- `test_script_variable_binding`
- `test_script_error_handling`

### 6. astraweave-fluids (0% → 40%)
**Current**: 0% (290 lines)  
**Target**: 40%  
**Effort**: 12 hours

**Infrastructure needed**:
- Mock GPU context
- Simulation state snapshots

**Tests to add**:
- `test_fluid_particle_creation`
- `test_fluid_grid_initialization`
- `test_fluid_velocity_field`
- `test_fluid_boundary_conditions`
- `test_fluid_timestep_calculation`

### 7. astraweave-net (24% → 60%)
**Current**: 24% (622/2620 lines)  
**Target**: 60%  
**Effort**: 14 hours

**Tests to add**:
- Packet serialization (10 tests)
- Connection state machine (8 tests)
- Rate limiting (5 tests)
- HMAC validation (5 tests)
- TLS handshake (mock) (5 tests)
- Timeout handling (5 tests)

### 8. astraweave-security (70% → 80%)
**Current**: 70% (2137/3222 lines)  
**Target**: 80%  
**Effort**: 8 hours

**Tests to add**:
- `test_rate_limiter_edge_cases`
- `test_hmac_tampering_detection`
- `test_tls_certificate_validation`
- `test_access_control_inheritance`
- `test_audit_log_rotation`

---

## P1 - High Priority (80h)

### 9. astraweave-persona (14% → 60%)
**Current**: 14% (1553/9862 lines)  
**Target**: 60%  
**Effort**: 16 hours

**Test categories**:
1. **Persona Creation** (4 tests)
   - Default persona
   - Custom traits
   - Serialization roundtrip
   - Validation

2. **Personality Traits** (8 tests)
   - Trait combination
   - Trait conflicts
   - Dynamic updates
   - Memory impact

3. **Conversation Style** (6 tests)
   - Formality levels
   - Emotional responses
   - Context adaptation
   - Memory injection

4. **Integration** (6 tests)
   - With LLM prompts
   - With memory system
   - With dialogue system

### 10. astraweave-rag (24% → 60%)
**Current**: 24% (1360/5577 lines)  
**Target**: 60%  
**Effort**: 14 hours

**Test categories**:
1. **Document Ingestion** (5 tests)
2. **Chunk Management** (5 tests)
3. **Vector Search** (using mock embeddings) (5 tests)
4. **Query Processing** (5 tests)
5. **Result Ranking** (5 tests)
6. **Cache Management** (5 tests)

### 11. astraweave-llm-eval (9% → 50%)
**Current**: 9% (292/2844 lines)  
**Target**: 50%  
**Effort**: 12 hours

**Test categories**:
1. **Metric Calculation** (8 tests)
2. **Benchmark Fixtures** (5 tests)
3. **Comparison Logic** (5 tests)
4. **Report Generation** (5 tests)
5. **Statistical Analysis** (5 tests)

### 12. astraweave-gameplay (36% → 60%)
**Current**: 36% (3150/8144 lines)  
**Target**: 60%  
**Effort**: 10 hours

**Test categories**:
1. **Combat Mechanics** (6 tests)
2. **Damage Calculation** (5 tests)
3. **Status Effects** (5 tests)
4. **Cooldown Management** (4 tests)
5. **Inventory Operations** (5 tests)

### 13. astraweave-scripting (31% → 60%)
**Current**: 31% (1207/4771 lines)  
**Target**: 60%  
**Effort**: 12 hours

**Test categories**:
1. **Script Loading** (5 tests)
2. **API Bindings** (8 tests)
3. **Sandbox Security** (6 tests)
4. **Event Hooks** (5 tests)
5. **Error Handling** (5 tests)

### 14. astraweave-persistence-ecs (27% → 60%)
**Current**: 27% (514/1941 lines)  
**Target**: 60%  
**Effort**: 10 hours

**Test categories**:
1. **Component Serialization** (8 tests)
2. **Entity Relationships** (5 tests)
3. **Version Migration** (5 tests)
4. **Corruption Recovery** (5 tests)
5. **Incremental Save** (5 tests)

### 15. astraweave-context (38% → 60%)
**Current**: 38% (2513/7466 lines)  
**Target**: 60%  
**Effort**: 10 hours

**Test categories**:
1. **Context Window Management** (5 tests)
2. **Token Budgeting** (5 tests)
3. **Priority Ordering** (5 tests)
4. **Truncation Strategies** (5 tests)
5. **Memory Integration** (5 tests)

### 16. astraweave-observability (22% → 60%)
**Current**: 22% (493/1889 lines)  
**Target**: 60%  
**Effort**: 8 hours

**Test categories**:
1. **Tracing Integration** (5 tests)
2. **Metrics Collection** (5 tests)
3. **Alert Triggering** (5 tests)
4. **Dashboard Data** (3 tests)
5. **Export Formats** (5 tests)

---

## P2 - Medium Priority (50h)

### 17. astraweave-scene (33% → 55%)
**Current**: 33% (861/2664 lines)  
**Target**: 55%  
**Effort**: 8 hours

### 18. astraweave-render (36% → 50%)
**Current**: 36% (8685/25342 lines)  
**Target**: 50%  
**Effort**: 16 hours (includes mock GPU setup)

### 19. astraweave-ui (20% → 45%)
**Current**: 20% (1919/8581 lines)  
**Target**: 45%  
**Effort**: 12 hours

### 20. astraweave-audio (22% → 40%)
**Current**: 22% (1530/7145 lines)  
**Target**: 40%  
**Effort**: 10 hours

### 21. astraweave-sdk (20% → 50%)
**Current**: 20% (415/2253 lines)  
**Target**: 50%  
**Effort**: 8 hours

### 22. astraweave-net-ecs (38% → 55%)
**Current**: 38% (550/1422 lines)  
**Target**: 55%  
**Effort**: 6 hours

### 23. astraweave-asset (49% → 65%)
**Current**: 49% (1311/2582 lines)  
**Target**: 65%  
**Effort**: 6 hours

### 24. astraweave-stress-test (16% → 40%)
**Current**: 16% (234/1418 lines)  
**Target**: 40%  
**Effort**: 6 hours

---

## P3 - Low Priority (20h)

### 25. astraweave-behavior (66% → 75%)
**Current**: 66% (1540/2443 lines)  
**Target**: 75%  
**Effort**: 5 hours

### 26. astraweave-llm (65% → 75%)
**Current**: 65% (6879/10466 lines)  
**Target**: 75%  
**Effort**: 6 hours

### 27. astraweave-assets (51% → 65%)
**Current**: 51% (1378/2372 lines)  
**Target**: 65%  
**Effort**: 5 hours

### 28. astraweave-terrain (50% → 65%)
**Current**: 50% (5531/11208 lines)  
**Target**: 65%  
**Effort**: 6 hours

---

## Test Infrastructure Requirements

### 1. Mock LLM Client
```rust
// Location: astraweave-llm/src/test_utils.rs
pub struct ConfigurableMockLlm {
    pub responses: HashMap<String, String>,
    pub latency: Duration,
    pub failure_rate: f32,
}

impl ChatClient for ConfigurableMockLlm {
    async fn complete(&self, prompt: &str) -> Result<String> {
        if rand::random::<f32>() < self.failure_rate {
            return Err(anyhow!("Simulated failure"));
        }
        tokio::time::sleep(self.latency).await;
        Ok(self.responses.get(prompt).cloned().unwrap_or_default())
    }
}
```

### 2. Mock GPU Context
```rust
// Location: astraweave-render/src/test_utils.rs
pub struct MockGpuContext {
    pub device: MockDevice,
    pub queue: MockQueue,
}

impl MockGpuContext {
    pub fn new() -> Self {
        // Create headless wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        // Use software adapter for tests
        // ...
    }
}
```

### 3. Test World Fixtures
```rust
// Location: astraweave-ecs/src/test_fixtures.rs
pub fn create_test_world_with_player() -> World {
    let mut world = World::new();
    let player = world.spawn();
    world.insert(player, Position { x: 0.0, y: 0.0, z: 0.0 });
    world.insert(player, Health { current: 100, max: 100 });
    world
}

pub fn create_test_world_with_combat() -> World {
    let mut world = create_test_world_with_player();
    let enemy = world.spawn();
    world.insert(enemy, Position { x: 10.0, y: 0.0, z: 0.0 });
    world.insert(enemy, Health { current: 50, max: 50 });
    world.insert(enemy, AIComponent::default());
    world
}
```

### 4. Network Mock Infrastructure
```rust
// Location: astraweave-net/src/test_utils.rs
pub struct MockSocket {
    pub sent_packets: Arc<Mutex<Vec<Packet>>>,
    pub receive_queue: Arc<Mutex<VecDeque<Packet>>>,
}

impl Socket for MockSocket {
    fn send(&self, packet: Packet) -> Result<()> {
        self.sent_packets.lock().unwrap().push(packet);
        Ok(())
    }
    
    fn recv(&self) -> Result<Option<Packet>> {
        Ok(self.receive_queue.lock().unwrap().pop_front())
    }
}
```

---

## Sprint Plan

### Sprint 1 (Week 1): Quick Wins
- [ ] dialogue (0% → 80%): 2h
- [ ] npc (0% → 70%): 4h
- [ ] secrets (0% → 70%): 3h
- [ ] ipc (0% → 50%): 6h

**Sprint Goal**: Eliminate 4 zero-coverage crates  
**Total Effort**: 15h

### Sprint 2 (Week 2): Core Fixes
- [ ] author (0% → 50%): 8h (rhai fix + tests)
- [ ] net security tests: 8h
- [ ] observability: 8h

**Sprint Goal**: Address security-critical gaps  
**Total Effort**: 24h

### Sprint 3 (Week 3): LLM Support
- [ ] persona (14% → 60%): 16h
- [ ] Mock LLM infrastructure: 8h

**Sprint Goal**: LLM persona coverage  
**Total Effort**: 24h

### Sprint 4 (Week 4): LLM Continuation
- [ ] rag (24% → 60%): 14h
- [ ] llm-eval (9% → 50%): 12h

**Sprint Goal**: RAG and evaluation coverage  
**Total Effort**: 26h

### Sprint 5 (Week 5): Gameplay
- [ ] gameplay (36% → 60%): 10h
- [ ] scripting (31% → 60%): 12h
- [ ] persistence-ecs (27% → 60%): 10h

**Sprint Goal**: Core gameplay coverage  
**Total Effort**: 32h

---

## Success Metrics

### Week 2 Checkpoint
- Zero-coverage crates: 6 → 2 (fluids, author if blocked)
- Critical crates (<30%): 16 → 12
- Overall coverage: 53% → 58%

### Week 4 Checkpoint
- Zero-coverage crates: 2 → 0
- Critical crates (<30%): 12 → 6
- Overall coverage: 58% → 65%

### Week 6 Checkpoint
- Crates ≥50%: 21 → 38
- Crates ≥70%: 17 → 25
- Overall coverage: 65% → 72%

---

## Appendix: Test Templates

### Unit Test Template
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_basic() {
        // Arrange
        let sut = SystemUnderTest::new();
        
        // Act
        let result = sut.do_something();
        
        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_feature_edge_case_empty() {
        let sut = SystemUnderTest::new();
        let result = sut.process(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_feature_error_handling() {
        let sut = SystemUnderTest::new();
        let result = sut.process_invalid();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected error"));
    }
}
```

### Async Test Template
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_async_operation() {
        let sut = AsyncSystem::new();
        
        let result = timeout(Duration::from_secs(5), sut.fetch())
            .await
            .expect("timeout")
            .expect("fetch failed");
        
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_async_timeout_handling() {
        let sut = AsyncSystem::with_delay(Duration::from_secs(10));
        
        let result = timeout(Duration::from_millis(100), sut.fetch()).await;
        
        assert!(result.is_err()); // Should timeout
    }
}
```

---

**Next Document**: COVERAGE_SPRINT_PLAN.md (detailed sprint breakdowns)
