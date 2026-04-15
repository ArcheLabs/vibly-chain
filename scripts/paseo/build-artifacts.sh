#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p vibly-chain-node
./target/release/vibly-chain-node export-genesis-state > paseo-genesis-state
./target/release/vibly-chain-node export-genesis-wasm > paseo-genesis-wasm
