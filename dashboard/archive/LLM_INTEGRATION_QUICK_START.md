# AstraWeave LLM Integration - Quick Start Guide

**For**: Developers implementing the LLM Integration Master Plan  
**Goal**: Get started with Phase 1 implementation in <30 minutes

---

## 🚀 Phase 1 Quick Start (Week 1)

### Step 1: Create New Crates (15 minutes)

```powershell
# Navigate to workspace root
cd c:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine

# Create embeddings crate
cargo new --lib astraweave-embeddings
cd astraweave-embeddings
```

**Add to `Cargo.toml`**:
```toml
[dependencies]
anyhow = { workspace = true }
nalgebra = "0.33"
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
criterion = "0.5"
```

**Repeat for**:
- `astraweave-context`
- `astraweave-prompts`
- `astraweave-rag`

### Step 2: Update Workspace `Cargo.toml`

Add to `[workspace.members]`:
```toml
"astraweave-embeddings",
"astraweave-context",
"astraweave-prompts",
"astraweave-rag",
```

### Step 3: Implement Embeddings Client (2 hours)

**File**: `astraweave-embeddings/src/lib.rs`

```rust
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

pub struct MockEmbeddingClient;

#[async_trait]
impl EmbeddingClient for MockEmbeddingClient {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Deterministic mock: hash text to 384-dim vector
        let mut vec = vec![0.0f32; 384];
        for (i, c) in text.chars().enumerate().take(384) {
            vec[i] = (c as u32 as f32) / 255.0;
        }
        Ok(vec)
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut result = Vec::with_capacity(texts.len());
        for text in texts {
            result.push(self.embed(text).await?);
        }
        Ok(result)
    }
}

pub struct VectorStore {
    embeddings: Vec<(String, Vec<f32>)>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self { embeddings: Vec::new() }
    }
    
    pub fn add(&mut self, text: String, embedding: Vec<f32>) {
        self.embeddings.push((text, embedding));
    }
    
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<_> = self.embeddings
            .iter()
            .map(|(text, emb)| {
                let similarity = cosine_similarity(query, emb);
                (text.clone(), similarity)
            })
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(k).collect()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (mag_a * mag_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding() {
        let client = MockEmbeddingClient;
        let embedding = client.embed("hello world").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    fn test_vector_search() {
        let mut store = VectorStore::new();
        let emb1 = vec![1.0, 0.0, 0.0];
        let emb2 = vec![0.9, 0.1, 0.0];
        let emb3 = vec![0.0, 1.0, 0.0];
        
        store.add("doc1".to_string(), emb1.clone());
        store.add("doc2".to_string(), emb2);
        store.add("doc3".to_string(), emb3);
        
        let results = store.search(&emb1, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1");
    }
}
```

### Step 4: Validate Implementation (5 minutes)

```powershell
# Check compilation
cargo check -p astraweave-embeddings

# Run tests
cargo test -p astraweave-embeddings

# Run benchmarks (optional)
cargo bench -p astraweave-embeddings
```

---

## 📋 Implementation Checklist

### Phase 1 (Weeks 1-4)

- [ ] **Week 1**: Embeddings layer
  - [ ] Create `astraweave-embeddings` crate
  - [ ] Implement `EmbeddingClient` trait
  - [ ] Implement `MockEmbeddingClient`
  - [ ] Implement `VectorStore` with cosine similarity
  - [ ] Write 10+ unit tests
  - [ ] Document API (rustdoc)

- [ ] **Week 2**: Context management
  - [ ] Create `astraweave-context` crate
  - [ ] Implement `ConversationHistory`
  - [ ] Token counting (simple char-based, then tiktoken)
  - [ ] Sliding window pruning
  - [ ] ECS component `CConversationHistory`
  - [ ] Write 10+ unit tests

- [ ] **Week 3**: Prompt templating
  - [ ] Create `astraweave-prompts` crate
  - [ ] Implement `PromptTemplate` with Handlebars
  - [ ] Variable substitution
  - [ ] Persona-specific generation
  - [ ] TOML-based library
  - [ ] Write 10+ unit tests

- [ ] **Week 4**: Integration + testing
  - [ ] Integrate with `astraweave-llm`
  - [ ] Integration tests (embeddings + context + prompts)
  - [ ] Performance benchmarks
  - [ ] Documentation (architecture guide)
  - [ ] Phase 1 completion summary

---

## 🎯 Daily Task Breakdown (Week 1)

### Day 1: Crate Setup
**Time**: 2 hours  
**Tasks**:
1. Create 4 new crates (embeddings, context, prompts, rag)
2. Update workspace `Cargo.toml`
3. Add dependencies
4. Create basic `lib.rs` with module structure
5. Validate compilation (`cargo check`)

### Day 2-3: Embeddings Implementation
**Time**: 8 hours  
**Tasks**:
1. Implement `EmbeddingClient` trait
2. Implement `MockEmbeddingClient` (deterministic)
3. Implement `VectorStore` (in-memory)
4. Implement cosine similarity
5. Write 5 unit tests
6. Write benchmarks

### Day 4-5: Advanced Embeddings
**Time**: 8 hours  
**Tasks**:
1. Add HNSW indexing (optional)
2. Add batch processing
3. Add persistence (serialize/deserialize)
4. Write 5 more unit tests
5. Documentation (rustdoc + architecture)

### Day 6-7: Testing + Polish
**Time**: 8 hours  
**Tasks**:
1. Integration tests with `astraweave-llm`
2. Performance benchmarks (embed 1000 texts)
3. Search accuracy tests (precision@10)
4. Code review + refactoring
5. Week 1 progress report

---

## 🧪 Testing Strategy

### Unit Tests (Per Crate)

**Embeddings**:
```rust
#[tokio::test]
async fn test_embedding_dimensions() { }

#[tokio::test]
async fn test_batch_embedding() { }

#[test]
fn test_cosine_similarity() { }

#[test]
fn test_vector_search_top_k() { }

#[test]
fn test_vector_search_empty() { }
```

**Context**:
```rust
#[test]
fn test_conversation_history_add() { }

#[test]
fn test_token_counting() { }

#[test]
fn test_sliding_window_pruning() { }

#[test]
fn test_context_budget_overflow() { }
```

**Prompts**:
```rust
#[test]
fn test_template_rendering() { }

#[test]
fn test_variable_substitution() { }

#[test]
fn test_persona_prompt_generation() { }

#[test]
fn test_toml_loading() { }
```

### Integration Tests

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_embed_search_flow() {
    let client = MockEmbeddingClient;
    let mut store = VectorStore::new();
    
    // Embed and store
    let texts = vec!["hello".to_string(), "world".to_string()];
    let embeddings = client.embed_batch(&texts).await.unwrap();
    for (text, emb) in texts.into_iter().zip(embeddings) {
        store.add(text, emb);
    }
    
    // Search
    let query = client.embed("hello").await.unwrap();
    let results = store.search(&query, 1);
    assert_eq!(results[0].0, "hello");
}
```

---

## 📊 Performance Targets (Phase 1)

### Embeddings
- **Latency**: <10ms per embedding (mock), <50ms (real model)
- **Throughput**: 1000 embeddings/sec (batched)
- **Memory**: <100MB for 10k embeddings (384-dim)

### Vector Search
- **Latency**: <10ms for 10k vectors (p95)
- **Accuracy**: 95%+ precision@10
- **Memory**: <50MB for 10k vectors

### Context Management
- **Latency**: <1ms for token counting
- **Latency**: <5ms for pruning
- **Memory**: <10MB per conversation (4096 tokens)

---

## 🐛 Troubleshooting

### Common Issues

**Issue**: Embeddings dimension mismatch
**Solution**: Ensure all embeddings are 384-dim (or configurable)

**Issue**: Vector search returns no results
**Solution**: Check cosine similarity calculation, ensure normalized vectors

**Issue**: Token counting inaccurate
**Solution**: Use tiktoken library for accurate counting

**Issue**: Context window overflow
**Solution**: Implement sliding window or summarization

---

## 📚 Resources

### Documentation
- [Master Plan](LLM_INTEGRATION_MASTER_PLAN.md) - Full 16-week plan
- [Architecture Guide](docs/llm_architecture.md) - System design (TODO)
- [API Docs](target/doc/astraweave_embeddings/index.html) - rustdoc

### External References
- [Sentence Transformers](https://www.sbert.net/) - Embedding models
- [HNSW](https://github.com/rust-cv/hnsw) - Approximate nearest neighbor
- [tiktoken](https://github.com/openai/tiktoken) - Token counting

---

## 🤝 Getting Help

**Questions?**
- Check [Master Plan](LLM_INTEGRATION_MASTER_PLAN.md) Section X
- Review existing LLM code in `astraweave-llm`
- Ask in project Discord/Slack

**Blockers?**
- File issue in GitHub
- Tag `@ai-integration-team`
- Review risk mitigation in master plan

---

## ✅ Success Criteria (Week 1)

**By End of Week 1**:
- ✅ 3 new crates created (embeddings, context, prompts)
- ✅ `MockEmbeddingClient` working (deterministic)
- ✅ `VectorStore` with search (<10ms for 1k vectors)
- ✅ 15+ unit tests passing
- ✅ Documentation complete (rustdoc)
- ✅ Compilation clean (0 warnings)

**Deliverable**: Foundation crates ready for Phase 2 integration

---

**Ready to start?** Run these commands:

```powershell
# Navigate to workspace
cd c:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine

# Create first crate
cargo new --lib astraweave-embeddings

# Start coding!
code astraweave-embeddings/src/lib.rs
```

**Good luck!** 🚀
