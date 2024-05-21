use crate::types::*;

pub fn deposit(
    op: Deposit,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    let account = accounts.entry(op.client).or_insert(Account {
        available: 0.0,
        held: 0.0,
        total: 0.0,
        locked: false,
    });
    if account.locked {
        return Err(TXError::AccountLocked(TX::Deposit(op)));
    }
    account.available += op.amount;
    account.total += op.amount;
    transactions.insert(
        op.tx,
        TXState {
            client: op.client,
            amount: op.amount,
            disputed: false,
        },
    );
    return Ok(());
}

pub fn withdraw(
    op: Withdrawal,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    let account = match accounts.get_mut(&op.client) {
        Some(acc) => acc,
        None => return Err(TXError::AccountNotFound(TX::Withdrawal(op))),
    };
    if account.locked {
        return Err(TXError::AccountLocked(TX::Withdrawal(op)));
    }
    if account.available < op.amount {
        return Err(TXError::NotEnoughFunds(
            account.available,
            op.amount,
            TX::Withdrawal(op),
        ));
    }
    account.available -= op.amount;
    account.total -= op.amount;
    transactions.insert(
        op.tx,
        TXState {
            client: op.client,
            amount: op.amount,
            disputed: false,
        },
    );

    return Ok(());
}

pub fn dispute(
    op: Dispute,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    let parent_tx = match transactions.get_mut(&op.tx) {
        Some(tx) => tx,
        None => return Err(TXError::ParentTXNotFound(TX::Dispute(op))),
    };
    let account = match accounts.get_mut(&parent_tx.client) {
        Some(acc) => acc,
        None => return Err(TXError::AccountNotFound(TX::Dispute(op))),
    };

    if op.client != parent_tx.client {
        return Err(TXError::ClientsDontMatch(parent_tx.client, TX::Dispute(op)));
    }
    if account.locked {
        return Err(TXError::AccountLocked(TX::Dispute(op)));
    }
    if parent_tx.disputed {
        return Err(TXError::ParentTXAlreadyDisputed(TX::Dispute(op)));
    }
    if account.available < parent_tx.amount {
        return Err(TXError::NotEnoughFunds(
            account.available,
            parent_tx.amount,
            TX::Dispute(op),
        ));
    }

    account.available -= parent_tx.amount;
    account.held += parent_tx.amount;
    parent_tx.disputed = true;
    return Ok(());
}

pub fn resolve(
    op: Resolve,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    let parent_tx = match transactions.get_mut(&op.tx) {
        Some(tx) => tx,
        None => return Err(TXError::ParentTXNotFound(TX::Resolve(op))),
    };
    let account = match accounts.get_mut(&op.client) {
        Some(acc) => acc,
        None => return Err(TXError::AccountNotFound(TX::Resolve(op))),
    };

    if op.client != parent_tx.client {
        return Err(TXError::ClientsDontMatch(parent_tx.client, TX::Resolve(op)));
    }
    if account.locked {
        return Err(TXError::AccountLocked(TX::Resolve(op)));
    }
    if !parent_tx.disputed {
        return Err(TXError::ParentTXNotDisputed(TX::Resolve(op)));
    }

    account.available += parent_tx.amount;
    account.held -= parent_tx.amount;
    transactions.remove(&op.tx);
    return Ok(());
}

pub fn chargeback(
    op: Chargeback,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    let parent_tx = match transactions.get(&op.tx) {
        Some(tx) => tx,
        None => return Err(TXError::ParentTXNotFound(TX::Chargeback(op))),
    };
    let account = match accounts.get_mut(&op.client) {
        Some(acc) => acc,
        None => return Err(TXError::AccountNotFound(TX::Chargeback(op))),
    };

    if op.client != parent_tx.client {
        return Err(TXError::ClientsDontMatch(
            parent_tx.client,
            TX::Chargeback(op),
        ));
    }
    if account.locked {
        return Err(TXError::AccountLocked(TX::Chargeback(op)));
    }
    if !parent_tx.disputed {
        return Err(TXError::ParentTXNotDisputed(TX::Chargeback(op)));
    }

    account.held -= parent_tx.amount;
    account.total -= parent_tx.amount;
    account.locked = true;
    transactions.remove(&op.tx);
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_deposit() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Deposit {
            client: 1,
            tx: 1,
            amount: 1.0,
        };
        deposit(op, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 1.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.0);
        assert_eq!(transactions.get(&1).unwrap().amount, 1.0);
    }

    #[test]
    fn test_withdraw() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Withdrawal {
            client: 1,
            tx: 1,
            amount: 1.0,
        };
        deposit(
            Deposit {
                client: 1,
                tx: 1,
                amount: 1.0,
            },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        withdraw(op, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 0.0);
        assert_eq!(transactions.get(&1).unwrap().amount, 1.0);
    }

    #[test]
    fn test_dispute() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        deposit(
            Deposit {
                client: 1,
                tx: 1,
                amount: 1.0,
            },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        dispute(op, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 0.0);
        assert_eq!(accounts.get(&1).unwrap().held, 1.0);
        assert_eq!(transactions.get(&1).unwrap().disputed, true);
    }

    #[test]
    fn test_resolve() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Resolve { client: 1, tx: 1 };
        deposit(
            Deposit {
                client: 1,
                tx: 1,
                amount: 1.0,
            },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        dispute(
            Dispute { client: 1, tx: 1 },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        resolve(op, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 1.0);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(transactions.get(&1), None);
    }

    #[test]
    fn test_chargeback() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        deposit(
            Deposit {
                client: 1,
                tx: 1,
                amount: 1.0,
            },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        dispute(
            Dispute { client: 1, tx: 1 },
            &mut accounts,
            &mut transactions,
        )
        .unwrap();
        chargeback(op, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 0.0);
        assert_eq!(accounts.get(&1).unwrap().locked, true);
        assert_eq!(transactions.get(&1), None);
    }

    #[test]
    fn test_deposit_locked_account() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Deposit {
            client: 1,
            tx: 1,
            amount: 1.0,
        };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: true,
            },
        );
        assert_eq!(
            deposit(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountLocked(TX::Deposit(op)))
        );
    }

    #[test]
    fn test_withdraw_not_enough_funds() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Withdrawal {
            client: 1,
            tx: 1,
            amount: 1.0,
        };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        assert_eq!(
            withdraw(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::NotEnoughFunds(0.0, op.amount, TX::Withdrawal(op)))
        );
    }

    #[test]
    fn test_dispute_parent_tx_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        assert_eq!(
            dispute(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXNotFound(TX::Dispute(op)))
        );
    }

    #[test]
    fn test_dispute_account_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: false,
            },
        );
        assert_eq!(
            dispute(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountNotFound(TX::Dispute(op)))
        );
    }

    #[test]
    fn test_dispute_account_locked() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: false,
            },
        );
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: true,
            },
        );
        assert_eq!(
            dispute(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountLocked(TX::Dispute(op)))
        );
    }

    #[test]
    fn test_dispute_parent_tx_already_disputed() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: true,
            },
        );
        assert_eq!(
            dispute(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXAlreadyDisputed(TX::Dispute(op)))
        );
    }

    #[test]
    fn test_dispute_not_enough_funds() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Dispute { client: 1, tx: 1 };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: false,
            },
        );
        assert_eq!(
            dispute(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::NotEnoughFunds(0.0, 1.0, TX::Dispute(op)))
        );
    }

    #[test]
    fn test_resolve_parent_tx_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Resolve { client: 1, tx: 1 };
        assert_eq!(
            resolve(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXNotFound(TX::Resolve(op)))
        );
    }

    #[test]
    fn test_resolve_account_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Resolve { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: true,
            },
        );
        assert_eq!(
            resolve(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountNotFound(TX::Resolve(op)))
        );
    }

    #[test]
    fn test_resolve_account_locked() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Resolve { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: true,
            },
        );
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: true,
            },
        );
        assert_eq!(
            resolve(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountLocked(TX::Resolve(op)))
        );
    }

    #[test]
    fn test_resolve_parent_tx_not_disputed() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Resolve { client: 1, tx: 1 };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: false,
            },
        );
        assert_eq!(
            resolve(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXNotDisputed(TX::Resolve(op)))
        );
    }

    #[test]
    fn test_chargeback_parent_tx_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        assert_eq!(
            chargeback(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXNotFound(TX::Chargeback(op)))
        );
    }

    #[test]
    fn test_chargeback_account_not_found() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: true,
            },
        );
        assert_eq!(
            chargeback(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountNotFound(TX::Chargeback(op)))
        );
    }

    #[test]
    fn test_chargeback_account_locked() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: true,
            },
        );
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: true,
            },
        );
        assert_eq!(
            chargeback(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::AccountLocked(TX::Chargeback(op)))
        );
    }

    #[test]
    fn test_chargeback_parent_tx_not_disputed() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        transactions.insert(
            1,
            TXState {
                client: 1,
                amount: 1.0,
                disputed: false,
            },
        );
        assert_eq!(
            chargeback(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ParentTXNotDisputed(TX::Chargeback(op)))
        );
    }

    #[test]
    fn test_chargeback_clients_dont_match() {
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();
        let op = Chargeback { client: 1, tx: 1 };
        accounts.insert(
            1,
            Account {
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            },
        );
        transactions.insert(
            1,
            TXState {
                client: 2,
                amount: 1.0,
                disputed: true,
            },
        );
        assert_eq!(
            chargeback(op.clone(), &mut accounts, &mut transactions),
            Err(TXError::ClientsDontMatch(2, TX::Chargeback(op)))
        );
    }
}
