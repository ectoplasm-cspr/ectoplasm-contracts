#!/bin/bash
set -e

NODE_ADDRESS="http://localhost:11101"
# Get the router package hash from the deploy output env or query it
# We'll expect the caller to provide DEPLOYER_ACCOUNT_HASH
DEPLOYER="$1"

if [ -z "$DEPLOYER" ]; then
    echo "Usage: $0 <DEPLOYER_ACCOUNT_HASH>"
    exit 1
fi

echo "Verifying Router initialization for deployer: $DEPLOYER"

STATE_ROOT=$(casper-client get-state-root-hash --node-address "$NODE_ADDRESS" | jq -r '.result.state_root_hash')
echo "State Root: $STATE_ROOT"

# Get Router Package
ROUTER_PKG=$(casper-client query-global-state --node-address "$NODE_ADDRESS" --state-root-hash "$STATE_ROOT" --key "$DEPLOYER" | jq -r '.result.stored_value.Account.named_keys[] | select(.name == "router_package_hash") | .key')

if [ -z "$ROUTER_PKG" ] || [ "$ROUTER_PKG" == "null" ]; then
    echo "❌ Router package not found in account named keys!"
    exit 1
fi
echo "✅ Found Router Package: $ROUTER_PKG"

# Get Router Contract
ROUTER_CONTRACT=$(casper-client query-global-state --node-address "$NODE_ADDRESS" --state-root-hash "$STATE_ROOT" --key "$ROUTER_PKG" | jq -r '.result.stored_value.ContractPackage.current_contract_hash')
echo "✅ Found Router Contract: $ROUTER_CONTRACT"

# Query Named Keys
NAMED_KEYS=$(casper-client query-global-state --node-address "$NODE_ADDRESS" --state-root-hash "$STATE_ROOT" --key "$ROUTER_CONTRACT" | jq '.result.stored_value.Contract.named_keys')

FACTORY_KEY=$(echo "$NAMED_KEYS" | jq -r '.[] | select(.name == "factory") | .key')
WCSPR_KEY=$(echo "$NAMED_KEYS" | jq -r '.[] | select(.name == "wcspr") | .key')

echo "----------------------------------------"
if [ -n "$FACTORY_KEY" ] && [ "$FACTORY_KEY" != "null" ]; then
    echo "✅ Router.factory is set: $FACTORY_KEY"
else
    echo "❌ Router.factory is NOT set!"
fi

if [ -n "$WCSPR_KEY" ] && [ "$WCSPR_KEY" != "null" ]; then
    echo "✅ Router.wcspr is set: $WCSPR_KEY"
else
    echo "❌ Router.wcspr is NOT set!"
fi
echo "----------------------------------------"
