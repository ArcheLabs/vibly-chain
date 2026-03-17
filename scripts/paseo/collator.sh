#!/usr/bin/env bash
set -euo pipefail

CHAIN_SPEC=${CHAIN_SPEC:-local}
./target/release/parachain-template-node \
  --chain "$CHAIN_SPEC" \
  --collator \
  --base-path ./tmp/paseo-collator \
  "$@"
