# Payments Engine Challenge

This project includes a toy implementation of a payments engine to demonstrate various Rust capabilities.

## Running the implementation

```sh
cargo run -- transactions.csv > accounts.csv
```

## Running the tests

There are unit tests for each functional unit and a load test for observing performance with a large input. The load test helps to compare design choices with rough measurements.

### Unit tests

```sh
cargo test
```

### Load test

```sh
cargo test
```

## Considerations

The following are the considerations I made when writing the solution.

### Balance Representation

For representing the balances I was torn between using an integer or decimal type. The integer type would contain the balance and 4 decimal places as a scaled integer, e.g 25 dollars would be represented as 250000. The decimal type would represent the balance as-is using a limited precision decimal type such as `rust_decimal::Decimal`.

The integer type would avoid difficulties around stored precision and arithmetic rounding associated with binary floating point types but could be prone to programmer error if the inherent scale is forgotten, possibly leading to severe monetary loss for the company. The limited precision decimal type should reduce programmer error because the scale is intrinsic, and more rounding strategies are possible. The precision of the decimal type would technically not be exactly 4 decimal places, merely a minimum of 4 decimal places, but this only extends the guarantee of precision.

I decided to choose the limited precision decimal type `rust_decimal::Decimal`.

I also choose to use checked arithmetic so that overflows could be detected and reported as an error.

### Negative balance

### Record Processing

I am assuming that duplicate transaction processing must be avoided. If a transaction is duplicated in the input it must only be processed once.

Given transactions are guaranteed to occur chronologically in the file this aids us in preventing duplicate processing of the same transaction. We can process transactions in the same order as the input while keeping track of the last transaction ID that we processed. Then, if we encounter a lower or equivalent transaction ID in the input we can emit an error because it must be out of order or duplicated. This not only validates the assertion that the transactions are provided chronologically but prevents us from processing the same transaction twice. If we encounter the same transaction subsequently we will not risk processing it twice because it will violate the chronological ordering rule too.

<!-- To provide further safety against duplicate processing I've prefered to use apply functions which consume the transaction type by passing ownership to the apply functions (see `domain/client_account.rs`). Then if the next programmer accidentally tries to use the transaction after it has already been applied they are presented with a reminder that it has been used. -->

### External Library Usage

Generally I try to stick to the std library wherever possible to keep my codebases looking somewhat consistent and to limit supply chain attacks. However, one exception is the anyhow crate which provides (subjectively) more ergonomic error types.

### Prefer Type Safety

I prefer using the type system over runtime validation to guarantee safety. E.g I use enums to represent different transaction types.

### Disputes

I've assumed that only deposits can be disputed, not withdrawals.

I've assumed that the same deposit can be disputed multiple times, as long is it resolved between each dispute.

### Large inputs

- Maximum number of transactions
- Maximum number of clients
- Run time
