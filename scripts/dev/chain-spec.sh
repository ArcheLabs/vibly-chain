#!/usr/bin/env bash
set -euo pipefail

./target/release/vibly-chain-node build-spec --disable-default-bootnode > dev_chain_spec.json
