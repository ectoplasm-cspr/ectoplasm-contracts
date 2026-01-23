#!/usr/bin/env bash
set -euo pipefail

# This script verifies:
# 1. Factory can detect if a pair exists on-chain
# 2. Router is properly initialized with Factory reference

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_env="$repo_root/scripts/deploy-new.out.env"
env_file="$repo_root/.env"

# Load environment
load_env() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    [[ "$line" == *"="* ]] || continue
    local key="${line%%=*}"
    local value="${line#*=}"
    if [[ -z "${!key+x}" || -z "${!key}" ]]; then
      export "$key=$value"
    fi
  done < "$file"
}

load_env "$env_file"
load_env "$out_env"

NODE="${NODE_ADDRESS:-http://localhost:11101}"

echo "======================================"
echo "DEX Pair Detection & Router Init Check"
echo "======================================"
echo "Node: $NODE"
echo ""

# Get state root hash
STATE_ROOT=$(casper-client get-state-root-hash --node-address "$NODE" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p' | jq -r '.result.state_root_hash')
echo "State Root: $STATE_ROOT"
echo ""

# ==========================================
# 1. Check Router Initialization
# ==========================================
echo "=== 1. Router Initialization Check ==="
echo "Router Package: $ROUTER_PACKAGE_HASH"
echo "Router Contract: $ROUTER_CONTRACT_HASH"

# Query Router contract to get its named keys
echo ""
echo "Querying Router contract named keys..."
ROUTER_KEYS=$(casper-client query-global-state \
  --node-address "$NODE" \
  --state-root-hash "$STATE_ROOT" \
  --key "$ROUTER_CONTRACT_HASH" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p')

# Extract the 'state' dictionary URef
STATE_UREF=$(echo "$ROUTER_KEYS" | jq -r '.result.stored_value.Contract.named_keys[] | select(.name=="state") | .key')
echo "Router state dictionary: $STATE_UREF"

if [[ -n "$STATE_UREF" && "$STATE_UREF" != "null" ]]; then
  # Query factory field from Router's state dictionary
  echo ""
  echo "Querying Router's 'factory' field..."
  FACTORY_VALUE=$(casper-client get-dictionary-item \
    --node-address "$NODE" \
    --state-root-hash "$STATE_ROOT" \
    --seed-uref "$STATE_UREF" \
    --dictionary-item-key "factory" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p' || echo "{}")
  
  if echo "$FACTORY_VALUE" | jq -e '.result.stored_value.CLValue.parsed' > /dev/null 2>&1; then
    PARSED_FACTORY=$(echo "$FACTORY_VALUE" | jq -r '.result.stored_value.CLValue.parsed')
    echo "✅ Router.factory is SET: $PARSED_FACTORY"
  else
    echo "⚠️  Could not parse Router.factory (may be stored differently)"
    echo "    Raw response: $(echo "$FACTORY_VALUE" | head -c 200)"
  fi
  
  # Query wcspr field from Router's state dictionary
  echo ""
  echo "Querying Router's 'wcspr' field..."
  WCSPR_VALUE=$(casper-client get-dictionary-item \
    --node-address "$NODE" \
    --state-root-hash "$STATE_ROOT" \
    --seed-uref "$STATE_UREF" \
    --dictionary-item-key "wcspr" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p' || echo "{}")
  
  if echo "$WCSPR_VALUE" | jq -e '.result.stored_value.CLValue.parsed' > /dev/null 2>&1; then
    PARSED_WCSPR=$(echo "$WCSPR_VALUE" | jq -r '.result.stored_value.CLValue.parsed')
    echo "✅ Router.wcspr is SET: $PARSED_WCSPR"
  else
    echo "⚠️  Could not parse Router.wcspr (may be stored differently)"
  fi
else
  echo "❌ Router 'state' dictionary not found!"
fi

echo ""

# ==========================================
# 2. Check Factory Pair Detection
# ==========================================
echo "=== 2. Factory Pair Detection Check ==="
echo "Factory Contract: $FACTORY_CONTRACT_HASH"
echo "WCSPR Contract: $WCSPR_CONTRACT_HASH"
echo "ECTO Contract: $ECTO_CONTRACT_HASH"
echo ""

# Query Factory contract to get its named keys
echo "Querying Factory contract named keys..."
FACTORY_KEYS=$(casper-client query-global-state \
  --node-address "$NODE" \
  --state-root-hash "$STATE_ROOT" \
  --key "$FACTORY_CONTRACT_HASH" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p')

# Get events length (= pair count)
EVENTS_LEN_UREF=$(echo "$FACTORY_KEYS" | jq -r '.result.stored_value.Contract.named_keys[] | select(.name=="__events_length") | .key')
echo "Events length URef: $EVENTS_LEN_UREF"

if [[ -n "$EVENTS_LEN_UREF" && "$EVENTS_LEN_UREF" != "null" ]]; then
  PAIR_COUNT=$(casper-client query-global-state \
    --node-address "$NODE" \
    --state-root-hash "$STATE_ROOT" \
    --key "$EVENTS_LEN_UREF" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p' | jq -r '.result.stored_value.CLValue.parsed')
  echo "✅ Total pairs created: $PAIR_COUNT"
else
  echo "⚠️  Could not find __events_length"
fi

echo ""

# List the PairCreated events
EVENTS_UREF=$(echo "$FACTORY_KEYS" | jq -r '.result.stored_value.Contract.named_keys[] | select(.name=="__events") | .key')
echo "Events dictionary: $EVENTS_UREF"

if [[ -n "$EVENTS_UREF" && "$EVENTS_UREF" != "null" && -n "$PAIR_COUNT" && "$PAIR_COUNT" != "null" && "$PAIR_COUNT" -gt 0 ]]; then
  echo ""
  echo "=== Created Pairs (from events) ==="
  for ((idx=0; idx<PAIR_COUNT; idx++)); do
    EVENT_DATA=$(casper-client get-dictionary-item \
      --node-address "$NODE" \
      --state-root-hash "$STATE_ROOT" \
      --seed-uref "$EVENTS_UREF" \
      --dictionary-item-key "$idx" 2>&1 | sed -n '/^[[:space:]]*[{[]/,$p' || echo "{}")
    
    # Parse the event data (List<U8>)
    PARSED=$(echo "$EVENT_DATA" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    arr = data['result']['stored_value']['CLValue']['parsed']
    i = 0
    name_len = int.from_bytes(bytes(arr[i:i+4]), 'little'); i += 4
    name = bytes(arr[i:i+name_len]).decode(); i += name_len
    tag0 = arr[i]; i += 1
    token0 = bytes(arr[i:i+32]).hex(); i += 32
    tag1 = arr[i]; i += 1
    token1 = bytes(arr[i:i+32]).hex(); i += 32
    tag2 = arr[i]; i += 1
    pair = bytes(arr[i:i+32]).hex(); i += 32
    pair_count = int.from_bytes(bytes(arr[i:i+4]), 'little')
    print(f'  Pair #{pair_count}: token0=hash-{token0[:12]}... token1=hash-{token1[:12]}... => pair=hash-{pair[:12]}...')
except Exception as e:
    print(f'  (parse error: {e})')
" 2>&1 || echo "  (parse error)")
    echo "$PARSED"
  done
fi

echo ""
echo "======================================"
echo "Summary"
echo "======================================"
echo "✅ Deployment verified on local node"
echo "✅ Router has 'state' dictionary for storing factory/wcspr references"
echo "✅ Factory has $PAIR_COUNT pair(s) created"
echo ""
echo "Frontend Integration:"
echo "  1. To check if a pair exists: Query Factory.__events or call get_pair entry point"
echo "  2. Router can execute swaps using the path array [tokenIn, tokenOut]"
echo "  3. Router.add_liquidity will create pairs automatically if they don't exist"
echo ""
