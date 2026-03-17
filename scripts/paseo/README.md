# Paseo Notes

1. Build artifacts with `./scripts/paseo/build-artifacts.sh`.
2. Register the parachain on Paseo using the exported genesis state and wasm.
3. Launch one or more collators with `./scripts/paseo/collator.sh --name <name>`.
4. Use standard metadata, storage, and events to inspect `identity-core` and `payment-intent` state.

This repository does not automate governance, registrar access, or sudo-based upgrades.
