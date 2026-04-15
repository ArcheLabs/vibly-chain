# Contributing

Thanks for helping improve vibly-chain. Keep changes focused and make the local
checks pass before opening a pull request.

## Local Checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude vibly-chain-node -j1
cargo build --release -p vibly-chain-node
```

For pallet-only work, run the targeted tests first:

```bash
cargo test -p pallet-identity-core
cargo test -p pallet-payment-intent
```

## Pull Requests

- Explain the user-visible behavior change and any runtime/storage impact.
- Keep runtime call indexes, storage names, and encoded types stable unless the
  pull request explicitly includes a migration plan.
- Add or update tests when changing pallet state transitions, authorization, or
  settlement behavior.
- Update README or rustdoc when changing public behavior.

## Runtime Safety

Runtime changes should call out:

- Storage migrations or lack of migration need.
- Weight changes and benchmark expectations.
- Compatibility with existing local and testnet chain specs.
- Any changes to identities, delegated capabilities, holds, or payment intent
  lifecycle rules.
