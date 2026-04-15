# Node

The node crate builds the `vibly-chain-node` collator binary. It handles CLI
parsing, chain-spec loading, RPC setup, consensus service wiring, and relay-chain
argument forwarding.

Important areas:

- `src/cli.rs`: command-line interface and examples.
- `src/chain_spec.rs`: development and local testnet chain specifications.
- `src/command.rs`: subcommand execution and chain-spec loading.
- `src/service.rs`: collator service construction.
- `src/rpc.rs`: RPC module setup.

Common commands:

```bash
cargo build --release -p vibly-chain-node
./target/release/vibly-chain-node --help
./target/release/vibly-chain-node --chain local --collator -- --chain rococo-local
```
