#!/bin/bash
# Deploy Incentive System Contracts to Casper Testnet

set -e

KEYS_DIR="$HOME/casper-keys"
NODE_ADDRESS="http://65.21.235.122:7777"
CHAIN_NAME="casper-test"
GAS_PRICE=1
PAYMENT_AMOUNT=200000000000  # 200 CSPR

echo "🚀 Deploying Incentive System Contracts to Casper Testnet"
echo "=========================================================="
echo ""

# Check if keys exist
if [ ! -f "$KEYS_DIR/secret_key.pem" ]; then
    echo "❌ No wallet found. Run ./scripts/setup-wallet.sh first"
    exit 1
fi

echo "📊 Checking balance..."
BALANCE=$(casper-client query-balance \
    --node-address "$NODE_ADDRESS" \
    --purse-identifier "$KEYS_DIR/public_key.pem" 2>&1 | grep -o '"balance": "[0-9]*"' | grep -o '[0-9]*')

if [ -z "$BALANCE" ] || [ "$BALANCE" -lt "500000000000" ]; then
    echo "❌ Insufficient balance. Need at least 500 CSPR for deployment"
    exit 1
fi

echo "✅ Balance: $((BALANCE / 1000000000)) CSPR"
echo ""

# Deploy GasDiscountManager
echo "1️⃣  Deploying GasDiscountManager..."
DEPLOY_RESULT_1=$(casper-client put-deploy \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$KEYS_DIR/secret_key.pem" \
    --payment-amount "$PAYMENT_AMOUNT" \
    --session-path "wasm/GasDiscountManager.wasm" 2>&1)

DEPLOY_HASH_1=$(echo "$DEPLOY_RESULT_1" | grep -o '"deploy_hash": "[a-f0-9]*"' | grep -o '[a-f0-9]\{64\}')

echo "   Deploy hash: $DEPLOY_HASH_1"
echo "   Waiting for deployment..."
sleep 20

# Get deploy info
casper-client get-deploy \
    --node-address "$NODE_ADDRESS" \
    "$DEPLOY_HASH_1" > /tmp/gas_discount_deploy.json 2>&1

echo "   ✅ GasDiscountManager deployed"
echo ""

# Deploy LpRewardsDistributor
echo "2️⃣  Deploying LpRewardsDistributor..."
DEPLOY_RESULT_2=$(casper-client put-deploy \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$KEYS_DIR/secret_key.pem" \
    --payment-amount "$PAYMENT_AMOUNT" \
    --session-path "wasm/LpRewardsDistributor.wasm" 2>&1)

DEPLOY_HASH_2=$(echo "$DEPLOY_RESULT_2" | grep -o '"deploy_hash": "[a-f0-9]*"' | grep -o '[a-f0-9]\{64\}')

echo "   Deploy hash: $DEPLOY_HASH_2"
echo "   Waiting for deployment..."
sleep 20

casper-client get-deploy \
    --node-address "$NODE_ADDRESS" \
    "$DEPLOY_HASH_2" > /tmp/lp_rewards_deploy.json 2>&1

echo "   ✅ LpRewardsDistributor deployed"
echo ""

# Deploy IncentiveManager
echo "3️⃣  Deploying IncentiveManager..."
DEPLOY_RESULT_3=$(casper-client put-deploy \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$KEYS_DIR/secret_key.pem" \
    --payment-amount "$PAYMENT_AMOUNT" \
    --session-path "wasm/IncentiveManager.wasm" 2>&1)

DEPLOY_HASH_3=$(echo "$DEPLOY_RESULT_3" | grep -o '"deploy_hash": "[a-f0-9]*"' | grep -o '[a-f0-9]\{64\}')

echo "   Deploy hash: $DEPLOY_HASH_3"
echo "   Waiting for deployment..."
sleep 20

casper-client get-deploy \
    --node-address "$NODE_ADDRESS" \
    "$DEPLOY_HASH_3" > /tmp/incentive_manager_deploy.json 2>&1

echo "   ✅ IncentiveManager deployed"
echo ""

echo "🎉 All Incentive Contracts Deployed!"
echo "===================================="
echo ""
echo "📝 Deployment Summary:"
echo "   GasDiscountManager:    $DEPLOY_HASH_1"
echo "   LpRewardsDistributor:  $DEPLOY_HASH_2"
echo "   IncentiveManager:      $DEPLOY_HASH_3"
echo ""
echo "🔗 View on Explorer:"
echo "   https://testnet.cspr.live/deploy/$DEPLOY_HASH_1"
echo "   https://testnet.cspr.live/deploy/$DEPLOY_HASH_2"
echo "   https://testnet.cspr.live/deploy/$DEPLOY_HASH_3"
echo ""
echo "💡 Next Steps:"
echo "   1. Wait for deployments to finalize (~2 minutes)"
echo "   2. Extract contract hashes from deploy info"
echo "   3. Initialize contracts with proper addresses"
echo "   4. Test the incentive system"
