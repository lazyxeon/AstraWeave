//! Echo Currency Component - Resource system for Veilweaver
//!
//! Echoes are the primary currency for repairing anchors, unlocking abilities,
//! and deploying tactical structures. Players earn Echoes through combat and exploration.
//!
//! # Echo Sources (9-10 total available)
//! - Z0 tutorial reward: +2-3 Echoes
//! - Z1 Rift Stalkers: +1 Echo each (×4 enemies)
//! - Z1 Sentinel: +2 Echoes
//! - Z1 hidden shard: +1 Echo
//!
//! # Echo Costs
//! - Z0 loomspire anchor: 5 Echoes (tutorial trap, too expensive)
//! - Z2 vista anchor: 2 Echoes (unlocks Echo Dash)
//! - Z1 cover anchors: 1 Echo each (×2 anchors)
//! - Echo Dash ability: 1 Echo per use
//!
//! # Example
//! ```rust
//! use astraweave_weaving::echo_currency::{EchoCurrency, TransactionReason};
//!
//! let mut currency = EchoCurrency::new();
//!
//! // Earn Echoes from combat
//! currency.add(4, TransactionReason::KillRiftStalker);
//! currency.add(2, TransactionReason::KillSentinel);
//! assert_eq!(currency.count(), 6);
//!
//! // Spend Echoes on anchor repair
//! let success = currency.spend(2, TransactionReason::RepairAnchor("vista_tutorial_anchor".into()));
//! assert!(success);
//! assert_eq!(currency.count(), 4);
//!
//! // Check transaction history
//! assert_eq!(currency.transaction_count(), 3);
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Echo currency component - tracks player's Echo balance and transaction history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoCurrency {
    /// Current Echo count
    count: u32,

    /// Transaction history (for UI feedback and debugging)
    transaction_log: Vec<Transaction>,

    /// Maximum transaction log size (prevent unbounded growth)
    max_log_size: usize,
}

impl EchoCurrency {
    /// Default maximum transaction log size
    pub const DEFAULT_MAX_LOG_SIZE: usize = 100;

    /// Create a new Echo currency with zero balance
    pub fn new() -> Self {
        Self {
            count: 0,
            transaction_log: Vec::new(),
            max_log_size: Self::DEFAULT_MAX_LOG_SIZE,
        }
    }

    /// Create a new Echo currency with starting balance (for testing)
    pub fn with_balance(count: u32) -> Self {
        let mut currency = Self::new();
        currency.count = count;
        currency
    }

    /// Get current Echo count
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Check if player has at least the specified amount
    pub fn has(&self, amount: u32) -> bool {
        self.count >= amount
    }

    /// Add Echoes (earn from combat, exploration, rewards)
    ///
    /// # Arguments
    /// * `amount` - Number of Echoes to add
    /// * `reason` - Why Echoes were gained (for UI feedback)
    ///
    /// # Example
    /// ```rust
    /// use astraweave_weaving::echo_currency::{EchoCurrency, TransactionReason};
    ///
    /// let mut currency = EchoCurrency::new();
    /// currency.add(2, TransactionReason::TutorialReward);
    /// currency.add(1, TransactionReason::KillRiftStalker);
    /// assert_eq!(currency.count(), 3);
    /// ```
    pub fn add(&mut self, amount: u32, reason: TransactionReason) {
        self.count += amount;
        self.log_transaction(amount as i32, reason);
    }

    /// Spend Echoes (repair anchors, use abilities, deploy structures)
    ///
    /// Returns `true` if transaction succeeded, `false` if insufficient balance
    ///
    /// # Arguments
    /// * `amount` - Number of Echoes to spend
    /// * `reason` - Why Echoes were spent (for UI feedback)
    ///
    /// # Example
    /// ```rust
    /// use astraweave_weaving::echo_currency::{EchoCurrency, TransactionReason};
    ///
    /// let mut currency = EchoCurrency::with_balance(5);
    /// let success = currency.spend(2, TransactionReason::RepairAnchor("vista".into()));
    /// assert!(success);
    /// assert_eq!(currency.count(), 3);
    ///
    /// let fail = currency.spend(10, TransactionReason::RepairAnchor("expensive".into()));
    /// assert!(!fail);
    /// assert_eq!(currency.count(), 3); // Balance unchanged
    /// ```
    pub fn spend(&mut self, amount: u32, reason: TransactionReason) -> bool {
        if !self.has(amount) {
            return false;
        }

        self.count -= amount;
        self.log_transaction(-(amount as i32), reason);
        true
    }

    /// Get transaction history (for UI display)
    pub fn transactions(&self) -> &[Transaction] {
        &self.transaction_log
    }

    /// Get number of transactions in history
    pub fn transaction_count(&self) -> usize {
        self.transaction_log.len()
    }

    /// Get most recent transaction (for UI feedback)
    pub fn last_transaction(&self) -> Option<&Transaction> {
        self.transaction_log.last()
    }

    /// Clear transaction history (useful for testing or save/load)
    pub fn clear_history(&mut self) {
        self.transaction_log.clear();
    }

    /// Log a transaction to history
    fn log_transaction(&mut self, amount: i32, reason: TransactionReason) {
        let transaction = Transaction::new(amount, reason);
        self.transaction_log.push(transaction);

        // Trim log if it exceeds max size (FIFO)
        if self.transaction_log.len() > self.max_log_size {
            self.transaction_log.remove(0);
        }
    }
}

impl Default for EchoCurrency {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction record - single Echo gain/spend event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Amount gained (+) or spent (-)
    amount: i32,

    /// Why transaction occurred
    reason: TransactionReason,

    /// Timestamp (game time in seconds)
    timestamp: f32,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(amount: i32, reason: TransactionReason) -> Self {
        Self {
            amount,
            reason,
            timestamp: 0.0, // Will be set by transaction system
        }
    }

    /// Get transaction amount (+gain, -spend)
    pub fn amount(&self) -> i32 {
        self.amount
    }

    /// Get transaction reason
    pub fn reason(&self) -> &TransactionReason {
        &self.reason
    }

    /// Get transaction timestamp (game time)
    pub fn timestamp(&self) -> f32 {
        self.timestamp
    }

    /// Set transaction timestamp (called by transaction system)
    pub fn set_timestamp(&mut self, timestamp: f32) {
        self.timestamp = timestamp;
    }

    /// Check if this is a gain transaction
    pub fn is_gain(&self) -> bool {
        self.amount > 0
    }

    /// Check if this is a spend transaction
    pub fn is_spend(&self) -> bool {
        self.amount < 0
    }
}

/// Reason for Echo transaction (for UI feedback and analytics)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TransactionReason {
    /// Tutorial reward (Z0, +2-3 Echoes)
    TutorialReward,

    /// Killed Rift Stalker (Z1, +1 Echo)
    KillRiftStalker,

    /// Killed Sentinel (Z1, +2 Echoes)
    KillSentinel,

    /// Found hidden shard (Z1, +1 Echo)
    FoundShard,

    /// Repaired anchor (-1 to -5 Echoes)
    RepairAnchor(String), // Anchor ID

    /// Used Echo Dash ability (-1 Echo per use)
    UseEchoDash,

    /// Used Echo Shield ability
    UseEchoShield,

    /// Deployed barricade (-1 Echo)
    DeployBarricade,

    /// Quest reward (+variable Echoes)
    QuestReward(String), // Quest ID

    /// Debug/testing transaction
    Debug(String),
}

impl fmt::Display for TransactionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TutorialReward => write!(f, "Tutorial Reward"),
            Self::KillRiftStalker => write!(f, "Kill: Rift Stalker"),
            Self::KillSentinel => write!(f, "Kill: Sentinel"),
            Self::FoundShard => write!(f, "Found: Echo Shard"),
            Self::RepairAnchor(id) => write!(f, "Repair: {}", id),
            Self::UseEchoDash => write!(f, "Use: Echo Dash"),
            Self::UseEchoShield => write!(f, "Use: Echo Shield"),
            Self::DeployBarricade => write!(f, "Deploy: Barricade"),
            Self::QuestReward(id) => write!(f, "Quest: {}", id),
            Self::Debug(msg) => write!(f, "Debug: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_creation() {
        let currency = EchoCurrency::new();
        assert_eq!(currency.count(), 0);
        assert_eq!(currency.transaction_count(), 0);
    }

    #[test]
    fn test_currency_with_balance() {
        let currency = EchoCurrency::with_balance(10);
        assert_eq!(currency.count(), 10);
    }

    #[test]
    fn test_add_echoes() {
        let mut currency = EchoCurrency::new();

        currency.add(2, TransactionReason::TutorialReward);
        assert_eq!(currency.count(), 2);
        assert_eq!(currency.transaction_count(), 1);

        currency.add(1, TransactionReason::KillRiftStalker);
        assert_eq!(currency.count(), 3);
        assert_eq!(currency.transaction_count(), 2);
    }

    #[test]
    fn test_spend_echoes_success() {
        let mut currency = EchoCurrency::with_balance(5);

        let success = currency.spend(2, TransactionReason::RepairAnchor("vista".into()));
        assert!(success);
        assert_eq!(currency.count(), 3);
        assert_eq!(currency.transaction_count(), 1);
    }

    #[test]
    fn test_spend_echoes_insufficient() {
        let mut currency = EchoCurrency::with_balance(1);

        let fail = currency.spend(5, TransactionReason::RepairAnchor("expensive".into()));
        assert!(!fail);
        assert_eq!(currency.count(), 1); // Balance unchanged
        assert_eq!(currency.transaction_count(), 0); // No transaction logged
    }

    #[test]
    fn test_has_balance() {
        let currency = EchoCurrency::with_balance(5);

        assert!(currency.has(0));
        assert!(currency.has(3));
        assert!(currency.has(5));
        assert!(!currency.has(6));
    }

    #[test]
    fn test_transaction_history() {
        let mut currency = EchoCurrency::new();

        currency.add(2, TransactionReason::TutorialReward);
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.spend(1, TransactionReason::UseEchoDash);

        assert_eq!(currency.transaction_count(), 3);

        let transactions = currency.transactions();
        assert_eq!(transactions[0].amount(), 2);
        assert_eq!(transactions[1].amount(), 1);
        assert_eq!(transactions[2].amount(), -1);
    }

    #[test]
    fn test_last_transaction() {
        let mut currency = EchoCurrency::new();

        assert!(currency.last_transaction().is_none());

        currency.add(2, TransactionReason::TutorialReward);
        let last = currency.last_transaction().unwrap();
        assert_eq!(last.amount(), 2);

        currency.spend(1, TransactionReason::UseEchoDash);
        let last = currency.last_transaction().unwrap();
        assert_eq!(last.amount(), -1);
    }

    #[test]
    fn test_transaction_log_trimming() {
        let mut currency = EchoCurrency::new();
        currency.max_log_size = 5;

        // Add 10 transactions (should trim to 5)
        for i in 0..10 {
            currency.add(1, TransactionReason::Debug(format!("Transaction {}", i)));
        }

        assert_eq!(currency.transaction_count(), 5);
        assert_eq!(currency.count(), 10);
    }

    #[test]
    fn test_clear_history() {
        let mut currency = EchoCurrency::new();
        currency.add(5, TransactionReason::TutorialReward);
        currency.spend(2, TransactionReason::RepairAnchor("test".into()));

        assert_eq!(currency.transaction_count(), 2);

        currency.clear_history();
        assert_eq!(currency.transaction_count(), 0);
        assert_eq!(currency.count(), 3); // Balance unchanged
    }

    #[test]
    fn test_transaction_is_gain_spend() {
        let gain = Transaction::new(5, TransactionReason::TutorialReward);
        assert!(gain.is_gain());
        assert!(!gain.is_spend());

        let spend = Transaction::new(-2, TransactionReason::RepairAnchor("test".into()));
        assert!(!spend.is_gain());
        assert!(spend.is_spend());
    }

    #[test]
    fn test_echo_economy_scenario() {
        // Simulate Z0-Z2 vertical slice Echo economy
        let mut currency = EchoCurrency::new();

        // Z0 tutorial reward
        currency.add(3, TransactionReason::TutorialReward);
        assert_eq!(currency.count(), 3);

        // Z2 vista anchor repair (unlock Echo Dash)
        let success = currency.spend(
            2,
            TransactionReason::RepairAnchor("vista_tutorial_anchor".into()),
        );
        assert!(success);
        assert_eq!(currency.count(), 1);

        // Z1 combat: Kill 4 Rift Stalkers + 1 Sentinel
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.add(2, TransactionReason::KillSentinel);
        assert_eq!(currency.count(), 7);

        // Z1 hidden shard
        currency.add(1, TransactionReason::FoundShard);
        assert_eq!(currency.count(), 8);

        // Z1 optional: Deploy 2 barricades
        currency.spend(1, TransactionReason::DeployBarricade);
        currency.spend(1, TransactionReason::DeployBarricade);
        assert_eq!(currency.count(), 6);

        // Use Echo Dash twice
        currency.spend(1, TransactionReason::UseEchoDash);
        currency.spend(1, TransactionReason::UseEchoDash);
        assert_eq!(currency.count(), 4);

        // Final balance: 4 Echoes (reserve for mobility)
        assert!(currency.has(4));
    }
}
