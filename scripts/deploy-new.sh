#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/deploy-new.sh [--build] [--skip-tokens] [--skip-dex] [--create-pairs]

Environment variables (recommended via .env):
  NODE_ADDRESS            Node RPC URL for casper-client.
                         Example local:  http://127.0.0.1:11101
                         Example testnet (hosted): https://<host>/rpc
                         Example testnet (node):   http://<host>:7777
  CHAIN_NAME              Chain name. Example local: casper-net-1  | testnet: casper-test
  SECRET_KEY_PATH         Path to secret key pem (for signing).
  DEPLOYER_ACCOUNT_HASH   account-hash-... used for Factory fee_to_setter + Router recipient.

Optional:
  GAS_PRICE_TOLERANCE     Default: 1
  TX_WAIT_TRIES           Default: 180
  TX_WAIT_SLEEP_S         Default: 5
  PAYMENT_TOKEN           Default: 600000000000
  PAYMENT_FACTORY         Default: 1500000000000
  PAYMENT_ROUTER          Default: 500000000000
  PAYMENT_CALL            Default: 300000000000
  PAYMENT_CREATE_PAIR     Default: 900000000000

What it does:
  - Deploys: WCSPR(LpToken), ECTO, USDC, WETH, WBTC, Factory, Router
  - Stores package hashes under your account named keys (Odra installer args)
  - Derives active contract hashes from package hashes via query-global-state
  - Optionally calls Factory.create_pair for common pairs

Notes:
  - Requires: casper-client, jq
  - This script targets Casper 2.x using casper-client v5+ (put-transaction).
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

# Load .env if present (no failure if absent).
# Do not override values already provided via the environment.
if [[ -f .env ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    if [[ "$line" != *"="* ]]; then
      continue
    fi
    key="${line%%=*}"
    value="${line#*=}"
    if [[ -z "${!key+x}" ]]; then
      export "$key=$value"
    fi
  done < .env
fi

NODE_ADDRESS="${NODE_ADDRESS:-}"
CHAIN_NAME="${CHAIN_NAME:-}"
SECRET_KEY_PATH="${SECRET_KEY_PATH:-}"
DEPLOYER_ACCOUNT_HASH="${DEPLOYER_ACCOUNT_HASH:-}"

GAS_PRICE_TOLERANCE="${GAS_PRICE_TOLERANCE:-1}"
TX_WAIT_TRIES="${TX_WAIT_TRIES:-180}"
TX_WAIT_SLEEP_S="${TX_WAIT_SLEEP_S:-5}"
PAYMENT_TOKEN="${PAYMENT_TOKEN:-600000000000}"
PAYMENT_FACTORY="${PAYMENT_FACTORY:-1500000000000}"
PAYMENT_ROUTER="${PAYMENT_ROUTER:-500000000000}"
PAYMENT_CALL="${PAYMENT_CALL:-300000000000}"
PAYMENT_CREATE_PAIR="${PAYMENT_CREATE_PAIR:-900000000000}"

BUILD=0
SKIP_TOKENS=0
SKIP_DEX=0
CREATE_PAIRS=0

for arg in "$@"; do
  case "$arg" in
    --build) BUILD=1 ;;
    --skip-tokens) SKIP_TOKENS=1 ;;
    --skip-dex) SKIP_DEX=1 ;;
    --create-pairs) CREATE_PAIRS=1 ;;
    *)
      echo "Unknown arg: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "Missing required command: $name" >&2
    exit 1
  fi
}

require casper-client
require jq

is_valid_account_hash() {
  local s="$1"
  [[ "$s" =~ ^account-hash-[0-9a-fA-F]{64}$ ]]
}

if [[ -z "$NODE_ADDRESS" || -z "$CHAIN_NAME" || -z "$SECRET_KEY_PATH" || -z "$DEPLOYER_ACCOUNT_HASH" ]]; then
  echo "Missing required env vars." >&2
  echo "Required: NODE_ADDRESS, CHAIN_NAME, SECRET_KEY_PATH, DEPLOYER_ACCOUNT_HASH" >&2
  echo "Tip: copy .env.example to .env and fill it." >&2
  exit 2
fi

if ! is_valid_account_hash "$DEPLOYER_ACCOUNT_HASH"; then
  echo "DEPLOYER_ACCOUNT_HASH must look like: account-hash-<64 hex chars>" >&2
  echo "Got: $DEPLOYER_ACCOUNT_HASH" >&2
  echo "To compute it from a public key file:" >&2
  echo "  casper-client account-address --public-key keys/public_key.pem" >&2
  exit 2
fi

if [[ ! -f "$SECRET_KEY_PATH" ]]; then
  echo "Secret key not found: $SECRET_KEY_PATH" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
wasm_dir="$repo_root/wasm"

log() { printf '%s\n' "$*"; }

casper_json() {
  # casper-client may emit non-JSON lines; keep only the JSON payload (from first '{' or '[').
  # This makes jq parsing resilient across client versions/environments.
  casper-client "$@" | sed -n '/^[[:space:]]*[{[]/,$p'
}

extract_tx_hash() {
  # casper-client v5 returns `.result.transaction_hash` as an enum-like object, e.g. {"Version1":"<hex>"}.
  # Normalize to the raw hex string.
  jq -r '.result.transaction_hash.Version1 // .result.transaction_hash.Version2 // .result.transaction_hash'
}

get_state_root_hash() {
  casper_json get-state-root-hash --node-address "$NODE_ADDRESS" | jq -r '.result.state_root_hash'
}

get_named_key() {
  local name="$1"
  casper_json get-account \
    --node-address "$NODE_ADDRESS" \
    --account-identifier "$DEPLOYER_ACCOUNT_HASH" \
    | jq -r ".result.account.named_keys[] | select(.name==\"$name\") | .key" \
    | head -n 1
}

active_contract_hash_from_package() {
  local package_hash="$1" # formatted, e.g. hash-....
  local srh
  srh="$(get_state_root_hash)"

  local contract_hash
  contract_hash="$(
    casper_json query-global-state \
      --node-address "$NODE_ADDRESS" \
      --state-root-hash "$srh" \
      --key "$package_hash" \
      | jq -r '.result.stored_value.ContractPackage.versions | max_by(.contract_version) | .contract_hash'
  )"

  # normalize "contract-<hex>" -> "hash-<hex>" for casper-client flags/SDK
  if [[ "$contract_hash" == contract-* ]]; then
    printf 'hash-%s\n' "${contract_hash#contract-}"
  elif [[ "$contract_hash" == hash-* ]]; then
    printf '%s\n' "$contract_hash"
  else
    echo "$contract_hash" # fall back
  fi
}

wait_tx() {
  local tx="$1"
  local max_tries="${2:-$TX_WAIT_TRIES}"
  local sleep_s="${3:-$TX_WAIT_SLEEP_S}"

  local log_every="${TX_WAIT_LOG_EVERY:-5}"

  for ((i=1; i<=max_tries; i++)); do
    if (( i == 1 || (log_every > 0 && i % log_every == 0) )); then
      echo "Waiting for transaction $tx ($i/$max_tries)..." >&2
    fi
    local json
    if ! json="$(casper_json get-transaction --node-address "$NODE_ADDRESS" "$tx" 2>/dev/null)"; then
      sleep "$sleep_s"
      continue
    fi

    # execution_info appears once executed/finalized
    local has_exec
    has_exec="$(echo "$json" | jq -r '.result.transaction.execution_info != null or .result.execution_info != null' 2>/dev/null || echo false)"
    if [[ "$has_exec" != "true" ]]; then
      sleep "$sleep_s"
      continue
    fi

    local err
    err="$(echo "$json" | jq -r '
      .result.transaction.execution_info.execution_result.Version2.error_message
      // .result.transaction.execution_info.execution_result.Version1.error_message
      // .result.execution_info.execution_result.Version2.error_message
      // .result.execution_info.execution_result.Version1.error_message
      // .result.transaction.execution_info.error_message
      // .result.execution_info.error_message
      // empty
    ' 2>/dev/null || true)"
    if [[ -n "$err" && "$err" != "null" ]]; then
      echo "Transaction failed: $tx" >&2
      echo "Error: $err" >&2
      return 1
    fi

    return 0
  done

  echo "Timed out waiting for transaction: $tx" >&2
  return 1
}

preflight() {
  log "==> Preflight"
  # Fail early on bad RPC / network config.
  if ! casper-client get-state-root-hash --node-address "$NODE_ADDRESS" >/dev/null 2>&1; then
    echo "Unable to reach node RPC at NODE_ADDRESS=$NODE_ADDRESS" >&2
    echo "Tip: NODE_ADDRESS must point to the node's JSON-RPC endpoint (often ends with /rpc on hosted endpoints)." >&2
    exit 2
  fi

  if ! casper-client get-account --node-address "$NODE_ADDRESS" --account-identifier "$DEPLOYER_ACCOUNT_HASH" >/dev/null 2>&1; then
    echo "Unable to fetch deployer account. Check DEPLOYER_ACCOUNT_HASH and funding." >&2
    echo "DEPLOYER_ACCOUNT_HASH=$DEPLOYER_ACCOUNT_HASH" >&2
    exit 2
  fi
}

call_init() {
  local package_hash="$1"
  local entry_point="init"
  shift 2
  local args=("$@")

  log "==> Calling $entry_point on $package_hash"

  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --contract-package-hash "$package_hash" \
    --session-entry-point "$entry_point" \
    --payment-amount "$PAYMENT_CALL" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    "${args[@]}" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
}


deploy_session_install() {
  local label="$1"
  local wasm_path="$2"
  local package_key_name="$3"
  shift 3

  log "==> Deploying $label"

  local out
  out="$(casper_json put-transaction session \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --wasm-path "$wasm_path" \
    --payment-amount "$PAYMENT_TOKEN" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --install-upgrade \
    --session-arg "odra_cfg_package_hash_key_name:string:'$package_key_name'" \
    --session-arg "odra_cfg_allow_key_override:bool:'true'" \
    --session-arg "odra_cfg_is_upgradable:bool:'true'" \
    --session-arg "odra_cfg_is_upgrade:bool:'false'" \
    "$@" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
  echo "$tx"
}

deploy_pair_factory() {
  log "==> Deploying PairFactory"

  local out
  out="$(casper_json put-transaction session \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --wasm-path "$wasm_dir/PairFactory.wasm" \
    --payment-amount "$PAYMENT_FACTORY" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --install-upgrade \
    --session-arg "odra_cfg_package_hash_key_name:string:'pair_factory_package_hash'" \
    --session-arg "odra_cfg_allow_key_override:bool:'true'" \
    --session-arg "odra_cfg_is_upgradable:bool:'true'" \
    --session-arg "odra_cfg_is_upgrade:bool:'false'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
  echo "$tx"
}

deploy_factory() {
  local pair_factory_pkg_hash="$1" # hash-... (PairFactory package hash)

  log "==> Deploying Factory"

  local out
  out="$(casper_json put-transaction session \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --wasm-path "$wasm_dir/Factory.wasm" \
    --payment-amount "$PAYMENT_FACTORY" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --install-upgrade \
    --session-arg "odra_cfg_package_hash_key_name:string:'factory_package_hash'" \
    --session-arg "odra_cfg_allow_key_override:bool:'true'" \
    --session-arg "odra_cfg_is_upgradable:bool:'true'" \
    --session-arg "odra_cfg_is_upgrade:bool:'false'" \
    --session-arg "fee_to_setter:key:'$DEPLOYER_ACCOUNT_HASH'" \
    --session-arg "pair_factory:key:'$pair_factory_pkg_hash'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
  echo "$tx"
}

deploy_router() {
  local factory_pkg_hash="$1" # hash-... (Factory package hash)
  local wcspr_pkg_hash="$2"   # hash-... (WCSPR package hash)

  log "==> Deploying Router"

  local out
  out="$(casper_json put-transaction session \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --wasm-path "$wasm_dir/Router.wasm" \
    --payment-amount "$PAYMENT_ROUTER" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --install-upgrade \
    --session-arg "odra_cfg_package_hash_key_name:string:'router_package_hash'" \
    --session-arg "odra_cfg_allow_key_override:bool:'true'" \
    --session-arg "odra_cfg_is_upgradable:bool:'true'" \
    --session-arg "odra_cfg_is_upgrade:bool:'false'" \
    --session-arg "factory:key:'$factory_pkg_hash'" \
    --session-arg "wcspr:key:'$wcspr_pkg_hash'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  log "TX: $tx"
  wait_tx "$tx"
  echo "$tx"
}

call_factory_create_pair() {
  local factory_pkg="$1" # hash-...
  local token_a="$2"     # hash-... (token package hash)
  local token_b="$3"     # hash-... (token package hash)

  echo "==> Factory.create_pair($token_a, $token_b)" >&2

  local out
  out="$(casper_json put-transaction package \
    --node-address "$NODE_ADDRESS" \
    --chain-name "$CHAIN_NAME" \
    --secret-key "$SECRET_KEY_PATH" \
    --payment-amount "$PAYMENT_CREATE_PAIR" \
    --gas-price-tolerance "$GAS_PRICE_TOLERANCE" \
    --standard-payment true \
    --contract-package-hash "$factory_pkg" \
    --session-entry-point create_pair \
    --session-arg "token_a:key:'$token_a'" \
    --session-arg "token_b:key:'$token_b'" \
  )"

  local tx
  tx="$(echo "$out" | extract_tx_hash)"
  echo "TX: $tx" >&2
  wait_tx "$tx"

  # Emit tx hash on stdout so callers can capture it.
  echo "$tx"
}

if [[ $BUILD -eq 1 ]]; then
  log "==> Building WASM (cargo odra build)"
  (cd "$repo_root" && cargo odra build)
fi

for f in Factory.wasm Router.wasm LpToken.wasm EctoToken.wasm UsdcToken.wasm WethToken.wasm WbtcToken.wasm; do
  if [[ ! -f "$wasm_dir/$f" ]]; then
    echo "Missing wasm: $wasm_dir/$f" >&2
    echo "Run: cargo odra build" >&2
    exit 2
  fi
done

if [[ $SKIP_DEX -eq 0 ]]; then
  if [[ ! -f "$wasm_dir/PairFactory.wasm" ]]; then
    echo "Missing wasm: $wasm_dir/PairFactory.wasm" >&2
    echo "Run: cargo odra build" >&2
    exit 2
  fi
fi

preflight

out_env="$repo_root/scripts/deploy-new.out.env"
out_env_tmp="$repo_root/scripts/.deploy-new.out.env.tmp"
rm -f "$out_env_tmp"

# Deploy tokens
if [[ $SKIP_TOKENS -eq 0 ]]; then
  deploy_session_install "WCSPR (LpToken)" "$wasm_dir/LpToken.wasm" "wcspr_token_package_hash" \
    --session-arg "name:string:'Wrapped CSPR'" \
    --session-arg "symbol:string:'WCSPR'"

  deploy_session_install "ECTO" "$wasm_dir/EctoToken.wasm" "ecto_token_package_hash"
  deploy_session_install "USDC" "$wasm_dir/UsdcToken.wasm" "usdc_token_package_hash"
  deploy_session_install "WETH" "$wasm_dir/WethToken.wasm" "weth_token_package_hash"
  deploy_session_install "WBTC" "$wasm_dir/WbtcToken.wasm" "wbtc_token_package_hash"
fi

# Resolve package hashes from named keys
PAIR_FACTORY_PKG="$(get_named_key pair_factory_package_hash || true)"
FACTORY_PKG="$(get_named_key factory_package_hash || true)"
ROUTER_PKG="$(get_named_key router_package_hash || true)"
WCSPR_PKG="$(get_named_key wcspr_token_package_hash || true)"
ECTO_PKG="$(get_named_key ecto_token_package_hash || true)"
USDC_PKG="$(get_named_key usdc_token_package_hash || true)"
WETH_PKG="$(get_named_key weth_token_package_hash || true)"
WBTC_PKG="$(get_named_key wbtc_token_package_hash || true)"

# Deploy DEX
if [[ $SKIP_DEX -eq 0 ]]; then
  deploy_pair_factory
  PAIR_FACTORY_PKG="$(get_named_key pair_factory_package_hash)"
  PAIR_FACTORY_CONTRACT="$(active_contract_hash_from_package "$PAIR_FACTORY_PKG")"
  
  # Pass PairFactory *package* hash for contract-to-contract calls.
  deploy_factory "$PAIR_FACTORY_PKG"
  FACTORY_PKG="$(get_named_key factory_package_hash)"

  FACTORY_CONTRACT="$(active_contract_hash_from_package "$FACTORY_PKG")"

  # Ensure WCSPR exists (either deployed above or already present)
  if [[ -z "${WCSPR_PKG:-}" || "$WCSPR_PKG" == "null" ]]; then
    WCSPR_PKG="$(get_named_key wcspr_token_package_hash)"
  fi
  WCSPR_CONTRACT="$(active_contract_hash_from_package "$WCSPR_PKG")"

  # Pass Factory + WCSPR *package* hashes for contract-to-contract calls.
  deploy_router "$FACTORY_PKG" "$WCSPR_PKG"

  ROUTER_PKG="$(get_named_key router_package_hash)"
fi

# Derive contract hashes (useful for reads)
PAIR_FACTORY_CONTRACT="${PAIR_FACTORY_PKG:+$(active_contract_hash_from_package "$PAIR_FACTORY_PKG")}" || true
FACTORY_CONTRACT="${FACTORY_PKG:+$(active_contract_hash_from_package "$FACTORY_PKG")}" || true
ROUTER_CONTRACT="${ROUTER_PKG:+$(active_contract_hash_from_package "$ROUTER_PKG")}" || true
WCSPR_CONTRACT="${WCSPR_PKG:+$(active_contract_hash_from_package "$WCSPR_PKG")}" || true
ECTO_CONTRACT="${ECTO_PKG:+$(active_contract_hash_from_package "$ECTO_PKG")}" || true
USDC_CONTRACT="${USDC_PKG:+$(active_contract_hash_from_package "$USDC_PKG")}" || true
WETH_CONTRACT="${WETH_PKG:+$(active_contract_hash_from_package "$WETH_PKG")}" || true
WBTC_CONTRACT="${WBTC_PKG:+$(active_contract_hash_from_package "$WBTC_PKG")}" || true

{
  echo "NODE_ADDRESS=$NODE_ADDRESS"
  echo "CHAIN_NAME=$CHAIN_NAME"
  echo "DEPLOYER_ACCOUNT_HASH=$DEPLOYER_ACCOUNT_HASH"
  echo
  echo "PAIR_FACTORY_PACKAGE_HASH=$PAIR_FACTORY_PKG"
  echo "FACTORY_PACKAGE_HASH=$FACTORY_PKG"
  echo "ROUTER_PACKAGE_HASH=$ROUTER_PKG"
  echo "WCSPR_PACKAGE_HASH=$WCSPR_PKG"
  echo "ECTO_PACKAGE_HASH=$ECTO_PKG"
  echo "USDC_PACKAGE_HASH=$USDC_PKG"
  echo "WETH_PACKAGE_HASH=$WETH_PKG"
  echo "WBTC_PACKAGE_HASH=$WBTC_PKG"
  echo
  echo "PAIR_FACTORY_CONTRACT_HASH=$PAIR_FACTORY_CONTRACT"
  echo "FACTORY_CONTRACT_HASH=$FACTORY_CONTRACT"
  echo "ROUTER_CONTRACT_HASH=$ROUTER_CONTRACT"
  echo "WCSPR_CONTRACT_HASH=$WCSPR_CONTRACT"
  echo "ECTO_CONTRACT_HASH=$ECTO_CONTRACT"
  echo "USDC_CONTRACT_HASH=$USDC_CONTRACT"
  echo "WETH_CONTRACT_HASH=$WETH_CONTRACT"
  echo "WBTC_CONTRACT_HASH=$WBTC_CONTRACT"
} >> "$out_env_tmp"

mv -f "$out_env_tmp" "$out_env"
log "==> Wrote: $out_env"

if [[ $CREATE_PAIRS -eq 1 ]]; then
  if [[ -z "${FACTORY_PKG:-}" || "$FACTORY_PKG" == "null" ]]; then
    echo "Missing FACTORY_PACKAGE_HASH; cannot create pairs." >&2
    exit 1
  fi

  # Create common pairs (using *package* hashes as token addresses).
  PAIR_TX_WCSPR_USDC="$(call_factory_create_pair "$FACTORY_PKG" "$WCSPR_PKG" "$USDC_PKG")"
  echo "PAIR_TX_WCSPR_USDC=$PAIR_TX_WCSPR_USDC" >> "$out_env"

  PAIR_TX_WCSPR_ECTO="$(call_factory_create_pair "$FACTORY_PKG" "$WCSPR_PKG" "$ECTO_PKG")"
  echo "PAIR_TX_WCSPR_ECTO=$PAIR_TX_WCSPR_ECTO" >> "$out_env"

  PAIR_TX_WCSPR_WETH="$(call_factory_create_pair "$FACTORY_PKG" "$WCSPR_PKG" "$WETH_PKG")"
  echo "PAIR_TX_WCSPR_WETH=$PAIR_TX_WCSPR_WETH" >> "$out_env"

  PAIR_TX_WCSPR_WBTC="$(call_factory_create_pair "$FACTORY_PKG" "$WCSPR_PKG" "$WBTC_PKG")"
  echo "PAIR_TX_WCSPR_WBTC=$PAIR_TX_WCSPR_WBTC" >> "$out_env"

  PAIR_TX_USDC_ECTO="$(call_factory_create_pair "$FACTORY_PKG" "$USDC_PKG" "$ECTO_PKG")"
  echo "PAIR_TX_USDC_ECTO=$PAIR_TX_USDC_ECTO" >> "$out_env"

  PAIR_TX_USDC_WETH="$(call_factory_create_pair "$FACTORY_PKG" "$USDC_PKG" "$WETH_PKG")"
  echo "PAIR_TX_USDC_WETH=$PAIR_TX_USDC_WETH" >> "$out_env"

  PAIR_TX_USDC_WBTC="$(call_factory_create_pair "$FACTORY_PKG" "$USDC_PKG" "$WBTC_PKG")"
  echo "PAIR_TX_USDC_WBTC=$PAIR_TX_USDC_WBTC" >> "$out_env"
fi

log "Done."
