use crate::transactions::*;
use crate::types::*;

pub fn process_transaction(
    transaction: TX,
    accounts: &mut Accounts,
    transactions: &mut Transactions,
) -> Result<(), TXError> {
    match transaction {
        TX::Deposit(operation) => deposit(operation, accounts, transactions),
        TX::Withdrawal(operation) => withdraw(operation, accounts, transactions),
        TX::Dispute(operation) => dispute(operation, accounts, transactions),
        TX::Resolve(operation) => resolve(operation, accounts, transactions),
        TX::Chargeback(operation) => chargeback(operation, accounts, transactions),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_transaction() {
        let mut accounts = Accounts::new();
        let mut transactions = Transactions::new();

        let transaction = TX::Deposit(Deposit {
            client: 1,
            tx: 1,
            amount: 1.0,
        });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 1.0);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.0);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Deposit(Deposit {
            client: 1,
            tx: 2,
            amount: 1.0,
        });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 2.0);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 2.0);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Withdrawal(Withdrawal {
            client: 1,
            tx: 3,
            amount: 0.5,
        });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 1.5);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.5);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Dispute(Dispute { client: 1, tx: 1 });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 0.5);
        assert_eq!(accounts.get(&1).unwrap().held, 1.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.5);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Resolve(Resolve { client: 1, tx: 1 });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 1.5);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.5);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Dispute(Dispute { client: 1, tx: 2 });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 0.5);
        assert_eq!(accounts.get(&1).unwrap().held, 1.0);
        assert_eq!(accounts.get(&1).unwrap().total, 1.5);
        assert_eq!(accounts.get(&1).unwrap().locked, false);

        let transaction = TX::Chargeback(Chargeback { client: 1, tx: 2 });
        process_transaction(transaction, &mut accounts, &mut transactions).unwrap();
        assert_eq!(accounts.get(&1).unwrap().available, 0.5);
        assert_eq!(accounts.get(&1).unwrap().held, 0.0);
        assert_eq!(accounts.get(&1).unwrap().total, 0.5);
        assert_eq!(accounts.get(&1).unwrap().locked, true);
    }
}
