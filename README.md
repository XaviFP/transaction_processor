# Transaction Processor

## Overview

This project is a toy payments engine that processes transactions from a CSV file, updates client accounts accordingly, handles disputes and chargebacks, and outputs the final state of the accounts.


### Input

The input CSV file contains the following columns:
```
type: The type of transaction (deposit, withdrawal, dispute, resolve, chargeback).

client: The client ID (u16).

tx: The transaction ID (u32).

amount:  The transaction amount (f64, only for Deposit and Withdrawal transaction types).
```
Example:

csv
```
type,client,tx,amount
deposit,1,1,1.0
deposit,2,2,2.0
withdrawal,1,3,0.5
dispute,1,1,
resolve,1,1,
chargeback,2,2,
```

### Output

The output CSV should list the client account states with the following columns:

    client: Client ID.
    available: Available funds.
    held: Held funds.
    total: Total funds (available + held).
    locked: Whether the account is locked.

Example:

```csv
client,available,held,total,locked
1,0.5,0.0,0.5,false
2,0.0,0.0,0.0,true
```

### Project Details
#### Transactions

There are five types of transactions:

Deposit: Increases the available and total funds of the client account.

Withdrawal: Decreases the available and total funds of the client account if sufficient funds are available.

Dispute: Puts a transaction under dispute, moving the disputed amount from available to held funds.

Resolve: Resolves a dispute, moving the disputed amount back from held to available funds.

Chargeback: Finalizes a dispute by deducting the disputed amount from the total and held funds and locking the account.


## Usage

To run the payments engine, use the following command:

```sh
$ cargo run -- transactions.csv > accounts.csv
```
transactions.csv is the input file containing a series of transactions.

The output, which contains the state of client accounts, will be written to stdout.

All errors ocurred while processing the transactions will be written to stderr.
## Running Tests

The project includes unit tests for most of the functionalities.

To run the tests, use:

```sh
$ cargo test
```

## Building

To build the project run
```sh
$ cargo build
```
