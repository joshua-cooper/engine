## Test

```sh
cargo test
```

## Build / Run

```sh
cargo run transactions.csv
```

## Assumptions

- **Every client present in the input should be tracked.** This means that clients
  present in the input will also be present in the output, even if they had no
  meaningful events. For example if there was a single chargeback with no
  other events.

- **The input is correct.** Input errors usually mean the program aborts with
  an error. For example if an invalid event type is used, or some fields are
  missing from a valid event type. No recovery is attempted. This also means
  that things like global uniqueness of transaction IDs is assumed to be true
  and not checked.

- **Locked accounts can't deposit or withdraw funds.** Although they can still
  raise disputes, resolutions and chargebacks, since that would be out of our
  control.

## Correctness

The finite state machine between valid deposit states is managed via
`DepositHistory`. To access the amount of a past deposit, a valid state
transition must be made, otherwise an error is returned.

By using the API of `DepositHistory`, the logic around `Account` is not
concerned with low level state transitions and becomes trivial. Unit tests in
the `account` module check that methods on `Account` exhibit the correct
behavior. Rust's privacy rules should make it impossible for consumers of
`Account` to put it in an invalid state, and the type system will encourage
them to handle the `Result` in case of errors.

There are also some end-to-end tests in the root of the library crate which
check that input is correctly parsed, accounts are correctly orchestrated
together and output is correctly rendered.

## Efficiency

The input CSV file is streamed as it's processed, which will reduce resource
usage for very large data sets. However, since the deposit history of each
account is kept forever, a deposit heavy workload would eventually grow to
consume lots of resources.
