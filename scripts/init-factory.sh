#!/bin/bash
set -e

# Load environment variables
if [ -f "ectoplasm-react/.env" ]; then
  export $(grep -v '^#' ectoplasm-react/.env | xargs)
fi

# Configuration
NODE_ADDRESS="${NODE_ADDRESS:-http://65.21.235.122:7777}"
CHAIN_NAME="${CHAIN_NAME:-casper-test}"
SECRET_KEY_PATH="${SECRET_KEY_PATH:-/tmp/casper-keys/secret_key.pem}"
DEPLOYER_ACCOUNT_HASH="${DEPLOYER_ACCOUNT_HASH}"

FACTORY_PACKAGE_HASH="${FACTORY_PACKAGE_HASH}"
PAIR_FACTORY_PACKAGE_HASH="${PAIR_FACTORY_PACKAGE_HASH}"

PAYMENT_AMOUNT="5000000000"  # 5 CSPR
GAS_PRICE_TOLERANCE="5"

echo "=== Initializing Factory Contract ==="
echo "Node: $NODE_ADDRESS"
echo "Chain: $CHAIN_NAME"
echo "Factory Package Hash: $FACTORY_PACKAGE_HASH"
echo "Pair Factory Package Hash: $PAIR_FACTORY_PACKAGE_HASH"
echo "Deployer Account: $DEPLOYER_ACCOUNT_HASH"
echo ""

# Extract just the hash without "hash-" prefix
FACTORY_HASH="${FACTORY_PACKAGE_HASH#hash-}"
PAIR_FACTORY_HASH="${PAIR_FACTORY_PACKAGE_HASH#hash-}"
FEE_TO_SETTER="$DEPLOYER_ACCOUNT_HASH"

echo "Calling Factory.init()..."
echo "  fee_to_setter: $FEE_TO_SETTER"
echo "  pair_factory: Key::Hash($PAIR_FACTORY_HASH)"
echo ""

# Call Factory.init(fee_to_setter, pair_factory)
# Note: Odra contracts need to use package hash, not contract hash
TX_OUTPUT=$(casper-client put-deploy \
  --node-address "$NODE_ADDRESS" \
  --chain-name "$CHAIN_NAME" \
  --secret-key "$SECRET_KEY_PATH" \
  --payment-amount "$PAYMENT_AMOUNT" \
  --session-package-hash "$FACTORY_HASH" \
  --session-entry-point "init" \
  --session-arg "fee_to_setter:key='$FEE_TO_SETTER'" \
  --session-arg "pair_factory:key='hash-$PAIR_FACTORY_HASH'" 2>&1)

# Extract deploy hash
DEPLOY_HASH=$(echo "$TX_OUTPUT" | grep -oP 'deploy_hash:\s*"\K[^"]+' || echo "$TX_OUTPUT" | grep -oP '"result":\{"deploy_hash":"\K[^"]+')

if [ -z "$DEPLOY_HASH" ]; then
  echo "❌ Failed to extract deploy hash from output:"
  echo "$TX_OUTPUT"
  echo ""
  echo "Full casper-client output:"
  echo "$TX_OUTPUT" | head -50
  exit 1
fi

echo "✅ Deploy submitted!"
echo "Deploy hash: $DEPLOY_HASH"
echo ""
echo "Waiting for deploy to complete..."

# Wait for deploy
MAX_WAIT=180
WAIT_INTERVAL=5
ELAPSED=0

while [ $ELAPSED -lt $MAX_WAIT ]; do
  sleep $WAIT_INTERVAL
  ELAPSED=$((ELAPSED + WAIT_INTERVAL))
  
  RESULT=$(casper-client get-deploy \
    --node-address "$NODE_ADDRESS" \
    "$DEPLOY_HASH" 2>/dev/null || echo "")
  
  if echo "$RESULT" | grep -q '"execution_results"'; then
    if echo "$RESULT" | grep -q '"Success"'; then
      echo "✅ Factory initialization SUCCESS!"
      echo ""
      echo "You can now:"
      echo "1. Add liquidity via the Router"
      echo "2. Pairs will be created automatically"
      exit 0
    elif echo "$RESULT" | grep -q '"Failure"'; then
      echo "❌ Factory initialization FAILED!"
      echo "$RESULT" | grep -A 10 "Failure"
      exit 1
    fi
  fi
  
  echo "  Still waiting... (${ELAPSED}s)"
done

echo "⏱️  Timeout waiting for deploy. Check manually:"
echo "casper-client get-deploy --node-address $NODE_ADDRESS $DEPLOY_HASH"
exit 1
