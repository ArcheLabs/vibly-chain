#!/usr/bin/env bash
set -euo pipefail

CHAIN_SPEC=${CHAIN_SPEC:-local}
./target/release/vibly-chain-node \
  --chain "$CHAIN_SPEC" \
  --collator \
  --base-path ./tmp/paseo-collator \
  "$@"
