use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde::de::{Error, MapAccess, Visitor};

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub typ: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TransactionVisitor;

        impl<'de> Visitor<'de> for TransactionVisitor {
            type Value = Transaction;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map representing a Transaction")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Transaction, V::Error>
            where
                V: MapAccess<'de>,
            {
                let typ = map.next_value::<String>()?.trim().to_string();
                let typ = match typ.to_lowercase().as_str() {
                    s if s == TXType::Dispute.as_str() => Ok(s.to_string()),
                    s if s == TXType::Resolve.as_str() => Ok(s.to_string()),
                    s if s == TXType::Chargeback.as_str() => Ok(s.to_string()),
                    s if s == TXType::Deposit.as_str() => Ok(s.to_string()),
                    s if s == TXType::Withdrawal.as_str() => Ok(s.to_string()),
                    _ => return Err(V::Error::custom("Invalid transaction type")),
                }?;
                let client = map
                    .next_value::<String>()?
                    .trim()
                    .parse::<u16>()
                    .map_err(V::Error::custom)?;
                let tx = map
                    .next_value::<String>()?
                    .trim()
                    .parse::<u32>()
                    .map_err(V::Error::custom)?;
                let s: Option<String> = map.next_value()?;
                let amount = if let Some(s) = s {
                    let s = s.trim().to_string();
                    let f = f64::from_str(&s).map_err(V::Error::custom)?;
                    match typ.as_str() {
                        "deposit" | "withdrawal" => {
                            if f.is_normal() && f.is_sign_positive() && f >= 0.0001 {
                                Some(truncate(f))
                            } else {
                                return Err(V::Error::custom(format!(
                                    "Invalid amount value: {:?}",
                                    f
                                )));
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                Ok(Transaction {
                    typ,
                    client,
                    tx,
                    amount,
                })
            }
        }

        deserializer.deserialize_map(TransactionVisitor)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Account {
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

#[derive(Debug, PartialEq)]
pub struct TXState {
    pub client: u16,
    pub amount: f64,
    pub disputed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Deposit {
    pub client: u16,
    pub tx: u32,
    pub amount: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Withdrawal {
    pub client: u16,
    pub tx: u32,
    pub amount: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dispute {
    pub client: u16,
    pub tx: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resolve {
    pub client: u16,
    pub tx: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chargeback {
    pub client: u16,
    pub tx: u32,
}

#[derive(Debug, PartialEq)]
pub enum TX {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

#[derive(Debug)]
pub enum TXType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TXType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TXType::Deposit => "deposit",
            TXType::Withdrawal => "withdrawal",
            TXType::Dispute => "dispute",
            TXType::Resolve => "resolve",
            TXType::Chargeback => "chargeback",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TXBuildError {
    InvalidTransaction,
}

impl fmt::Display for TXBuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TXBuildError::InvalidTransaction => write!(f, "ValidationError: Invalid Transaction"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TXError {
    AccountLocked(TX),
    AccountNotFound(TX),
    ClientsDontMatch(u16, TX),
    NotEnoughFunds(f64, f64, TX),
    ParentTXAlreadyDisputed(TX),
    ParentTXNotDisputed(TX),
    ParentTXNotFound(TX),
}

impl fmt::Display for TXError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TXError::AccountLocked(tx) => write!(
                f,
                "TransactionError: Couldn't process {:?} as account for client is locked. {:?}",
                tx.name(),
                tx
            ),
            TXError::AccountNotFound(tx) => write!(
                f,
                "TransactionError: Account for client not found: {:?}",
                tx
            ),
            TXError::ClientsDontMatch(client, tx) => write!(
                f,
                "TransactionError: Clients don't match. Need {:?} have {:?}",
                client, tx
            ),
            TXError::NotEnoughFunds(available, needed, tx) => write!(
                f,
                "TransactionError: Not enough funds. Have {:?} need {:?}. {:?}",
                available, needed, tx
            ),
            TXError::ParentTXAlreadyDisputed(tx) => write!(
                f,
                "TransactionError: Parent transaction already disputed: {:?}",
                tx
            ),
            TXError::ParentTXNotDisputed(tx) => write!(
                f,
                "TransactionError: Parent transaction not disputed: {:?}",
                tx
            ),
            TXError::ParentTXNotFound(tx) => write!(
                f,
                "TransactionError: Parent transaction not found: {:?}",
                tx
            ),
        }
    }
}

impl TX {
    pub fn from_transaction(transaction: Transaction) -> Result<Self, TXBuildError> {
        match transaction.typ.as_str() {
            s if s == TXType::Deposit.as_str() => Ok(TX::Deposit(Deposit {
                client: transaction.client,
                tx: transaction.tx,
                amount: transaction.amount.unwrap(),
            })),
            s if s == TXType::Withdrawal.as_str() => Ok(TX::Withdrawal(Withdrawal {
                client: transaction.client,
                tx: transaction.tx,
                amount: transaction.amount.unwrap(),
            })),
            s if s == TXType::Dispute.as_str() => Ok(TX::Dispute(Dispute {
                client: transaction.client,
                tx: transaction.tx,
            })),
            s if s == TXType::Resolve.as_str() => Ok(TX::Resolve(Resolve {
                client: transaction.client,
                tx: transaction.tx,
            })),
            s if s == TXType::Chargeback.as_str() => Ok(TX::Chargeback(Chargeback {
                client: transaction.client,
                tx: transaction.tx,
            })),
            _ => Err(TXBuildError::InvalidTransaction),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TX::Deposit(_) => TXType::Deposit.as_str(),
            TX::Withdrawal(_) => TXType::Withdrawal.as_str(),
            TX::Dispute(_) => TXType::Dispute.as_str(),
            TX::Resolve(_) => TXType::Resolve.as_str(),
            TX::Chargeback(_) => TXType::Chargeback.as_str(),
        }
    }
}

pub fn truncate(f: f64) -> f64 {
    (f * 10000.0).trunc() / 10000.0
}

pub type Accounts = HashMap<u16, Account>;
pub type Transactions = HashMap<u32, TXState>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_transaction_deserialize() -> Result<(), Box<dyn Error>> {
        let csv_data = "\
type,client,tx,amount,somerandomfield
 deposit , 1, 1, 2500.12345, randominfo
withdrawal,1,1,1.0
dispute,1,1,0
dispute,1,1,1.0
resolve,1,1,
chargeback,1,1
";

        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(csv_data.as_bytes());
        let mut transactions = reader.deserialize();

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "deposit".to_string(),
                client: 1,
                tx: 1,
                amount: Some(2500.1234)
            }
        );

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "withdrawal".to_string(),
                client: 1,
                tx: 1,
                amount: Some(1.0)
            }
        );

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "dispute".to_string(),
                client: 1,
                tx: 1,
                amount: None
            }
        );

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "dispute".to_string(),
                client: 1,
                tx: 1,
                amount: None
            }
        );

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "resolve".to_string(),
                client: 1,
                tx: 1,
                amount: None
            }
        );

        let transaction: Transaction = transactions.next().unwrap()?;
        assert_eq!(
            transaction,
            Transaction {
                typ: "chargeback".to_string(),
                client: 1,
                tx: 1,
                amount: None
            }
        );

        Ok(())
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate(0.0001), 0.0001);
        assert_eq!(truncate(0.00001), 0.0000);
        assert_eq!(truncate(5.37895), 5.3789);
    }

    #[test]
    fn test_name() {
        assert_eq!(
            TX::Deposit(Deposit {
                client: 1,
                tx: 1,
                amount: 0.0001
            })
            .name(),
            "deposit"
        );
        assert_eq!(
            TX::Withdrawal(Withdrawal {
                client: 1,
                tx: 1,
                amount: 0.0001
            })
            .name(),
            "withdrawal"
        );
        assert_eq!(TX::Dispute(Dispute { client: 1, tx: 1 }).name(), "dispute");
        assert_eq!(TX::Resolve(Resolve { client: 1, tx: 1 }).name(), "resolve");
        assert_eq!(
            TX::Chargeback(Chargeback { client: 1, tx: 1 }).name(),
            "chargeback"
        );
    }
}
