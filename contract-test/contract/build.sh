#!/bin/bash
set -e

cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/oysterpack_near_stake_token_test.wasm ./res/