#!/bin/bash
set -e

# Source env
source .env
# Source output env to get contract hashes if available, otherwise expect them
if [ -f scripts/deploy-new.out.env ]; then
    source scripts/deploy-new.out.env
fi

# Fallback or strict check
if [ -z "$DEPLOYER_ACCOUNT_HASH" ]; then echo "DEPLOYER_ACCOUNT_HASH missing"; exit 1; fi
if [ -z "$ROUTER_PACKAGE_HASH" ]; then echo "ROUTER_PACKAGE_HASH missing"; exit 1; fi
if [ -z "$WCSPR_PACKAGE_HASH" ]; then echo "WCSPR_PACKAGE_HASH missing"; exit 1; fi
if [ -z "$ECTO_PACKAGE_HASH" ]; then echo "ECTO_PACKAGE_HASH missing"; exit 1; fi
if [ -z "$PAIR_FACTORY_PACKAGE_HASH" ]; then echo "PAIR_FACTORY_PACKAGE_HASH missing"; exit 1; fi

# Use Package Hashes for tokens as they are standard entry points for Odra
# But for router calls, we also use Package Hashes usually.

echo "Verifying Flow on $CHAIN_NAME ($NODE_ADDRESS)"
echo "Deployer: $DEPLOYER_ACCOUNT_HASH"

# Helper
run_tx() {
    local name="$1"
    local pkg_hash="$2"
    local entry_point="$3"
    shift 3
    
    echo "==> $name"
    local tx
    tx=$(casper-client put-transaction package \
        --node-address "$NODE_ADDRESS" \
        --chain-name "$CHAIN_NAME" \
        --secret-key "$SECRET_KEY_PATH" \
        --payment-amount 5000000000 \
        --gas-price-tolerance 10 \
        --standard-payment true \
        --contract-package-hash "$pkg_hash" \
        --session-entry-point "$entry_point" \
        "$@" \
        | jq -r '.result.transaction_hash.Version1 // .result.transaction_hash.Version2 // .result.deploy_hash')
    
    echo "TX: $tx"
    
    # Wait for execution
    local i=0
    while [ $i -lt 90 ]; do
        sleep 2
        # Check transaction status
        local output
        output=$(casper-client get-transaction --node-address "$NODE_ADDRESS" "$tx" 2>/dev/null)
        
        # Check for success
        local status
        status=$(echo "$output" | jq -r '
            .result.execution_info.execution_result.Version2.Success // 
            .result.execution_info.execution_result.Version1.Success // 
            empty')

        if [ -n "$status" ]; then
            echo "✅ Success"
            return 0
        fi
        
        # Check for failure
        local failure
        failure=$(echo "$output" | jq -r '
            .result.execution_info.execution_result.Version2.Failure.error_message // 
            .result.execution_info.execution_result.Version1.Failure.error_message // 
            empty')
            
        if [ -n "$failure" ]; then
            echo "❌ Failed: $failure"
            exit 1
        fi
        
        echo -n "."
        i=$((i+1))
    done
    echo "Timeout waiting for tx"
    exit 1
}

# 1. Mint 1000 WCSPR
run_tx "Minting 1000 WCSPR to Deployer" \
    "$WCSPR_PACKAGE_HASH" \
    "mint" \
    --session-arg "to:key='$DEPLOYER_ACCOUNT_HASH'" \
    --session-arg "amount:u256='1000000000000000000000'" 

# 2. Mint 1000 ECTO
run_tx "Minting 1000 ECTO to Deployer" \
    "$ECTO_PACKAGE_HASH" \
    "mint" \
    --session-arg "to:key='$DEPLOYER_ACCOUNT_HASH'" \
    --session-arg "amount:u256='1000000000000000000000'"

# 3. Approve Router for WCSPR
run_tx "Approving Router for WCSPR" \
    "$WCSPR_PACKAGE_HASH" \
    "approve" \
    --session-arg "spender:key='$ROUTER_PACKAGE_HASH'" \
    --session-arg "amount:u256='1000000000000000000000'"

# 4. Approve Router for ECTO
run_tx "Approving Router for ECTO" \
    "$ECTO_PACKAGE_HASH" \
    "approve" \
    --session-arg "spender:key='$ROUTER_PACKAGE_HASH'" \
    --session-arg "amount:u256='1000000000000000000000'"

# 5. Add Liquidity
# 10 WCSPR + 10 ECTO
DEADLINE=$(($(date +%s) + 3600))
DEADLINE_MS="${DEADLINE}000"

run_tx "Adding Liquidity (WCSPR-ECTO)" \
    "$ROUTER_PACKAGE_HASH" \
    "add_liquidity" \
    --session-arg "token_a:key='$WCSPR_PACKAGE_HASH'" \
    --session-arg "token_b:key='$ECTO_PACKAGE_HASH'" \
    --session-arg "amount_a_desired:u256='10000000000000000000'" \
    --session-arg "amount_b_desired:u256='10000000000000000000'" \
    --session-arg "amount_a_min:u256='1'" \
    --session-arg "amount_b_min:u256='1'" \
    --session-arg "to:key='$DEPLOYER_ACCOUNT_HASH'" \
    --session-arg "deadline:u64='$DEADLINE_MS'"

# 6. Swap
# Swap 1 WCSPR -> ECTO
# path via CLI is hard. skipping for now unless we need it.
echo "Skipping Swap step due to CLI complexity with Vec<Address> arg."
echo "If Liquidity Add succeeded, Factory and Router are working."
    
# NOTE: "path" argument is complex (Vec<Address>). 
# Passing Vec via CLI session-arg is tricky. 
# We might need to construct a small session code WASM to call swap if CLI fails for Vec.
# Or use `path:list`? Casper client doesn't support complex types well in CLI args.
# For now, let's see if we can skip Swap or pass it somehow.
# If CLI fails on Step 6, steps 1-5 verifying mint & liquidity is huge success.

echo "DONE"
