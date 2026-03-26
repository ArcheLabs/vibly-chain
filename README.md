# vibly-chain

A minimal truth parachain built from the Polkadot SDK parachain template baseline (`ab8a5f2c78653ffcc0b5b1a0e752a6ee49087b6a`).

## Scope

- `pallet-identity-core`: on-chain root identity, authorized keys, active pointers, transport bindings.
- `pallet-payment-intent`: on-chain payment intents for native asset `asset_id = 0` using balances transfer/hold.
- No custom RPC, no action registry, no agent runtime, no Matrix integration.

## Workspace

- `node/`
- `runtime/`
- `primitives/common`
- `primitives/identity`
- `primitives/payment`
- `pallets/identity-core`
- `pallets/payment-intent`
- `integration-tests/zombienet`
- `scripts/dev`
- `scripts/paseo`

## Common Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p parachain-template-node
```

## Dev Network

Use `scripts/dev/build.sh` and `scripts/dev/zombienet-local.sh` for a local relay chain + two collator setup.

Prerequisites:

- Rust target: `rustup target add wasm32-unknown-unknown`
- Zombienet CLI: `npm install -g @zombienet/cli`
- Relay chain binaries: `zombienet setup polkadot`

Recommended verification:

```bash
zombienet version
polkadot --version
```

Typical flow:

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```

Notes:

- `scripts/dev/zombienet-local.sh` uses the native provider.
- The script changes into the repository root before spawning the network, so it can be run from any working directory.
- If `polkadot` is not on `PATH`, add the directory used by `zombienet setup polkadot` to your shell `PATH`.

## Paseo

Use `scripts/paseo/build-artifacts.sh` to build release artifacts and `scripts/paseo/collator.sh` as a collator launch template. Registration and upgrade notes live in `scripts/paseo/README.md`.
