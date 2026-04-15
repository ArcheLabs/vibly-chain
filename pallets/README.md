# Pallets

This directory contains the custom vibly-chain FRAME pallets.

## `identity-core`

Stores root identities, owner and recovery accounts, delegated authorization keys,
active content pointers, and external transport bindings. Other pallets use its
`IdentityAccess` implementation instead of reading identity storage directly.

## `payment-intent`

Stores native-asset payment intents between identity-backed actors. It supports
direct settlement and hold-based settlement for `asset_id = 0`.

## Checks

```bash
cargo test -p pallet-identity-core
cargo test -p pallet-payment-intent
```
