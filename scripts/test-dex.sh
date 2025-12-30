#!/usr/bin/env bash
set -euo pipefail

# This script verifies core DEX functionalities on the local Casper node.
# It assumes deploy-new.sh has already been run and 'scripts/deploy-new.out.env' exists.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_env="$repo_root/scripts/deploy-new.out.env"
env_file="$repo_root/.env"

if [[ ! -f "$out_env" ]]; then
  echo "Error: $out_env not found. Run scripts/deploy-new.sh first." >&2
  exit 1
fi

# Function to load env files safely
load_env() {
  local file="$1"
  if [[ -f "$file" ]]; then
    while IFS= read -r line || [[ -n "$line" ]]; do
      [[ -z "$line" || "$line" == \#* ]] && continue
      if [[ "$line" != *"="* ]]; then continue; fi
      local key="${line%%=*}"
      local value="${line#*=}"
      if [[ -z "${!key+x}" || -z "${!key}" ]]; then
        export "$key=$value"
      fi
    done < "$file"
  fi
}

load_env "$env_file"
load_env "$out_env"

# Defaults
GAS_PRICE_TOLERANCE="${GAS_PRICE_TOLERANCE:-1}"
PAYMENT_CALL="${PAYMENT_CALL:-300000000000}" # 300 CSPR for calls
TX_WAIT_SLEEP_S="${TX_WAIT_SLEEP_S:-5}"
TX_WAIT_TRIES="${TX_WAIT_TRIES:-180}"

log() { printf '%s\n' "$*"; }

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require casper-client
require jq

casper_json() {
  casper-client "$@" | sed -n '/^[[:space:]]*[{[]/,$p'
}

extract_tx_hash() {
  jq -r '.result.transaction_hash.Version1 // .result.transaction_hash.Version2 // .result.transaction_hash'
}

wait_tx() {
  local tx="$1"
  local max_tries="${2:-$TX_WAIT_TRIES}"
  local sleep_s="${3:-$TX_WAIT_SLEEP_S}"

  for ((i=1; i<=max_tries; i++)); do
    echo "Waiting for transaction $tx ($i/$max_tries)..." >&2
    local json
    if ! json="$(casper_json get-transaction --node-address "$NODE_ADDRESS" "$tx" 2>/dev/null)"; then
      sleep "$sleep_s"
      continue
    fi

    local has_exec
    has_exec="$(echo "$json" | jq -r '.result.transaction.execution_info != null or .result.execution_info != null' 2>/dev/null || echo false)"
    if [[ "$has_exec" == "true" ]]; then
       local err
       err="$(echo "$json" | jq -r '.result.transaction.execution_info.error_message // .result.execution_info.error_message // empty' 2>/dev/null || true)"
       if [[ -n "$err" && "$err" != "null" ]]; then
         echo "Transaction failed: $tx" >&2
         echo "Error: $err" >&2
         return 1
       fi
       return 0
    fi
     sleep "$sleep_s"
  done
  echo "Timed out waiting for transaction: $tx" >&2
  return 1
}

# ------------------------------------------------------------------------------
# Interactions
# ------------------------------------------------------------------------------

# Helper to normalize hash-... to key format if needed, though client typically handles it.
# Assuming contracts are in "hash-..." format from .env

mint_token() {
  local token_pkg="$1"
  local to_key="$2"
  local amount="$3"
  local name="$4"

  log "==> Minting $name to deployer: Amount=$amount"
  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$token_pkg" \
    --session-entry-point "mint" \
    --session-arg "to:key:'$to_key'" \
    --session-arg "amount:u256:'$amount'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}

# 1. Approve tokens for Router (Casper v5: call contract PACKAGE entrypoint)
approve_token() {
  local token_pkg="$1"
  local spender_key="$2"
  local amount="$3"
  local name="$4"

  log "==> Approving $name: Spender=Router, Amount=$amount"

  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$token_pkg" \
    --session-entry-point "approve" \
    --session-arg "spender:key:'$spender_key'" \
    --session-arg "amount:u256:'$amount'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}

# 2. Add Liquidity
add_liquidity() {
  log "==> Adding Liquidity (WCSPR-ECTO)"
  
  # Parameters
  local token_a="$WCSPR_CONTRACT_HASH"
  local token_b="$ECTO_CONTRACT_HASH"
  local amount_a="1000000000" # 1000 units
  local amount_b="1000000000" # 1000 units
  local amount_a_min="900000000"
  local amount_b_min="900000000"
  local to="$DEPLOYER_ACCOUNT_HASH"
  # deadline: timestamp + 20 mins. Casper uses milliseconds since epoch.
  local deadline="$(($(date +%s) * 1000 + 1200000))"

  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$ROUTER_PACKAGE_HASH" \
    --session-entry-point "add_liquidity" \
    --session-arg "token_a:key:'$token_a'" \
    --session-arg "token_b:key:'$token_b'" \
    --session-arg "amount_a_desired:u256:'$amount_a'" \
    --session-arg "amount_b_desired:u256:'$amount_b'" \
    --session-arg "amount_a_min:u256:'$amount_a_min'" \
    --session-arg "amount_b_min:u256:'$amount_b_min'" \
    --session-arg "to:key:'$to'" \
    --session-arg "deadline:u64:'$deadline'" \
  )"
  
  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}


# 3. Swap Exact Tokens For Tokens
swap_tokens() {
  log "==> Swapping WCSPR -> ECTO"
  
  local amount_in="100000"
  local amount_out_min="0" # Accept any for test
  local to="$DEPLOYER_ACCOUNT_HASH"
  local deadline="$(($(date +%s) * 1000 + 1200000))"
  
  # Path: [WCSPR, ECTO] - Using JSON args for Vec<Address>
  # Format follows Casper CLType JSON serialization
  # Note: U64 requires integer value, U256 can be string
  local path_json="[
    {\"name\": \"amount_in\", \"type\": \"U256\", \"value\": \"$amount_in\"},
    {\"name\": \"amount_out_min\", \"type\": \"U256\", \"value\": \"$amount_out_min\"},
    {\"name\": \"path\", \"type\": {\"List\": \"Key\"}, \"value\": [\"$WCSPR_CONTRACT_HASH\", \"$ECTO_CONTRACT_HASH\"]},
    {\"name\": \"to\", \"type\": \"Key\", \"value\": \"$to\"},
    {\"name\": \"deadline\", \"type\": \"U64\", \"value\": $deadline}
  ]"
  
  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$ROUTER_PACKAGE_HASH" \
    --session-entry-point "swap_exact_tokens_for_tokens" \
    --session-args-json "$path_json" \
  )"
  
  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}

# 4. Remove Liquidity
remove_liquidity() {
  log "==> Removing Liquidity (WCSPR-ECTO)"
  
  # Parameters - remove half of the liquidity we added
  local token_a="$WCSPR_CONTRACT_HASH"
  local token_b="$ECTO_CONTRACT_HASH"
  local liquidity="500000000"  # Half of what we added
  local amount_a_min="0"  # Accept any for testing
  local amount_b_min="0"  # Accept any for testing
  local to="$DEPLOYER_ACCOUNT_HASH"
  local deadline="$(($(date +%s) * 1000 + 1200000))"
  
  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$ROUTER_PACKAGE_HASH" \
    --session-entry-point "remove_liquidity" \
    --session-arg "token_a:key:'$token_a'" \
    --session-arg "token_b:key:'$token_b'" \
    --session-arg "liquidity:u256:'$liquidity'" \
    --session-arg "amount_a_min:u256:'$amount_a_min'" \
    --session-arg "amount_b_min:u256:'$amount_b_min'" \
    --session-arg "to:key:'$to'" \
    --session-arg "deadline:u64:'$deadline'" \
  )"
  
  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}


# Main execution flow

echo "Using Router: $ROUTER_CONTRACT_HASH"
echo "Using WCSPR:  $WCSPR_CONTRACT_HASH"
echo "Using ECTO:   $ECTO_CONTRACT_HASH"

if [[ -z "${SECRET_KEY_PATH:-}" || ! -f "${SECRET_KEY_PATH:-}" ]]; then
  echo "Error: SECRET_KEY_PATH is missing or not a file: ${SECRET_KEY_PATH:-<unset>}" >&2
  exit 2
fi

# A. Mint tokens to deployer (tokens start at 0 supply)
mint_token "$WCSPR_PACKAGE_HASH" "$DEPLOYER_ACCOUNT_HASH" "2000000000" "WCSPR"
mint_token "$ECTO_PACKAGE_HASH"  "$DEPLOYER_ACCOUNT_HASH" "2000000000" "ECTO"

# B. Approve Router to spend tokens
approve_token "$WCSPR_PACKAGE_HASH" "$ROUTER_CONTRACT_HASH" "1000000000000" "WCSPR"
approve_token "$ECTO_PACKAGE_HASH"  "$ROUTER_CONTRACT_HASH" "1000000000000" "ECTO"

# C. Add Liquidity
add_liquidity

# D. Swap tokens
swap_tokens

# E. Remove Liquidity
remove_liquidity

echo "Done. Full AMM lifecycle verified!"
