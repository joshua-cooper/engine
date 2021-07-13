use {
    crate::{Amount, TransactionId},
    std::collections::HashMap,
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum DepositError {
    #[error("Transaction ID {0} has already been used")]
    DuplicateTransactionId(TransactionId),
    #[error("Account is locked")]
    AccountLocked,
}

#[derive(Debug, Error)]
pub enum WithdrawError {
    #[error("Account is locked")]
    AccountLocked,
    #[error("Insufficient funds")]
    InsufficientFunds,
}

#[derive(Debug, Error)]
pub enum DisputeError {
    #[error("Deposit does not exist")]
    DepositDoesNotExist,
    #[error("Deposit is already disputed")]
    DepositAlreadyDisputed,
    #[error("Deposit has already been reversed")]
    DepositAlreadyReversed,
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("Deposit does not exist")]
    DepositDoesNotExist,
    #[error("Deposit is not currently disputed")]
    DepositNotDisputed,
    #[error("Deposit has already been reversed")]
    DepositAlreadyReversed,
}

#[derive(Debug, Error)]
pub enum ChargebackError {
    #[error("Deposit does not exist")]
    DepositDoesNotExist,
    #[error("Deposit is not currently disputed")]
    DepositNotDisputed,
    #[error("Deposit has already been reversed")]
    DepositAlreadyReversed,
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("Deposit error: {0}")]
    Deposit(#[from] DepositError),
    #[error("Withdraw error: {0}")]
    Withdraw(#[from] WithdrawError),
    #[error("Dispute error: {0}")]
    Dispute(#[from] DisputeError),
    #[error("Resolve error: {0}")]
    Resolve(#[from] ResolveError),
    #[error("Chargeback error: {0}")]
    Chargeback(#[from] ChargebackError),
}

#[derive(Debug, Clone, Copy)]
enum DepositState {
    MaybeSettled,
    Disputed,
    Reversed,
}

#[derive(Debug, Clone, Copy)]
struct ProcessedDeposit {
    state: DepositState,
    amount: Amount,
}

impl ProcessedDeposit {
    fn new(amount: Amount) -> Self {
        Self {
            state: DepositState::MaybeSettled,
            amount,
        }
    }
}

/// Thin wrapper around `std::collections::HashMap` that manages the finite state machines for a
/// collection of deposits.
#[derive(Debug, Default, Clone)]
struct DepositHistory {
    inner: HashMap<TransactionId, ProcessedDeposit>,
}

impl DepositHistory {
    fn insert(
        &mut self,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), DepositError> {
        if self.inner.contains_key(&transaction_id) {
            return Err(DepositError::DuplicateTransactionId(transaction_id));
        }
        self.inner
            .insert(transaction_id, ProcessedDeposit::new(amount));
        Ok(())
    }

    fn dispute(&mut self, transaction_id: TransactionId) -> Result<&Amount, DisputeError> {
        let deposit = self
            .inner
            .get_mut(&transaction_id)
            .ok_or(DisputeError::DepositDoesNotExist)?;
        match deposit.state {
            DepositState::MaybeSettled => {
                deposit.state = DepositState::Disputed;
                Ok(&deposit.amount)
            }
            DepositState::Disputed => Err(DisputeError::DepositAlreadyDisputed),
            DepositState::Reversed => Err(DisputeError::DepositAlreadyReversed),
        }
    }

    fn resolve(&mut self, transaction_id: TransactionId) -> Result<&Amount, ResolveError> {
        let deposit = self
            .inner
            .get_mut(&transaction_id)
            .ok_or(ResolveError::DepositDoesNotExist)?;
        match deposit.state {
            DepositState::MaybeSettled => Err(ResolveError::DepositNotDisputed),
            DepositState::Disputed => {
                deposit.state = DepositState::MaybeSettled;
                Ok(&deposit.amount)
            }
            DepositState::Reversed => Err(ResolveError::DepositAlreadyReversed),
        }
    }

    fn chargeback(&mut self, transaction_id: TransactionId) -> Result<&Amount, ChargebackError> {
        let deposit = self
            .inner
            .get_mut(&transaction_id)
            .ok_or(ChargebackError::DepositDoesNotExist)?;
        match deposit.state {
            DepositState::MaybeSettled => Err(ChargebackError::DepositNotDisputed),
            DepositState::Disputed => {
                deposit.state = DepositState::Reversed;
                Ok(&deposit.amount)
            }
            DepositState::Reversed => Err(ChargebackError::DepositAlreadyReversed),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Account {
    locked: bool,
    available_funds: Amount,
    held_funds: Amount,
    deposit_history: DepositHistory,
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn available_funds(&self) -> Amount {
        self.available_funds
    }

    pub fn held_funds(&self) -> Amount {
        self.held_funds
    }

    pub fn total_funds(&self) -> Amount {
        self.available_funds + self.held_funds
    }

    pub fn deposit(
        &mut self,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), DepositError> {
        if self.locked {
            return Err(DepositError::AccountLocked);
        }

        self.deposit_history.insert(transaction_id, amount)?;
        self.available_funds += amount;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: Amount) -> Result<(), WithdrawError> {
        if self.locked {
            return Err(WithdrawError::AccountLocked);
        }

        if self.available_funds < amount {
            return Err(WithdrawError::InsufficientFunds);
        }

        self.available_funds -= amount;

        Ok(())
    }

    pub fn dispute(&mut self, transaction_id: TransactionId) -> Result<(), DisputeError> {
        let amount = self.deposit_history.dispute(transaction_id)?;
        self.available_funds -= *amount;
        self.held_funds += *amount;
        Ok(())
    }

    pub fn resolve(&mut self, transaction_id: TransactionId) -> Result<(), ResolveError> {
        let amount = self.deposit_history.resolve(transaction_id)?;
        self.held_funds -= *amount;
        self.available_funds += *amount;
        Ok(())
    }

    pub fn chargeback(&mut self, transaction_id: TransactionId) -> Result<(), ChargebackError> {
        let amount = self.deposit_history.chargeback(transaction_id)?;
        self.held_funds -= *amount;
        self.locked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, rust_decimal_macros::dec};

    #[test]
    fn can_deposit_funds() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));

        assert!(a.is_ok());
        assert_eq!(account.available_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn can_withdraw_funds() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.withdraw(Amount::from(dec!(10)));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert_eq!(account.available_funds(), Amount::from(dec!(140.99)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(140.99)));
    }

    #[test]
    fn cannot_withdraw_too_much() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.withdraw(Amount::from(dec!(160)));

        assert!(a.is_ok());
        assert!(b.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn can_dispute_existing_deposit() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn ignores_dispute_without_deposit() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(2));

        assert!(a.is_ok());
        assert!(b.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn ignores_double_dispute() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.dispute(TransactionId::from(1));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn can_resolve_after_dispute() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.resolve(TransactionId::from(1));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_ok());
        assert_eq!(account.available_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn ignores_resolve_without_dispute() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.resolve(TransactionId::from(2));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn can_chargeback_after_dispute() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.chargeback(TransactionId::from(1));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_ok());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(0)));
    }

    #[test]
    fn ignores_chargeback_without_dispute() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.chargeback(TransactionId::from(2));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(150.99)));
        assert_eq!(account.total_funds(), Amount::from(dec!(150.99)));
    }

    #[test]
    fn cannot_dispute_again_after_chargeback() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.chargeback(TransactionId::from(1));
        let d = account.dispute(TransactionId::from(1));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_ok());
        assert!(d.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(0)));
    }

    #[test]
    fn cannot_deposit_after_account_is_locked() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.dispute(TransactionId::from(1));
        let c = account.chargeback(TransactionId::from(1));
        let d = account.deposit(TransactionId::from(2), Amount::from(dec!(123.45)));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_ok());
        assert!(d.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(0)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(0)));
    }

    #[test]
    fn cannot_withdraw_after_account_is_locked() {
        let mut account = Account::new();

        let a = account.deposit(TransactionId::from(1), Amount::from(dec!(150.99)));
        let b = account.deposit(TransactionId::from(2), Amount::from(dec!(123.45)));
        let c = account.dispute(TransactionId::from(1));
        let d = account.chargeback(TransactionId::from(1));
        let e = account.withdraw(Amount::from(dec!(1.50)));

        assert!(a.is_ok());
        assert!(b.is_ok());
        assert!(c.is_ok());
        assert!(d.is_ok());
        assert!(e.is_err());
        assert_eq!(account.available_funds(), Amount::from(dec!(123.45)));
        assert_eq!(account.held_funds(), Amount::from(dec!(0)));
        assert_eq!(account.total_funds(), Amount::from(dec!(123.45)));
    }
}
