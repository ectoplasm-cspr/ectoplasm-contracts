#!/bin/bash
# Check Casper Testnet Balance

set -e

KEYS_DIR="$HOME/casper-keys"
NODE_ADDRESS="http://65.21.235.122:7777"
CHAIN_NAME="casper-test"

if [ ! -f "$KEYS_DIR/public_key_hex" ]; then
    echo "❌ No wallet found. Run ./scripts/setup-wallet.sh first"
    exit 1
fi

ACCOUNT_HASH=$(cat "$KEYS_DIR/public_key_hex")

echo "🔍 Checking balance for account:"
echo "   $ACCOUNT_HASH"
echo ""

# Get account balance
echo "📊 Querying balance..."
RESULT=$(casper-client query-balance \
    --node-address "$NODE_ADDRESS" \
    --purse-identifier "$KEYS_DIR/public_key.pem" 2>&1)

if echo "$RESULT" | grep -q "error"; then
    echo "⚠️  Account not found or not yet funded"
    echo "$RESULT"
    echo ""
    echo "💡 Make sure you've requested testnet funds from:"
    echo "   https://testnet.cspr.live/tools/faucet"
else
    echo "$RESULT"
    echo ""
    echo "✅ Account is funded!"
    echo "💡 1 CSPR = 1,000,000,000 motes (9 decimals)"
fi
