#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p parachain-template-node
./target/release/parachain-template-node export-genesis-state > paseo-genesis-state
./target/release/parachain-template-node export-genesis-wasm > paseo-genesis-wasm
