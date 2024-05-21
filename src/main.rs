use clap::{Arg, Command};
use csv::Writer;
use serde::{Serialize, Serializer};
use std::io;

mod process_transaction;
mod transactions;
mod types;

use process_transaction::*;
use types::*;

fn main() {
    let matches = Command::new("transaction_processor")
        .version("1.0")
        .author("Your Name <xavi@delape.net>")
        .about("Processes transactions and generates account balances")
        .arg(
            Arg::new("input")
                .help("Sets the input CSV file to use")
                .required(true)
                .index(1),
        )
        .get_matches();
    let input_path = matches.get_one::<String>("input").unwrap();

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(input_path)
        .unwrap();
    let mut accounts = Accounts::new();
    let mut transactions = Transactions::new();

    for record in rdr.deserialize() {
        let tx: Transaction = match record {
            Ok(tx) => tx,
            Err(err) => {
                eprintln!("Failed to deserialize transaction: {}", err);
                continue;
            }
        };

        let transaction = match TX::from_transaction(tx) {
            Ok(transaction) => transaction,
            Err(err) => {
                eprintln!("Failed to parse transaction: {}", err);
                continue;
            }
        };

        match process_transaction(transaction, &mut accounts, &mut transactions) {
            Ok(_) => (),
            Err(err) => eprintln!("{}", err),
        }
    }

    write_accounts(&accounts, io::stdout())
}

fn write_accounts(accounts: &Accounts, wtr: impl io::Write) {
    let mut writer = Writer::from_writer(wtr);
    let mut acc: OutputAccount;
    for (client, account) in accounts {
        acc = OutputAccount::new(client, account);
        match writer.serialize(acc) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to write account: {}", err),
        }
    }
    writer.flush().unwrap();
}

#[derive(Debug, Serialize)]
struct OutputAccount {
    client: u16,
    #[serde(serialize_with = "truncate_serialize")]
    available: f64,
    #[serde(serialize_with = "truncate_serialize")]
    held: f64,
    #[serde(serialize_with = "truncate_serialize")]
    total: f64,
    locked: bool,
}

impl OutputAccount {
    fn new(client: &u16, account: &Account) -> Self {
        Self {
            client: *client,
            available: account.available,
            held: account.held,
            total: account.total,
            locked: account.locked,
        }
    }
}

fn truncate_serialize<S>(x: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f64(truncate(*x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_accounts() {
        let mut accounts = Accounts::new();
        accounts.insert(
            1,
            Account {
                available: 1.0,
                held: 0.0,
                total: 1.0,
                locked: false,
            },
        );
        accounts.insert(
            2,
            Account {
                available: 2.0,
                held: 0.0,
                total: 2.0,
                locked: false,
            },
        );

        let mut buf = Vec::new();
        write_accounts(&accounts, &mut buf);

        let expected1 = "\
client,available,held,total,locked\n\
1,1.0,0.0,1.0,false\n\
2,2.0,0.0,2.0,false\n\
";
        let expected2 = "\
client,available,held,total,locked\n\
2,2.0,0.0,2.0,false\n\
1,1.0,0.0,1.0,false\n\
";
        let expected = if buf == expected1.as_bytes() {
            expected1
        } else {
            expected2
        };
        assert_eq!(String::from_utf8(buf).unwrap(), expected);
    }
}
