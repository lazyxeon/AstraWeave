/// Echo Transaction System
///
/// Centralized logging and statistics tracking for all Echo transactions.
/// Monitors gains/spends and provides analytics.
///
/// Integration:
/// - Input: Query<&EchoCurrency> (all entities with Echo currency)
/// - Output: TransactionEvents (for UI/statistics)
use crate::echo_currency::{EchoCurrency, Transaction, TransactionReason};

/// Transaction statistics
#[derive(Debug, Clone, Default)]
pub struct TransactionStats {
    pub total_gained: u32,
    pub total_spent: u32,
    pub net_balance: i32,
    pub kill_earnings: u32,
    pub repair_spending: u32,
    pub ability_spending: u32,
}

impl TransactionStats {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_currency(currency: &EchoCurrency) -> Self {
        let mut stats = Self::new();

        for txn in currency.transactions() {
            if txn.amount() > 0 {
                stats.total_gained += txn.amount() as u32;

                // Track kill earnings
                match txn.reason() {
                    TransactionReason::KillRiftStalker | TransactionReason::KillSentinel => {
                        stats.kill_earnings += txn.amount() as u32;
                    }
                    _ => {}
                }
            } else {
                stats.total_spent += (-txn.amount()) as u32;

                // Track repair spending
                if matches!(txn.reason(), TransactionReason::RepairAnchor(_)) {
                    stats.repair_spending += (-txn.amount()) as u32;
                }

                // Track ability spending
                match txn.reason() {
                    TransactionReason::UseEchoDash
                    | TransactionReason::UseEchoShield
                    | TransactionReason::DeployBarricade => {
                        stats.ability_spending += (-txn.amount()) as u32;
                    }
                    _ => {}
                }
            }
        }

        stats.net_balance = stats.total_gained as i32 - stats.total_spent as i32;
        stats
    }
}

/// Transaction event for UI feedback
#[derive(Debug, Clone)]
pub struct TransactionFeedbackEvent {
    pub amount: i32,
    pub reason: String,
    pub timestamp: f32,
}

impl TransactionFeedbackEvent {
    pub fn from_transaction(txn: &Transaction) -> Self {
        Self {
            amount: txn.amount(),
            reason: format!("{:?}", txn.reason()),
            timestamp: txn.timestamp(),
        }
    }
}

/// Echo transaction system (ECS system function)
///
/// Monitors Echo transactions and emits feedback events for UI.
pub fn echo_transaction_system(
    currency: &EchoCurrency,
    previous_balance: &mut u32,
) -> Option<TransactionFeedbackEvent> {
    let current_balance = currency.count();

    // Check if balance changed
    if current_balance == *previous_balance {
        return None;
    }

    // Get last transaction
    let last_txn = currency.last_transaction()?;

    // Update previous balance
    *previous_balance = current_balance;

    Some(TransactionFeedbackEvent::from_transaction(last_txn))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_from_empty_currency() {
        let currency = EchoCurrency::new();
        let stats = TransactionStats::from_currency(&currency);

        assert_eq!(stats.total_gained, 0);
        assert_eq!(stats.total_spent, 0);
        assert_eq!(stats.net_balance, 0);
    }

    #[test]
    fn test_stats_kill_earnings() {
        let mut currency = EchoCurrency::new();
        currency.add(1, TransactionReason::KillRiftStalker);
        currency.add(2, TransactionReason::KillSentinel);
        currency.add(1, TransactionReason::FoundShard);

        let stats = TransactionStats::from_currency(&currency);

        assert_eq!(stats.total_gained, 4);
        assert_eq!(stats.kill_earnings, 3); // 1 + 2 = 3
    }

    #[test]
    fn test_stats_repair_spending() {
        let mut currency = EchoCurrency::with_balance(10);
        currency.spend(5, TransactionReason::RepairAnchor("Z0".to_string()));
        currency.spend(2, TransactionReason::UseEchoDash);

        let stats = TransactionStats::from_currency(&currency);

        assert_eq!(stats.total_spent, 7);
        assert_eq!(stats.repair_spending, 5);
        assert_eq!(stats.ability_spending, 2);
    }

    #[test]
    fn test_stats_net_balance() {
        let mut currency = EchoCurrency::new();
        currency.add(10, TransactionReason::TutorialReward);
        currency.spend(3, TransactionReason::RepairAnchor("Z1".to_string()));
        currency.add(5, TransactionReason::KillSentinel);

        let stats = TransactionStats::from_currency(&currency);

        assert_eq!(stats.total_gained, 15);
        assert_eq!(stats.total_spent, 3);
        assert_eq!(stats.net_balance, 12);
    }

    #[test]
    fn test_transaction_feedback_event() {
        let mut currency = EchoCurrency::new();
        let mut previous_balance = 0;

        // No transaction yet
        let event = echo_transaction_system(&currency, &mut previous_balance);
        assert!(event.is_none());

        // Add Echoes
        currency.add(5, TransactionReason::TutorialReward);
        let event = echo_transaction_system(&currency, &mut previous_balance);

        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.amount, 5);
        assert!(event.reason.contains("TutorialReward"));
    }

    #[test]
    fn test_no_event_if_balance_unchanged() {
        let currency = EchoCurrency::with_balance(10);
        let mut previous_balance = 10;

        let event = echo_transaction_system(&currency, &mut previous_balance);
        assert!(event.is_none());
    }

    #[test]
    fn test_multiple_transactions() {
        let mut currency = EchoCurrency::new();
        let mut previous_balance = 0;

        // Transaction 1
        currency.add(3, TransactionReason::KillRiftStalker);
        let event1 = echo_transaction_system(&currency, &mut previous_balance);
        assert!(event1.is_some());
        assert_eq!(event1.unwrap().amount, 3);

        // Transaction 2
        currency.add(2, TransactionReason::FoundShard);
        let event2 = echo_transaction_system(&currency, &mut previous_balance);
        assert!(event2.is_some());
        assert_eq!(event2.unwrap().amount, 2);
    }
}
