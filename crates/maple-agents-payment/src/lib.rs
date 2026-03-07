//! MAPLE Agents Payment -- reference payment processing agent.
//!
//! Provides a payment agent with operations for processing payments,
//! checking balances, and transferring funds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("insufficient funds: available {available}, required {required}")]
    InsufficientFunds { available: f64, required: f64 },
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("invalid amount: {0}")]
    InvalidAmount(String),
    #[error("payment error: {0}")]
    Internal(String),
}

pub type PaymentResult<T> = Result<T, PaymentError>;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Status of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Reversed,
}

/// A payment request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub currency: String,
    pub memo: Option<String>,
}

impl PaymentRequest {
    pub fn new(from: impl Into<String>, to: impl Into<String>, amount: f64) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            amount,
            currency: "USD".to_string(),
            memo: None,
        }
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }
}

/// Result of a processed payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResultData {
    pub transaction_id: String,
    pub status: TransactionStatus,
    pub fee: f64,
    pub timestamp: DateTime<Utc>,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub currency: String,
}

/// Account balance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: String,
    pub available: f64,
    pub pending: f64,
    pub currency: String,
}

// ---------------------------------------------------------------------------
// Payment Agent
// ---------------------------------------------------------------------------

/// Reference payment processing agent.
pub struct PaymentAgent {
    balances: HashMap<String, f64>,
    transactions: Vec<PaymentResultData>,
    fee_rate: f64,
}

impl Default for PaymentAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentAgent {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            transactions: Vec::new(),
            fee_rate: 0.029, // 2.9%
        }
    }

    /// Set up an account with initial balance.
    pub fn create_account(&mut self, account_id: impl Into<String>, initial_balance: f64) {
        self.balances.insert(account_id.into(), initial_balance);
    }

    /// Process a payment.
    pub fn process_payment(&mut self, request: &PaymentRequest) -> PaymentResult<PaymentResultData> {
        if request.amount <= 0.0 {
            return Err(PaymentError::InvalidAmount("amount must be positive".into()));
        }

        let from_balance = self
            .balances
            .get(&request.from)
            .ok_or_else(|| PaymentError::AccountNotFound(request.from.clone()))?;

        let fee = request.amount * self.fee_rate;
        let total = request.amount + fee;

        if *from_balance < total {
            return Err(PaymentError::InsufficientFunds {
                available: *from_balance,
                required: total,
            });
        }

        // Ensure 'to' account exists
        if !self.balances.contains_key(&request.to) {
            return Err(PaymentError::AccountNotFound(request.to.clone()));
        }

        // Debit from, credit to
        *self.balances.get_mut(&request.from).unwrap() -= total;
        *self.balances.get_mut(&request.to).unwrap() += request.amount;

        let result = PaymentResultData {
            transaction_id: Uuid::new_v4().to_string(),
            status: TransactionStatus::Completed,
            fee,
            timestamp: Utc::now(),
            from: request.from.clone(),
            to: request.to.clone(),
            amount: request.amount,
            currency: request.currency.clone(),
        };
        self.transactions.push(result.clone());
        Ok(result)
    }

    /// Check the balance of an account.
    pub fn check_balance(&self, account_id: &str) -> PaymentResult<AccountBalance> {
        let balance = self
            .balances
            .get(account_id)
            .ok_or_else(|| PaymentError::AccountNotFound(account_id.to_string()))?;
        Ok(AccountBalance {
            account_id: account_id.to_string(),
            available: *balance,
            pending: 0.0,
            currency: "USD".to_string(),
        })
    }

    /// Transfer funds between accounts (alias for process_payment).
    pub fn transfer(&mut self, from: &str, to: &str, amount: f64) -> PaymentResult<PaymentResultData> {
        self.process_payment(&PaymentRequest::new(from, to, amount))
    }

    /// Get transaction history.
    pub fn transaction_history(&self) -> &[PaymentResultData] {
        &self.transactions
    }

    /// Get transactions for a specific account.
    pub fn account_transactions(&self, account_id: &str) -> Vec<&PaymentResultData> {
        self.transactions
            .iter()
            .filter(|t| t.from == account_id || t.to == account_id)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> PaymentAgent {
        let mut agent = PaymentAgent::new();
        agent.create_account("alice", 1000.0);
        agent.create_account("bob", 500.0);
        agent
    }

    #[test]
    fn test_process_payment() {
        let mut agent = make_agent();
        let req = PaymentRequest::new("alice", "bob", 100.0);
        let result = agent.process_payment(&req).unwrap();
        assert_eq!(result.status, TransactionStatus::Completed);
        assert!(result.fee > 0.0);
    }

    #[test]
    fn test_insufficient_funds() {
        let mut agent = make_agent();
        let req = PaymentRequest::new("bob", "alice", 10000.0);
        assert!(agent.process_payment(&req).is_err());
    }

    #[test]
    fn test_check_balance() {
        let agent = make_agent();
        let balance = agent.check_balance("alice").unwrap();
        assert!((balance.available - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_account_not_found() {
        let agent = make_agent();
        assert!(agent.check_balance("charlie").is_err());
    }

    #[test]
    fn test_transfer() {
        let mut agent = make_agent();
        let result = agent.transfer("alice", "bob", 50.0).unwrap();
        assert_eq!(result.amount, 50.0);
        let alice = agent.check_balance("alice").unwrap();
        assert!(alice.available < 1000.0);
    }

    #[test]
    fn test_invalid_amount() {
        let mut agent = make_agent();
        let req = PaymentRequest::new("alice", "bob", -10.0);
        assert!(agent.process_payment(&req).is_err());
    }

    #[test]
    fn test_transaction_history() {
        let mut agent = make_agent();
        agent.transfer("alice", "bob", 10.0).unwrap();
        agent.transfer("bob", "alice", 5.0).unwrap();
        assert_eq!(agent.transaction_history().len(), 2);
    }

    #[test]
    fn test_account_transactions() {
        let mut agent = make_agent();
        agent.transfer("alice", "bob", 10.0).unwrap();
        agent.transfer("bob", "alice", 5.0).unwrap();
        let alice_txns = agent.account_transactions("alice");
        assert_eq!(alice_txns.len(), 2);
    }

    #[test]
    fn test_fee_deducted() {
        let mut agent = make_agent();
        agent.transfer("alice", "bob", 100.0).unwrap();
        let alice = agent.check_balance("alice").unwrap();
        // Should be less than 900 due to fee
        assert!(alice.available < 900.0);
    }

    #[test]
    fn test_payment_with_memo() {
        let mut agent = make_agent();
        let req = PaymentRequest::new("alice", "bob", 10.0).with_memo("lunch");
        let result = agent.process_payment(&req).unwrap();
        assert_eq!(result.status, TransactionStatus::Completed);
    }
}
