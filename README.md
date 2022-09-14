# Payments Engine Challenge

This project includes a toy implementation of a payments engine.

## Running the implementation

```sh
cargo run -- transactions.csv > accounts.csv
```

## Running the tests

### Unit tests

There are unit tests for each functional unit.

```sh
cargo test
```

### Large file test

There is also a large file test for observing performance with a large input (13GB, 1 billion events). To run, comment the `#[ignore]` attribute on the `test_large_file()` test. Then run the tests.

```sh
cargo test
```

## Considerations

The following are the considerations I made when writing the solution.

### Negative Balance

I have assumed that a negative available balance is acceptable for the purposes of allowing a dispute to be applied. To be clear though, a withdrawal is not able to result in a negative available balance.

### Disputes

Disputes are possible on 'transactions' which I've understood to be either deposits or withdrawals but not disputes, resolves or chargebacks because they merely refer to transactions rather than being transactions themselves.

Additionally I've assumed that only deposits can be disputed, not withdrawals.

I've assumed that the same deposit can be disputed multiple times, as long is it resolved between each dispute.

### Duplicate Transactions

Given there are requirements for disputes, resolves and chargebacks to be idempotent I am assuming deposits and withdrawals must be idempotent too. If a deposit or withdrawal is present in the input twice the system ignores all but the first instance of each.

Idempotency such as this helps in a distributed system where retries are necessary in cases of undetermined delivery.

### Ownership

To provide further safety against accidental duplicate processing due to programmer error I've prefered to use apply functions which consume the transaction type by passing ownership to the apply functions (see `src/domain/client_account.rs`). This helps to avoid a transaction being used twice because the borrow checker would inform the programmer.

### Atomic Operations

Given the system operates in the payments domain, the integrity of the state is paramount. Corruption could be catastrophic to the business and/or the clients. I have intentionally ensured all domain state operations are atomic and consistent. If the operation is valid all relevant state is changed. If there is an error condition the state is left entirely unaffected.

### Concurrency

For the purposes of this exercise I have assumed operations are not concurrent. The current implementation is not thread safe.

If this system is presented as a web service (for example) concurrency protections (e.g. mutex, optimistic locking) would need to be introduced.

### Balance Representation

For representing the balances I was torn between using an integer or decimal type. The integer type would contain the balance and 4 decimal places as a scaled integer, e.g 25 dollars would be represented as 250000. The decimal type would represent the balance as-is using a limited precision decimal type such as `rust_decimal::Decimal`.

The integer type would avoid difficulties around stored precision and arithmetic rounding associated with binary floating point types but could be prone to programmer error if the inherent scale is forgotten, possibly leading to severe monetary loss for the company (I've see this happen before!). The limited precision decimal type should reduce programmer error because the scale is intrinsic, and more rounding strategies are possible. The precision of the decimal type would technically not be exactly 4 decimal places, merely a minimum of 4 decimal places, but this only extends the guarantee of precision.

I decided to choose the limited precision decimal type `rust_decimal::Decimal`.

I also choose to use checked arithmetic where appropriate so that overflows and underflows could be detected and reported as an error, even when running the code as a release build with debug protections.

### Identifiers for 'Events'

One improvement to the data model could be to include identifiers for the operations. The transaction ID does not uniquely identify an operation because some operations refer to transactions rather than define them. There could also be an 'event ID' to uniquely identify each operation.

One example is disputing a deposit twice (if resolved after the first time) for example. An 'event ID' to could be added to uniquely identify each dispute. This could also be used to help with delivery, enforcing ordering and help de-duplicate operations.

### Resource Usage

I've used the CSV reader as an iterator over the CSV file. It will stream the file off disk and free the memory of each record once it has been processed, so there is no need to load the whole file into memory.

The large file test (see above) demonstrates the measured resource usage of this system. The test generates a 13 GB file containing 1 billion events (I'm using the word transaction to mean deposits and withdrawals only, and event to mean all five types). It takes about 5 minutes to run and uses 8GB of memory at it's peak, likely due to the filling `HashSets` of processed transaction IDs and deposit state. It would be even worse with a higher cardinality of client IDs. This could be offloaded to a database and the memory usage would be much lower.

If this was used in a high scale server with many TCP connections I would allow incoming data to build up in fixed sized buffers for each connection, then consume with various threads with queues. To achieve high thread utilisation I would evaluate using async I/O to notify when new data has arrived rather than having threads sleeping or spinning.

### Error Handling

Rather than panicing I've relied upon `Result` passing with useful detail for debugging the issue.

I would handle error results differently depending upon the deployment of the system:

- In a web service the result could be returned as an HTTP response with status 4xx or 5xx depending upon the error.
- In a job the process could log the error, fail, and retry with backoffs.
- In an event driven distributed system this error could be emitted as an 'error event' and published to a queue which the original submitter could listen to.

### Prefer Type Safety

I prefer using the type system over runtime validation to guarantee safety, e.g I use enums to represent different transaction types. There is extra code for deserializing using Serde because deserializing to enums [is difficult](https://stackoverflow.com/questions/69417454/serialize-deserialize-csv-with-nested-enum-struct-with-serde-in-rust) in a terse way.

### External Library Usage

Generally I try to stick to the std library wherever possible to keep my codebases looking somewhat consistent and to limit supply chain attacks. However, one example exception I usually make is the anyhow crate which provides (subjectively) more ergonomic error types.

### Tests

My testing strategy was to spend most effort on the critical domain code so most tests are around the client account application functions.
