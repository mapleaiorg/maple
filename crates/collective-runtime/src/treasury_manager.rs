//! Treasury Manager — financial operations for Collectives
//!
//! Wraps the Treasury type with policy checks, audit trails,
//! and escrow lifecycle management. All financial operations
//! produce receipts for accountability.

use collective_types::{
    Account, AccountId, AccountType, Amount, AuditJournal, CollectiveError, CollectiveId,
    CollectiveReceipt, CollectiveResult, Escrow, EscrowId, ReceiptType, Treasury,
};
use resonator_types::ResonatorId;
use tracing::{info, warn};

/// Manages treasury operations with policy enforcement and audit trails
pub struct TreasuryManager {
    /// The underlying treasury
    treasury: Treasury,
    /// Maximum single withdrawal without additional approval
    max_single_withdrawal: Amount,
}

impl TreasuryManager {
    /// Create a new treasury manager with a fresh treasury
    pub fn new(collective_id: CollectiveId) -> Self {
        Self {
            treasury: Treasury::new(collective_id),
            max_single_withdrawal: Amount::new(u64::MAX), // No limit by default
        }
    }

    /// Create from an existing treasury
    pub fn from_treasury(treasury: Treasury) -> Self {
        Self {
            treasury,
            max_single_withdrawal: Amount::new(u64::MAX),
        }
    }

    /// Set maximum single withdrawal
    pub fn set_max_single_withdrawal(&mut self, max: Amount) {
        self.max_single_withdrawal = max;
    }

    // --- Account operations ---

    /// Create a new account
    pub fn create_account(
        &mut self,
        name: impl Into<String>,
        account_type: AccountType,
        journal: &mut AuditJournal,
    ) -> AccountId {
        let account = Account::new(name, account_type);
        let id = account.id.clone();
        self.treasury.add_account(account);

        journal.log_receipt(CollectiveReceipt::new(
            self.treasury.collective_id.clone(),
            ReceiptType::Financial,
            ResonatorId::new("system"),
            format!("Account created: {}", id),
        ));

        id
    }

    /// Deposit into an account
    pub fn deposit(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
        depositor: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        self.treasury.deposit(account_id, amount)?;

        info!(
            account = %account_id,
            amount = amount.0,
            depositor = %depositor,
            "Deposit completed"
        );

        journal.log_receipt(
            CollectiveReceipt::new(
                self.treasury.collective_id.clone(),
                ReceiptType::Financial,
                depositor.clone(),
                format!("Deposit: {} into {}", amount, account_id),
            )
            .with_metadata("amount", amount.0.to_string())
            .with_metadata("account", account_id.0.clone()),
        );

        Ok(())
    }

    /// Withdraw from an account (with policy check)
    pub fn withdraw(
        &mut self,
        account_id: &AccountId,
        amount: Amount,
        withdrawer: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        // Policy check: single withdrawal limit
        if amount > self.max_single_withdrawal {
            warn!(
                amount = amount.0,
                limit = self.max_single_withdrawal.0,
                withdrawer = %withdrawer,
                "Withdrawal exceeds single transaction limit"
            );
            return Err(CollectiveError::PolicyViolation(format!(
                "Withdrawal {} exceeds single transaction limit {}",
                amount, self.max_single_withdrawal
            )));
        }

        self.treasury.withdraw(account_id, amount)?;

        info!(
            account = %account_id,
            amount = amount.0,
            withdrawer = %withdrawer,
            "Withdrawal completed"
        );

        journal.log_receipt(
            CollectiveReceipt::new(
                self.treasury.collective_id.clone(),
                ReceiptType::Financial,
                withdrawer.clone(),
                format!("Withdrawal: {} from {}", amount, account_id),
            )
            .with_metadata("amount", amount.0.to_string())
            .with_metadata("account", account_id.0.clone()),
        );

        Ok(())
    }

    /// Transfer between accounts
    pub fn transfer(
        &mut self,
        from_account: &AccountId,
        to_account: &AccountId,
        amount: Amount,
        initiator: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        // Validate both accounts exist
        if self.treasury.get_account(from_account).is_none() {
            return Err(CollectiveError::AccountNotFound(from_account.0.clone()));
        }
        if self.treasury.get_account(to_account).is_none() {
            return Err(CollectiveError::AccountNotFound(to_account.0.clone()));
        }

        // Withdraw from source
        self.treasury.withdraw(from_account, amount)?;
        // Deposit to destination
        self.treasury.deposit(to_account, amount)?;

        info!(
            from = %from_account,
            to = %to_account,
            amount = amount.0,
            "Transfer completed"
        );

        journal.log_receipt(
            CollectiveReceipt::new(
                self.treasury.collective_id.clone(),
                ReceiptType::Financial,
                initiator.clone(),
                format!(
                    "Transfer: {} from {} to {}",
                    amount, from_account, to_account
                ),
            )
            .with_metadata("amount", amount.0.to_string())
            .with_metadata("from_account", from_account.0.clone())
            .with_metadata("to_account", to_account.0.clone()),
        );

        Ok(())
    }

    // --- Escrow operations ---

    /// Create an escrow
    pub fn create_escrow(
        &mut self,
        amount: Amount,
        depositor: ResonatorId,
        beneficiary: ResonatorId,
        conditions: Vec<String>,
        journal: &mut AuditJournal,
    ) -> EscrowId {
        let mut escrow = Escrow::new(amount, depositor.clone(), beneficiary.clone());
        for condition in &conditions {
            escrow = escrow.with_condition(condition.clone());
        }

        let id = self.treasury.create_escrow(escrow);

        info!(
            escrow_id = %id,
            amount = amount.0,
            depositor = %depositor,
            beneficiary = %beneficiary,
            "Escrow created"
        );

        journal.log_receipt(
            CollectiveReceipt::new(
                self.treasury.collective_id.clone(),
                ReceiptType::Financial,
                depositor,
                format!("Escrow created: {} for {}", id, amount),
            )
            .with_metadata("escrow_id", id.0.clone())
            .with_metadata("amount", amount.0.to_string()),
        );

        id
    }

    /// Release an escrow (to beneficiary)
    pub fn release_escrow(
        &mut self,
        escrow_id: &EscrowId,
        releaser: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let escrow = self
            .treasury
            .get_escrow_mut(escrow_id)
            .ok_or_else(|| CollectiveError::EscrowNotFound(escrow_id.0.clone()))?;

        if !escrow.is_held() {
            return Err(CollectiveError::TreasuryError(format!(
                "Escrow {} is not in held state (status: {:?})",
                escrow_id, escrow.status
            )));
        }

        escrow.release();

        info!(escrow_id = %escrow_id, releaser = %releaser, "Escrow released");

        journal.log_receipt(CollectiveReceipt::new(
            self.treasury.collective_id.clone(),
            ReceiptType::Financial,
            releaser.clone(),
            format!("Escrow released: {}", escrow_id),
        ));

        Ok(())
    }

    /// Refund an escrow (back to depositor)
    pub fn refund_escrow(
        &mut self,
        escrow_id: &EscrowId,
        refunder: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let escrow = self
            .treasury
            .get_escrow_mut(escrow_id)
            .ok_or_else(|| CollectiveError::EscrowNotFound(escrow_id.0.clone()))?;

        if !escrow.is_held() {
            return Err(CollectiveError::TreasuryError(format!(
                "Escrow {} is not in held state",
                escrow_id
            )));
        }

        escrow.refund();

        info!(escrow_id = %escrow_id, "Escrow refunded");

        journal.log_receipt(CollectiveReceipt::new(
            self.treasury.collective_id.clone(),
            ReceiptType::Financial,
            refunder.clone(),
            format!("Escrow refunded: {}", escrow_id),
        ));

        Ok(())
    }

    /// Dispute an escrow
    pub fn dispute_escrow(
        &mut self,
        escrow_id: &EscrowId,
        disputer: &ResonatorId,
        journal: &mut AuditJournal,
    ) -> CollectiveResult<()> {
        let escrow = self
            .treasury
            .get_escrow_mut(escrow_id)
            .ok_or_else(|| CollectiveError::EscrowNotFound(escrow_id.0.clone()))?;

        if !escrow.is_held() {
            return Err(CollectiveError::TreasuryError(format!(
                "Escrow {} is not in held state",
                escrow_id
            )));
        }

        escrow.dispute();

        warn!(escrow_id = %escrow_id, disputer = %disputer, "Escrow disputed");

        journal.log_receipt(CollectiveReceipt::new(
            self.treasury.collective_id.clone(),
            ReceiptType::Financial,
            disputer.clone(),
            format!("Escrow disputed: {}", escrow_id),
        ));

        Ok(())
    }

    // --- Query methods ---

    pub fn treasury(&self) -> &Treasury {
        &self.treasury
    }

    pub fn total_balance(&self) -> Amount {
        self.treasury.total_balance()
    }

    pub fn total_escrowed(&self) -> Amount {
        self.treasury.total_escrowed()
    }

    pub fn get_account(&self, id: &AccountId) -> Option<&Account> {
        self.treasury.get_account(id)
    }

    pub fn get_escrow(&self, id: &EscrowId) -> Option<&Escrow> {
        self.treasury.get_escrow(id)
    }

    pub fn receipt_count(&self) -> usize {
        self.treasury.receipts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use collective_types::EscrowStatus;

    fn setup() -> (TreasuryManager, AuditJournal) {
        let id = CollectiveId::new("test");
        (TreasuryManager::new(id.clone()), AuditJournal::new(id))
    }

    #[test]
    fn test_deposit_withdraw() {
        let (mut mgr, mut journal) = setup();
        let actor = ResonatorId::new("actor-1");
        let op_id = AccountId::new("operating");

        mgr.deposit(&op_id, Amount::new(10_000), &actor, &mut journal)
            .unwrap();
        assert_eq!(mgr.total_balance(), Amount::new(10_000));

        mgr.withdraw(&op_id, Amount::new(3_000), &actor, &mut journal)
            .unwrap();
        assert_eq!(mgr.total_balance(), Amount::new(7_000));
    }

    #[test]
    fn test_withdrawal_limit() {
        let (mut mgr, mut journal) = setup();
        let actor = ResonatorId::new("actor-1");
        let op_id = AccountId::new("operating");

        mgr.set_max_single_withdrawal(Amount::new(5_000));
        mgr.deposit(&op_id, Amount::new(100_000), &actor, &mut journal)
            .unwrap();

        let result = mgr.withdraw(&op_id, Amount::new(10_000), &actor, &mut journal);
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer() {
        let (mut mgr, mut journal) = setup();
        let actor = ResonatorId::new("actor-1");
        let op_id = AccountId::new("operating");

        let reserve_id = mgr.create_account("Reserve", AccountType::Reserve, &mut journal);

        mgr.deposit(&op_id, Amount::new(10_000), &actor, &mut journal)
            .unwrap();

        mgr.transfer(
            &op_id,
            &reserve_id,
            Amount::new(3_000),
            &actor,
            &mut journal,
        )
        .unwrap();

        assert_eq!(mgr.get_account(&op_id).unwrap().balance, Amount::new(7_000));
        assert_eq!(
            mgr.get_account(&reserve_id).unwrap().balance,
            Amount::new(3_000)
        );
    }

    #[test]
    fn test_escrow_lifecycle() {
        let (mut mgr, mut journal) = setup();
        let depositor = ResonatorId::new("depositor");
        let beneficiary = ResonatorId::new("beneficiary");

        let escrow_id = mgr.create_escrow(
            Amount::new(5_000),
            depositor.clone(),
            beneficiary.clone(),
            vec!["Work completed".into()],
            &mut journal,
        );

        assert_eq!(mgr.total_escrowed(), Amount::new(5_000));

        mgr.release_escrow(&escrow_id, &depositor, &mut journal)
            .unwrap();

        assert_eq!(mgr.total_escrowed(), Amount::zero());
        assert_eq!(
            mgr.get_escrow(&escrow_id).unwrap().status,
            EscrowStatus::Released
        );
    }

    #[test]
    fn test_escrow_dispute() {
        let (mut mgr, mut journal) = setup();
        let depositor = ResonatorId::new("depositor");
        let beneficiary = ResonatorId::new("beneficiary");

        let escrow_id = mgr.create_escrow(
            Amount::new(5_000),
            depositor.clone(),
            beneficiary.clone(),
            vec![],
            &mut journal,
        );

        mgr.dispute_escrow(&escrow_id, &beneficiary, &mut journal)
            .unwrap();

        assert_eq!(
            mgr.get_escrow(&escrow_id).unwrap().status,
            EscrowStatus::Disputed
        );
    }

    #[test]
    fn test_escrow_refund() {
        let (mut mgr, mut journal) = setup();
        let depositor = ResonatorId::new("depositor");
        let beneficiary = ResonatorId::new("beneficiary");

        let escrow_id = mgr.create_escrow(
            Amount::new(5_000),
            depositor.clone(),
            beneficiary,
            vec![],
            &mut journal,
        );

        mgr.refund_escrow(&escrow_id, &depositor, &mut journal)
            .unwrap();

        assert_eq!(
            mgr.get_escrow(&escrow_id).unwrap().status,
            EscrowStatus::Refunded
        );
    }

    #[test]
    fn test_cannot_release_disputed_escrow() {
        let (mut mgr, mut journal) = setup();
        let depositor = ResonatorId::new("depositor");
        let beneficiary = ResonatorId::new("beneficiary");

        let escrow_id = mgr.create_escrow(
            Amount::new(5_000),
            depositor.clone(),
            beneficiary.clone(),
            vec![],
            &mut journal,
        );

        mgr.dispute_escrow(&escrow_id, &beneficiary, &mut journal)
            .unwrap();

        let result = mgr.release_escrow(&escrow_id, &depositor, &mut journal);
        assert!(result.is_err());
    }
}
