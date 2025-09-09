#!/bin/bash

echo "Cleaning test environment..."

# Kill any existing validators
pkill -f solana-test-validator 2>/dev/null || true

# Remove ledger data  
rm -rf test-ledger .anchor/test-ledger 2>/dev/null || true

# Wait a moment
sleep 2

echo "Running tests with clean state..."
anchor test

echo "Test run complete."