#!/bin/bash

# Kill any existing validator
pkill -f solana-test-validator || true
sleep 2

# Start fresh validator
solana-test-validator --reset &
VALIDATOR_PID=$!

# Wait for validator to be ready
sleep 5

# Set environment for tests
export ANCHOR_PROVIDER_URL="http://localhost:8899"
export ANCHOR_WALLET="$HOME/.config/solana/id.json"

# Build and deploy program
anchor build
anchor deploy

# Run tests
yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts

# Cleanup
kill $VALIDATOR_PID || true