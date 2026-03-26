# Zombienet

`local.toml` starts a local relay chain with two `vibly-chain` collators. It is intentionally small and only targets smoke coverage for the identity and payment pallets.

Prerequisites:

```bash
rustup target add wasm32-unknown-unknown
npm install -g @zombienet/cli
zombienet setup polkadot
```

Verify the required binaries are available:

```bash
zombienet version
polkadot --version
```

Typical flow:

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```

The launcher uses the native provider and resolves paths from the repository root, so it can be run from any working directory.
