//! Treasury types: asset management for Collectives
//!
//! The Treasury holds the collective's financial state—accounts,
//! escrows, allocations, and receipts. It's a data structure,
//! not an execution engine.

use crate::{Amount, CollectiveId};
use chrono::{DateTime, Utc};
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a Treasury account
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

impl AccountId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A financial account within a Treasury
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    /// Unique account identifier
    pub id: AccountId,
    /// Human-readable account name
    pub name: String,
    /// Current balance
    pub balance: Amount,
    /// Type of account
    pub account_type: AccountType,
    /// When the account was created
    pub created_at: DateTime<Utc>,
}

impl Account {
    pub fn new(name: impl Into<String>, account_type: AccountType) -> Self {
        Self {
            id: AccountId::generate(),
            name: name.into(),
            balance: Amount::zero(),
            account_type,
            created_at: Utc::now(),
        }
    }

    pub fn with_id(mut self, id: AccountId) -> Self {
        self.id = id;
        self
    }

    pub fn with_balance(mut self, balance: Amount) -> Self {
        self.balance = balance;
        self
    }

    /// Deposit into the account
    pub fn deposit(&mut self, amount: Amount) {
        self.balance = self.balance.saturating_add(amount);
    }

    /// Withdraw from the account (returns error if insufficient)
    pub fn withdraw(&mut self, amount: Amount) -> Result<(), crate::CollectiveError> {
        if self.balance < amount {
            return Err(crate::CollectiveError::InsufficientBudget {
                required: amount.0,
                available: self.balance.0,
            });
        }
        self.balance = self.balance.saturating_sub(amount);
        Ok(())
    }
}

/// Types of treasury accounts
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccountType {
    /// Main operating account
    #[default]
    Operating,
    /// Reserve fund
    Reserve,
    /// Escrow holding account
    Escrow,
    /// Custom account type
    Custom(String),
}

/// Unique identifier for an Escrow
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EscrowId(pub String);

impl EscrowId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for EscrowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An escrow holding funds between two parties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Escrow {
    /// Unique escrow identifier
    pub id: EscrowId,
    /// Amount held in escrow
    pub amount: Amount,
    /// Who deposited the funds
    pub depositor: ResonatorId,
    /// Who will receive the funds on release
    pub beneficiary: ResonatorId,
    /// Conditions that must be met for release
    pub release_conditions: Vec<String>,
    /// Current status
    pub status: EscrowStatus,
    /// When the escrow was created
    pub created_at: DateTime<Utc>,
}

impl Escrow {
    pub fn new(amount: Amount, depositor: ResonatorId, beneficiary: ResonatorId) -> Self {
        Self {
            id: EscrowId::generate(),
            amount,
            depositor,
            beneficiary,
            release_conditions: Vec::new(),
            status: EscrowStatus::Held,
            created_at: Utc::now(),
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.release_conditions.push(condition.into());
        self
    }

    /// Release the escrow (mark as released)
    pub fn release(&mut self) {
        self.status = EscrowStatus::Released;
    }

    /// Refund the escrow (back to depositor)
    pub fn refund(&mut self) {
        self.status = EscrowStatus::Refunded;
    }

    /// Mark the escrow as disputed
    pub fn dispute(&mut self) {
        self.status = EscrowStatus::Disputed;
    }

    pub fn is_held(&self) -> bool {
        matches!(self.status, EscrowStatus::Held)
    }
}

/// Status of an escrow
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EscrowStatus {
    /// Funds are held
    #[default]
    Held,
    /// Funds released to beneficiary
    Released,
    /// Funds returned to depositor
    Refunded,
    /// Escrow is under dispute
    Disputed,
}

/// An allocation of funds to a specific resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Allocation {
    /// Who the allocation is for
    pub resonator_id: ResonatorId,
    /// Amount allocated
    pub amount: Amount,
    /// Purpose of the allocation
    pub purpose: String,
    /// When allocated
    pub allocated_at: DateTime<Utc>,
}

impl Allocation {
    pub fn new(resonator_id: ResonatorId, amount: Amount, purpose: impl Into<String>) -> Self {
        Self {
            resonator_id,
            amount,
            purpose: purpose.into(),
            allocated_at: Utc::now(),
        }
    }
}

/// The complete Treasury for a Collective
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Treasury {
    /// The collective this treasury belongs to
    pub collective_id: CollectiveId,
    /// All accounts
    pub accounts: HashMap<AccountId, Account>,
    /// All escrows
    pub escrows: HashMap<EscrowId, Escrow>,
    /// Allocations to members
    pub allocations: HashMap<ResonatorId, Allocation>,
    /// Receipts log
    pub receipts: Vec<TreasuryReceipt>,
}

impl Treasury {
    /// Create a new treasury with a default operating account
    pub fn new(collective_id: CollectiveId) -> Self {
        let mut accounts = HashMap::new();
        let operating =
            Account::new("Operating", AccountType::Operating).with_id(AccountId::new("operating"));
        accounts.insert(operating.id.clone(), operating);

        Self {
            collective_id,
            accounts,
            escrows: HashMap::new(),
            allocations: HashMap::new(),
            receipts: Vec::new(),
        }
    }

    /// Add an account
    pub fn add_account(&mut self, account: Account) {
        self.accounts.insert(account.id.clone(), account);
    }

    /// Get an account by ID
    pub fn get_account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.get(id)
    }

    /// Get a mutable account by ID
    pub fn get_account_mut(&mut self, id: &AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(id)
    }

    /// Create an escrow
    pub fn create_escrow(&mut self, escrow: Escrow) -> EscrowId {
        let id = escrow.id.clone();
        self.escrows.insert(id.clone(), escrow);
        self.emit_receipt(TreasuryOperation::EscrowCreate, Amount::zero());
        id
    }

    /// Get an escrow by ID
    pub fn get_escrow(&self, id: &EscrowId) -> Option<&Escrow> {
        self.escrows.get(id)
    }

    /// Get a mutable escrow by ID
    pub fn get_escrow_mut(&mut self, id: &EscrowId) -> Option<&mut Escrow> {
        self.escrows.get_mut(id)
    }

    /// Total balance across all accounts
    pub fn total_balance(&self) -> Amount {
        self.accounts
            .values()
            .fold(Amount::zero(), |acc, a| acc.saturating_add(a.balance))
    }

    /// Total held in escrow
    pub fn total_escrowed(&self) -> Amount {
        self.escrows
            .values()
            .filter(|e| e.is_held())
            .fold(Amount::zero(), |acc, e| acc.saturating_add(e.amount))
    }

    /// Emit a treasury receipt
    fn emit_receipt(&mut self, operation: TreasuryOperation, amount: Amount) {
        self.receipts.push(TreasuryReceipt {
            receipt_id: uuid::Uuid::new_v4().to_string(),
            operation,
            amount,
            timestamp: Utc::now(),
        });
    }

    /// Deposit into an account
    pub fn deposit(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
    ) -> Result<(), crate::CollectiveError> {
        let account = self
            .accounts
            .get_mut(account_id)
            .ok_or_else(|| crate::CollectiveError::AccountNotFound(account_id.0.clone()))?;
        account.deposit(amount);
        self.emit_receipt(TreasuryOperation::Deposit, amount);
        Ok(())
    }

    /// Withdraw from an account
    pub fn withdraw(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
    ) -> Result<(), crate::CollectiveError> {
        let account = self
            .accounts
            .get_mut(account_id)
            .ok_or_else(|| crate::CollectiveError::AccountNotFound(account_id.0.clone()))?;
        account.withdraw(amount)?;
        self.emit_receipt(TreasuryOperation::Withdrawal, amount);
        Ok(())
    }
}

/// A receipt for a treasury operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TreasuryReceipt {
    /// Unique receipt identifier
    pub receipt_id: String,
    /// What operation was performed
    pub operation: TreasuryOperation,
    /// Amount involved
    pub amount: Amount,
    /// When the operation occurred
    pub timestamp: DateTime<Utc>,
}

/// Types of treasury operations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreasuryOperation {
    /// Deposit into an account
    Deposit,
    /// Withdrawal from an account
    Withdrawal,
    /// Transfer between accounts
    Transfer,
    /// Escrow creation
    EscrowCreate,
    /// Escrow release
    EscrowRelease,
    /// Escrow refund
    EscrowRefund,
    /// Budget allocation to a member
    Allocation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_operations() {
        let mut account =
            Account::new("Test", AccountType::Operating).with_balance(Amount::new(1000));

        assert_eq!(account.balance, Amount::new(1000));

        account.deposit(Amount::new(500));
        assert_eq!(account.balance, Amount::new(1500));

        account.withdraw(Amount::new(300)).unwrap();
        assert_eq!(account.balance, Amount::new(1200));

        // Over-withdraw fails
        let result = account.withdraw(Amount::new(2000));
        assert!(result.is_err());
        assert_eq!(account.balance, Amount::new(1200)); // Unchanged
    }

    #[test]
    fn test_treasury_creation() {
        let treasury = Treasury::new(CollectiveId::new("coll-1"));
        assert!(treasury.get_account(&AccountId::new("operating")).is_some());
        assert_eq!(treasury.total_balance(), Amount::zero());
    }

    #[test]
    fn test_treasury_deposit_withdraw() {
        let mut treasury = Treasury::new(CollectiveId::new("coll-1"));
        let op_id = AccountId::new("operating");

        treasury.deposit(&op_id, Amount::new(10_000)).unwrap();
        assert_eq!(treasury.total_balance(), Amount::new(10_000));

        treasury.withdraw(&op_id, Amount::new(3_000)).unwrap();
        assert_eq!(treasury.total_balance(), Amount::new(7_000));

        assert_eq!(treasury.receipts.len(), 2);
    }

    #[test]
    fn test_escrow() {
        let mut treasury = Treasury::new(CollectiveId::new("coll-1"));
        let escrow = Escrow::new(
            Amount::new(5000),
            ResonatorId::new("depositor"),
            ResonatorId::new("beneficiary"),
        )
        .with_condition("Task completed");

        let escrow_id = treasury.create_escrow(escrow);
        assert_eq!(treasury.total_escrowed(), Amount::new(5000));

        let e = treasury.get_escrow_mut(&escrow_id).unwrap();
        assert!(e.is_held());
        e.release();
        assert!(!e.is_held());
        assert_eq!(e.status, EscrowStatus::Released);

        // Released escrows don't count in total escrowed
        assert_eq!(treasury.total_escrowed(), Amount::zero());
    }

    #[test]
    fn test_account_id() {
        let id = AccountId::generate();
        assert!(!id.0.is_empty());
        assert_eq!(format!("{}", AccountId::new("acc-1")), "acc-1");
    }

    #[test]
    fn test_escrow_id() {
        let id = EscrowId::generate();
        assert!(!id.0.is_empty());
    }

    #[test]
    fn test_nonexistent_account() {
        let mut treasury = Treasury::new(CollectiveId::new("coll-1"));
        let result = treasury.deposit(&AccountId::new("nonexistent"), Amount::new(100));
        assert!(result.is_err());
    }
}
