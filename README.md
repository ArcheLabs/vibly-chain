# vibly-chain

`vibly-chain` is a minimal truth parachain built with the Polkadot SDK. It keeps
identity roots, identity-scoped pointers, external transport bindings, and native
asset payment intents on chain.

## Scope

- `pallet-identity-core`: root identities, recovery accounts, delegated keys,
  active content pointers, and external transport bindings.
- `pallet-payment-intent`: native-asset payment intents for `asset_id = 0`,
  including direct settlement and held-funds settlement.
- `primitives/*`: shared SCALE types used by the runtime and custom pallets.
- `node/`: the `vibly-chain-node` collator binary.
- `runtime/`: parachain runtime wiring for the custom pallets and standard
  FRAME/Cumulus pallets.

Current non-goals:

- No custom RPC layer.
- No action registry pallet.
- No agent runtime.
- No Matrix, Discord, Telegram, or email integration logic beyond transport
  binding records.
- No automated governance, registrar access, or sudo-based upgrades.

## Prerequisites

- Rust toolchain from `rust-toolchain.toml`.
- `wasm32-unknown-unknown` target, installed by rustup from the toolchain file.
- Zombienet CLI for local multi-node smoke tests:

```bash
npm install -g @zombienet/cli
zombienet setup polkadot
```

If `polkadot` is not on `PATH`, add the directory used by `zombienet setup
polkadot` to your shell `PATH`.

## Common Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude vibly-chain-node -j1
cargo build --release -p vibly-chain-node
```

## Local Network

Build the node and start a local relay chain with two collators:

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```

Generate a plain development chain spec:

```bash
./scripts/dev/chain-spec.sh
```

The Zombienet config uses the native provider and expects the release binary at
`./target/release/vibly-chain-node`.

## Paseo Artifacts

Build the release binary and export the genesis state/wasm for Paseo-style
registration:

```bash
./scripts/paseo/build-artifacts.sh
```

Use `scripts/paseo/collator.sh` as a launch example after the parachain is
registered. Registration and upgrade notes live in `scripts/paseo/README.md`.

## Repository Map

- `node/`: collator CLI, chain spec, RPC, and service wiring.
- `runtime/`: runtime configuration, genesis presets, APIs, benchmarks, weights.
- `pallets/identity-core/`: identity and transport-binding state machine.
- `pallets/payment-intent/`: payment-intent state machine.
- `primitives/common/`: shared base types.
- `primitives/identity/`: identity data model and authorization trait.
- `primitives/payment/`: payment-intent data model.
- `integration-tests/zombienet/`: local network smoke-test configuration.
- `scripts/dev/`: local build and chain-spec helpers.
- `scripts/paseo/`: testnet artifact and collator helpers.

## Contributing

See `CONTRIBUTING.md` for local checks and contribution expectations. Report
security issues using `SECURITY.md`.
