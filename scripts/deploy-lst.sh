#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/deploy-lst.sh [--build]

Environment variables (recommended via .env):
  NODE_ADDRESS            Node RPC URL for casper-client.
  CHAIN_NAME              Chain name. Example: casper-test
  SECRET_KEY_PATH         Path to secret key pem (for signing).
  DEPLOYER_ACCOUNT_HASH   account-hash-... used for admin calls.

Optional:
  GAS_PRICE_TOLERANCE     Default: 1
  TX_WAIT_TRIES           Default: 180
  TX_WAIT_SLEEP_S         Default: 5
  PAYMENT_TOKEN           Default: 600000000000
  PAYMENT_CALL            Default: 300000000000

What it does:
  - Deploys sCSPR token + StakingManager
  - Initializes StakingManager with sCSPR package hash
  - Updates sCSPR staking manager to deployed StakingManager
  - Writes hashes to scripts/deploy-lst.out.env
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

# Load .env if present (no failure if absent).
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
SECRET_KEY_PATH="${SECRET_KEY_PATH:-keys/secret_key_pkcs8.pem}"
DEPLOYER_ACCOUNT_HASH="${DEPLOYER_ACCOUNT_HASH:-}"

GAS_PRICE_TOLERANCE="${GAS_PRICE_TOLERANCE:-1}"
TX_WAIT_TRIES="${TX_WAIT_TRIES:-180}"
TX_WAIT_SLEEP_S="${TX_WAIT_SLEEP_S:-5}"
PAYMENT_TOKEN="${PAYMENT_TOKEN:-600000000000}"
PAYMENT_CALL="${PAYMENT_CALL:-300000000000}"

BUILD=0
for arg in "$@"; do
  case "$arg" in
    --build) BUILD=1 ;;
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

if [[ -z "$NODE_ADDRESS" || -z "$CHAIN_NAME" || -z "$DEPLOYER_ACCOUNT_HASH" ]]; then
  echo "Missing required env vars." >&2
  echo "Required: NODE_ADDRESS, CHAIN_NAME, DEPLOYER_ACCOUNT_HASH" >&2
  exit 2
fi

if ! is_valid_account_hash "$DEPLOYER_ACCOUNT_HASH"; then
  echo "DEPLOYER_ACCOUNT_HASH must look like: account-hash-<64 hex chars>" >&2
  echo "Got: $DEPLOYER_ACCOUNT_HASH" >&2
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
  casper-client "$@" | sed -n '/^[[:space:]]*[{[]/,$p'
}

extract_tx_hash() {
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
  local package_hash="$1"
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

  if [[ "$contract_hash" == contract-* ]]; then
    printf 'hash-%s\n' "${contract_hash#contract-}"
  elif [[ "$contract_hash" == hash-* ]]; then
    printf '%s\n' "$contract_hash"
  else
    echo "$contract_hash"
  fi
}

wait_tx() {
  local tx="$1"
  local max_tries="${2:-$TX_WAIT_TRIES}"
  local sleep_s="${3:-$TX_WAIT_SLEEP_S}"

  for ((i=1; i<=max_tries; i++)); do
    local json
    if ! json="$(casper_json get-transaction --node-address "$NODE_ADDRESS" "$tx" 2>/dev/null)"; then
      sleep "$sleep_s"
      continue
    fi

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
  if ! casper-client get-state-root-hash --node-address "$NODE_ADDRESS" >/dev/null 2>&1; then
    echo "Unable to reach node RPC at NODE_ADDRESS=$NODE_ADDRESS" >&2
    exit 2
  fi

  if ! casper-client get-account --node-address "$NODE_ADDRESS" --account-identifier "$DEPLOYER_ACCOUNT_HASH" >/dev/null 2>&1; then
    echo "Unable to fetch deployer account. Check DEPLOYER_ACCOUNT_HASH and funding." >&2
    exit 2
  fi
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

call_entry_point() {
  local package_hash="$1"
  local entry_point="$2"
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

if [[ $BUILD -eq 1 ]]; then
  log "==> Building WASM (cargo odra build)"
  (cd "$repo_root" && cargo odra build)
fi

for f in ScsprToken.wasm StakingManager.wasm; do
  if [[ ! -f "$wasm_dir/$f" ]]; then
    echo "Missing wasm: $wasm_dir/$f" >&2
    echo "Run: cargo odra build" >&2
    exit 2
  fi
done

preflight

out_env="$repo_root/scripts/deploy-lst.out.env"
out_env_tmp="$repo_root/scripts/.deploy-lst.out.env.tmp"
rm -f "$out_env_tmp"

# 1) Deploy sCSPR token (temporary staking manager = deployer)
SCSPR_PKG="$(get_named_key scspr_token_package_hash || true)"
if [[ -z "${SCSPR_PKG:-}" || "$SCSPR_PKG" == "null" ]]; then
  deploy_session_install "sCSPR Token" "$wasm_dir/ScsprToken.wasm" "scspr_token_package_hash" \
    --session-arg "staking_manager:key:'$DEPLOYER_ACCOUNT_HASH'"
  SCSPR_PKG="$(get_named_key scspr_token_package_hash)"
else
  log "==> sCSPR Token already deployed: $SCSPR_PKG"
fi
SCSPR_CONTRACT="$(active_contract_hash_from_package "$SCSPR_PKG")"

# 2) Deploy StakingManager, passing sCSPR package hash
STAKING_MANAGER_PKG="$(get_named_key staking_manager_package_hash || true)"
if [[ -z "${STAKING_MANAGER_PKG:-}" || "$STAKING_MANAGER_PKG" == "null" ]]; then
  deploy_session_install "Staking Manager" "$wasm_dir/StakingManager.wasm" "staking_manager_package_hash" \
    --session-arg "scspr_token_address:key:'$SCSPR_PKG'"
  STAKING_MANAGER_PKG="$(get_named_key staking_manager_package_hash)"
else
  log "==> Staking Manager already deployed: $STAKING_MANAGER_PKG"
fi
STAKING_MANAGER_CONTRACT="$(active_contract_hash_from_package "$STAKING_MANAGER_PKG")"

# 3) Wire sCSPR token to staking manager
call_entry_point "$SCSPR_PKG" set_staking_manager \
  --session-arg "new_manager:key:'$STAKING_MANAGER_PKG'"

{
  echo "NODE_ADDRESS=$NODE_ADDRESS"
  echo "CHAIN_NAME=$CHAIN_NAME"
  echo "DEPLOYER_ACCOUNT_HASH=$DEPLOYER_ACCOUNT_HASH"
  echo
  echo "SCSPR_PACKAGE_HASH=$SCSPR_PKG"
  echo "SCSPR_CONTRACT_HASH=$SCSPR_CONTRACT"
  echo "STAKING_MANAGER_PACKAGE_HASH=$STAKING_MANAGER_PKG"
  echo "STAKING_MANAGER_CONTRACT_HASH=$STAKING_MANAGER_CONTRACT"
} > "$out_env_tmp"

mv "$out_env_tmp" "$out_env"

echo "✅ LST deployment complete"
echo "SCSPR_PACKAGE_HASH=$SCSPR_PKG"
echo "SCSPR_CONTRACT_HASH=$SCSPR_CONTRACT"
echo "STAKING_MANAGER_PACKAGE_HASH=$STAKING_MANAGER_PKG"
echo "STAKING_MANAGER_CONTRACT_HASH=$STAKING_MANAGER_CONTRACT"
echo "Saved to: $out_env"
