# Runtime

The runtime is the vibly-chain state transition function. It wires the custom
identity and payment pallets into the Polkadot SDK parachain runtime baseline and
exposes the standard runtime APIs used by the collator node.

Important areas:

- `src/lib.rs`: runtime type definitions and pallet composition.
- `src/configs/`: pallet configuration and XCM-related configuration.
- `src/genesis_config_presets.rs`: development and local testnet genesis presets.
- `src/benchmarks.rs`: runtime benchmark registration.
- `src/weights/`: generated and static weight configuration.

Build the runtime through the node package:

```bash
cargo build --release -p vibly-chain-node
```
