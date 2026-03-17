# Zombienet

`local.toml` starts a local relay chain with two `vibly-chain` collators. It is intentionally small and only targets smoke coverage for the identity and payment pallets.

Typical flow:

```bash
./scripts/dev/build.sh
./scripts/dev/zombienet-local.sh
```
