# vibly-chain

`vibly-chain` is the Vibly coordination chain built on the [Polkadot SDK](https://github.com/paritytech/polkadot-sdk). It provides two runtime configurations:

| Configuration | Binary | Default RPC |
|---|---|---|
| **Parachain** (`node/` + `runtime/`) | `vibly-chain-node` | 9988 |
| **Solo-node** (`solo-node/` + `solo-runtime/`) | `vibly-solo-node` | **9944** |

The solo-node is the primary target for local development and E2E testing with `vibly-coordinator` and `vibly-indexer`.

## Pallets

### Shared (parachain + solo-node)

| Pallet | Description |
|---|---|
| `pallet-identity-core` | Root identity, recovery accounts, delegate keys, content pointers, external transport bindings |
| `pallet-payment-intent` | Native-asset (asset\_id=0) payment intents with direct and hold-based settlement |

### Solo-node only

| Pallet | Description |
|---|---|
| `pallet-onboarding-distribution` | Agent registration, registrar assignment |
| `pallet-agent-staking` | Agent stake bonding, unbonding, release-block mechanism |
| `pallet-membership` (GuardianMembership) | Guardian member management; a single Guardian member can pause a proposal |
| `pallet-collective` (GuardianCollective) | Guardian collective; 2/3 majority can cancel or restore a pause |
| `pallet-vibly-emergency` | Emergency pause / resume / cancel interface for Guardian member or collective origins |

> OpenGov (`pallet_referenda`, `ConvictionVoting`, Treasury) is not included in the solo runtime. Vibly's proposal/voting/review flows are modelled as coordinator-side domain events; only final payment/penalty/pause facts are recorded on-chain.

## Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- `wasm32-unknown-unknown` target (installed automatically by rustup)
- [Zombienet CLI](https://github.com/paritytech/zombienet) for multi-node tests: `npm install -g @zombienet/cli && zombienet setup polkadot`

## Build

```bash
# Format check and lint
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# Unit tests
cargo test --workspace --exclude vibly-chain-node -j1

# Build parachain collator
cargo build --release -p vibly-chain-node

# Build solo-node (local development and E2E)
cargo build --release -p vibly-solo-node
```

## Solo-node: local development

```bash
cargo build --release -p vibly-solo-node
./target/release/vibly-solo-node --dev --tmp
# WebSocket: ws://127.0.0.1:9944
# Polkadot.js Apps: https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944
```

With external RPC access (required for Docker-based indexer):

```bash
./target/release/vibly-solo-node --dev --tmp --rpc-external --rpc-cors all
```

Pair with the coordinator:

```bash
cd ../vibly-coordinator && pnpm dev
```

## Parachain: local network (Zombienet)

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```

Generate a dev chain spec:

```bash
./scripts/dev/chain-spec.sh
```

## Paseo testnet deployment

```bash
./scripts/paseo/build-artifacts.sh
# see scripts/paseo/README.md for upload and collator setup
```

## Repository structure

| Directory | Contents |
|---|---|
| `node/` | Parachain collator CLI, chain spec, RPC, services |
| `runtime/` | Parachain runtime (identity + payment) |
| `solo-node/` | Solo-node CLI |
| `solo-runtime/` | Solo runtime (identity + payment + agent staking + Guardian emergency) |
| `pallets/identity-core/` | Identity state machine pallet |
| `pallets/payment-intent/` | Payment intent state machine pallet |
| `pallets/agent-staking/` | Agent stake bonding and release-block pallet |
| `pallets/onboarding-distribution/` | Agent registration and registrar assignment pallet |
| `primitives/` | Shared SCALE types |
| `integration-tests/` | Zombienet local network tests |
| `scripts/dev/` | Local build and chain spec tooling |
| `scripts/paseo/` | Testnet artefacts and collator tooling |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Report security issues via [SECURITY.md](SECURITY.md).
