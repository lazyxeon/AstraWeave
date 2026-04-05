//! GPU Memory Budgeter
//!
//! Tracks GPU memory allocations by category and enforces configurable budgets
//! to prevent OOM conditions and enable intelligent streaming decisions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Memory allocation categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MemoryCategory {
    /// Vertex/index buffers
    Geometry,
    /// Texture data (color, normal, etc.)
    Textures,
    /// Render targets and framebuffers
    RenderTargets,
    /// Uniform/storage buffers
    Uniforms,
    /// Staging buffers (transient)
    Staging,
    /// Shadow maps
    Shadows,
    /// IBL/environment maps
    Environment,
    /// Other/uncategorized
    Other,
}

impl MemoryCategory {
    pub fn all() -> &'static [MemoryCategory] {
        &[
            MemoryCategory::Geometry,
            MemoryCategory::Textures,
            MemoryCategory::RenderTargets,
            MemoryCategory::Uniforms,
            MemoryCategory::Staging,
            MemoryCategory::Shadows,
            MemoryCategory::Environment,
            MemoryCategory::Other,
        ]
    }
}

/// Budget configuration for a memory category
#[derive(Debug, Clone)]
pub struct CategoryBudget {
    /// Soft limit - trigger warnings/streaming when exceeded
    pub soft_limit: u64,
    /// Hard limit - refuse allocations when exceeded
    pub hard_limit: u64,
    /// Current allocation
    pub current: u64,
}

impl Default for CategoryBudget {
    fn default() -> Self {
        Self {
            soft_limit: 256 * 1024 * 1024, // 256 MB soft
            hard_limit: 512 * 1024 * 1024, // 512 MB hard
            current: 0,
        }
    }
}

/// Callback for budget events
pub type BudgetCallback = Arc<dyn Fn(BudgetEvent) + Send + Sync>;

/// Budget event types
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BudgetEvent {
    /// Soft limit exceeded
    SoftLimitExceeded {
        category: MemoryCategory,
        current: u64,
        limit: u64,
    },
    /// Hard limit would be exceeded (allocation rejected)
    HardLimitBlocked {
        category: MemoryCategory,
        requested: u64,
        available: u64,
    },
    /// Memory pressure warning (total budget usage high)
    MemoryPressure {
        total_used: u64,
        total_budget: u64,
        percentage: f32,
    },
}

/// GPU Memory Budget Manager
pub struct GpuMemoryBudget {
    /// Per-category budgets
    budgets: RwLock<HashMap<MemoryCategory, CategoryBudget>>,

    /// Total memory used (atomic for fast queries)
    total_used: AtomicU64,

    /// Total budget across all categories
    total_budget: AtomicU64,

    /// Event callbacks
    callbacks: RwLock<Vec<BudgetCallback>>,

    /// Memory pressure threshold (0.0 - 1.0)
    pressure_threshold: f32,
}

impl Default for GpuMemoryBudget {
    fn default() -> Self {
        Self::new()
    }
}

// INVARIANT: All RwLock operations use `.expect("lock poisoned")` because lock
// poisoning indicates a prior panic — no safe recovery is possible at this layer.
#[allow(clippy::expect_used)]
impl GpuMemoryBudget {
    /// Create a new budget manager with default limits
    pub fn new() -> Self {
        let mut budgets = HashMap::new();
        for &cat in MemoryCategory::all() {
            budgets.insert(cat, CategoryBudget::default());
        }

        Self {
            budgets: RwLock::new(budgets),
            total_used: AtomicU64::new(0),
            total_budget: AtomicU64::new(2 * 1024 * 1024 * 1024), // 2 GB default total
            callbacks: RwLock::new(Vec::new()),
            pressure_threshold: 0.85,
        }
    }

    /// Create with custom total budget
    pub fn with_total_budget(total_bytes: u64) -> Self {
        let mgr = Self::new();
        mgr.total_budget.store(total_bytes, Ordering::SeqCst);

        // Distribute budget proportionally
        let per_category = total_bytes / 8;
        let mut budgets = mgr.budgets.write().unwrap_or_else(|e| e.into_inner());
        for budget in budgets.values_mut() {
            budget.soft_limit = (per_category as f64 * 0.75) as u64;
            budget.hard_limit = per_category;
        }

        // Give extra to textures (most memory-hungry)
        if let Some(tex_budget) = budgets.get_mut(&MemoryCategory::Textures) {
            tex_budget.soft_limit = (total_bytes as f64 * 0.3) as u64;
            tex_budget.hard_limit = (total_bytes as f64 * 0.4) as u64;
        }

        drop(budgets);
        mgr
    }

    /// Register a callback for budget events
    pub fn on_event(&self, callback: BudgetCallback) {
        self.callbacks
            .write()
            .expect("lock poisoned")
            .push(callback);
    }

    /// Attempt to allocate memory in a category
    /// Returns true if allocation succeeded, false if blocked
    pub fn try_allocate(&self, category: MemoryCategory, bytes: u64) -> bool {
        let mut budgets = self.budgets.write().unwrap_or_else(|e| e.into_inner());

        if let Some(budget) = budgets.get_mut(&category) {
            let new_total = budget.current + bytes;

            // Check hard limit
            if new_total > budget.hard_limit {
                self.fire_event(BudgetEvent::HardLimitBlocked {
                    category,
                    requested: bytes,
                    available: budget.hard_limit.saturating_sub(budget.current),
                });
                return false;
            }

            // Perform allocation
            budget.current = new_total;
            self.total_used.fetch_add(bytes, Ordering::SeqCst);

            // Check soft limit
            if new_total > budget.soft_limit {
                self.fire_event(BudgetEvent::SoftLimitExceeded {
                    category,
                    current: new_total,
                    limit: budget.soft_limit,
                });
            }

            // Check total pressure
            self.check_pressure();

            true
        } else {
            false
        }
    }

    /// Record a deallocation
    pub fn deallocate(&self, category: MemoryCategory, bytes: u64) {
        let mut budgets = self.budgets.write().unwrap_or_else(|e| e.into_inner());

        if let Some(budget) = budgets.get_mut(&category) {
            budget.current = budget.current.saturating_sub(bytes);
            self.total_used.fetch_sub(bytes, Ordering::SeqCst);
        }
    }

    /// Get current usage for a category
    pub fn get_usage(&self, category: MemoryCategory) -> u64 {
        self.budgets
            .read()
            .expect("lock poisoned")
            .get(&category)
            .map(|b| b.current)
            .unwrap_or(0)
    }

    /// Get total memory usage
    pub fn total_usage(&self) -> u64 {
        self.total_used.load(Ordering::SeqCst)
    }

    /// Get usage as percentage of total budget
    pub fn usage_percentage(&self) -> f32 {
        let used = self.total_used.load(Ordering::SeqCst) as f64;
        let total = self.total_budget.load(Ordering::SeqCst) as f64;
        if total > 0.0 {
            (used / total) as f32
        } else {
            0.0
        }
    }

    /// Get snapshot of all category usage
    pub fn snapshot(&self) -> Vec<(MemoryCategory, u64, u64)> {
        self.budgets
            .read()
            .expect("lock poisoned")
            .iter()
            .map(|(&cat, budget)| (cat, budget.current, budget.hard_limit))
            .collect()
    }

    /// Set budget for a specific category
    pub fn set_category_budget(&self, category: MemoryCategory, soft: u64, hard: u64) {
        let mut budgets = self.budgets.write().unwrap_or_else(|e| e.into_inner());
        if let Some(budget) = budgets.get_mut(&category) {
            budget.soft_limit = soft;
            budget.hard_limit = hard;
        }
    }

    fn check_pressure(&self) {
        let percentage = self.usage_percentage();
        if percentage > self.pressure_threshold {
            self.fire_event(BudgetEvent::MemoryPressure {
                total_used: self.total_used.load(Ordering::SeqCst),
                total_budget: self.total_budget.load(Ordering::SeqCst),
                percentage,
            });
        }
    }

    fn fire_event(&self, event: BudgetEvent) {
        let callbacks = self.callbacks.read().unwrap_or_else(|e| e.into_inner());
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_basic_allocation() {
        let budget = GpuMemoryBudget::new();

        assert!(budget.try_allocate(MemoryCategory::Textures, 1024));
        assert_eq!(budget.get_usage(MemoryCategory::Textures), 1024);
        assert_eq!(budget.total_usage(), 1024);

        budget.deallocate(MemoryCategory::Textures, 512);
        assert_eq!(budget.get_usage(MemoryCategory::Textures), 512);
        assert_eq!(budget.total_usage(), 512);
    }

    #[test]
    fn test_hard_limit_blocking() {
        let budget = GpuMemoryBudget::with_total_budget(1024 * 1024); // 1 MB total
        budget.set_category_budget(MemoryCategory::Geometry, 1024, 2048);

        // Should succeed
        assert!(budget.try_allocate(MemoryCategory::Geometry, 1024));

        // Should succeed (at limit)
        assert!(budget.try_allocate(MemoryCategory::Geometry, 1024));

        // Should fail (over limit)
        assert!(!budget.try_allocate(MemoryCategory::Geometry, 1));
    }

    #[test]
    fn test_soft_limit_callback() {
        let budget = GpuMemoryBudget::new();
        budget.set_category_budget(MemoryCategory::Textures, 512, 1024);

        let triggered = Arc::new(AtomicBool::new(false));
        let triggered_clone = triggered.clone();

        budget.on_event(Arc::new(move |event| {
            if matches!(event, BudgetEvent::SoftLimitExceeded { .. }) {
                triggered_clone.store(true, Ordering::SeqCst);
            }
        }));

        // Below soft limit
        assert!(budget.try_allocate(MemoryCategory::Textures, 256));
        assert!(!triggered.load(Ordering::SeqCst));

        // Above soft limit
        assert!(budget.try_allocate(MemoryCategory::Textures, 512));
        assert!(triggered.load(Ordering::SeqCst));
    }

    #[test]
    fn test_usage_percentage() {
        let budget = GpuMemoryBudget::new();
        // Default has 2GB total budget and 512MB per category (hard limit)
        // Allocate 400MB which is within Textures hard limit
        assert!(budget.try_allocate(MemoryCategory::Textures, 400 * 1024 * 1024));
        let pct = budget.usage_percentage();
        // 400MB / 2GB = 0.195... (~20%)
        assert!((pct - 0.195).abs() < 0.02, "Expected ~20%, got {}", pct);
    }

    #[test]
    fn test_snapshot() {
        let budget = GpuMemoryBudget::new();

        budget.try_allocate(MemoryCategory::Textures, 1000);
        budget.try_allocate(MemoryCategory::Geometry, 500);

        let snapshot = budget.snapshot();
        assert!(snapshot.len() >= 2);

        let tex_entry = snapshot
            .iter()
            .find(|(cat, _, _)| *cat == MemoryCategory::Textures);
        assert!(tex_entry.is_some());
        assert_eq!(tex_entry.unwrap().1, 1000);
    }
}
