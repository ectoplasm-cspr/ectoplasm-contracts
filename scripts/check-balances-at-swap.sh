#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_env="$repo_root/scripts/deploy-new.out.env"
env_file="$repo_root/.env"

if [[ ! -f "$out_env" ]]; then
  echo "Error: $out_env not found. Run scripts/deploy-new.sh first." >&2
  exit 1
fi

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

require() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }
}

require casper-client
require jq
require python3

casper_json() {
  casper-client "$@" | sed -n '/^[[:space:]]*[{[]/,$p'
}

swap_id="${1:-}"
if [[ -z "$swap_id" ]]; then
  echo "Usage: $0 <swap_tx_or_deploy_hash>" >&2
  exit 2
fi

# 1) Find swap block (transaction hash OR deploy hash).
swap_block=""

if tx_json="$(casper_json get-transaction --node-address "$NODE_ADDRESS" "$swap_id" 2>/dev/null)"; then
  swap_block="$(echo "$tx_json" | jq -r '.result.transaction.execution_info.block_hash // .result.execution_info.block_hash // empty')"
fi

if [[ -z "$swap_block" ]]; then
  deploy_json="$(casper_json get-deploy --node-address "$NODE_ADDRESS" "$swap_id")"
  swap_block="$(echo "$deploy_json" | jq -r '.result.execution_info.block_hash // empty')"
fi

if [[ -z "$swap_block" || "$swap_block" == "null" ]]; then
  echo "Error: could not resolve block hash for $swap_id" >&2
  exit 1
fi

# 2) Resolve pre/post swap state roots from swap block + parent block.
block_json="$(casper_json get-block --node-address "$NODE_ADDRESS" --block-identifier "$swap_block")"
state_after="$(echo "$block_json" | jq -r '.result.block_with_signatures.block.Version2.header.state_root_hash // .result.block_with_signatures.block.Version1.header.state_root_hash')"
parent_hash="$(echo "$block_json" | jq -r '.result.block_with_signatures.block.Version2.header.parent_hash // .result.block_with_signatures.block.Version1.header.parent_hash')"
parent_json="$(casper_json get-block --node-address "$NODE_ADDRESS" --block-identifier "$parent_hash")"
state_before="$(echo "$parent_json" | jq -r '.result.block_with_signatures.block.Version2.header.state_root_hash // .result.block_with_signatures.block.Version1.header.state_root_hash')"

named_key() {
  local contract_hash="$1"
  local key_name="$2"
  local state_root="$3"
  casper_json query-global-state --node-address "$NODE_ADDRESS" --state-root-hash "$state_root" --key "$contract_hash" \
    | jq -r ".result.stored_value.Contract.named_keys[] | select(.name==\"$key_name\") | .key"
}

cl_parsed() {
  local state_root="$1"
  local uref="$2"
  casper_json query-global-state --node-address "$NODE_ADDRESS" --state-root-hash "$state_root" --key "$uref" \
    | jq -r '.result.stored_value.CLValue.parsed'
}

balance_from_events() {
  local token_label="$1"
  local contract_hash="$2"
  local state_root="$3"

  local events_uref events_len_uref len
  events_uref="$(named_key "$contract_hash" "__events" "$state_root")"
  events_len_uref="$(named_key "$contract_hash" "__events_length" "$state_root")"
  len="$(cl_parsed "$state_root" "$events_len_uref")"

  # Fetch all event byte arrays into JSON list-of-lists, then decode and sum in Python.
  # This avoids doing parsing logic in bash.
  python3 - "$NODE_ADDRESS" "$state_root" "$events_uref" "$len" "$DEPLOYER_ACCOUNT_HASH" "$token_label" <<'PY'
import json
import subprocess
import sys

node, state_root, events_uref, length_s, deployer, label = sys.argv[1:7]
length = int(length_s)

def casper_json(*args: str) -> dict:
    p = subprocess.run(["casper-client", *args], capture_output=True, text=True, check=True)
    # strip any non-JSON preamble
    lines = p.stdout.splitlines()
    start = 0
    for idx, line in enumerate(lines):
        if line.lstrip().startswith("{") or line.lstrip().startswith("["):
            start = idx
            break
    return json.loads("\n".join(lines[start:]))

def decode_event_u8_list(arr):
    i = 0
    if len(arr) < 4:
        return None
    name_len = int.from_bytes(bytes(arr[i:i+4]), "little"); i += 4
    name = bytes(arr[i:i+name_len]).decode(errors="replace"); i += name_len

    def read_addr():
        nonlocal i
        tag = arr[i]; i += 1
        b = bytes(arr[i:i+32]); i += 32
        if tag == 0:
            return "account-hash-" + b.hex()
        if tag == 1:
            return "hash-" + b.hex()
        return f"tag{tag}-" + b.hex()

    from_addr = read_addr()
    to_addr = read_addr()
    n = arr[i]; i += 1
    val = int.from_bytes(bytes(arr[i:i+n]), "little"); i += n
    return {"name": name, "from": from_addr, "to": to_addr, "value": val}

balance = 0
for idx in range(length):
    j = casper_json(
        "get-dictionary-item",
        "--node-address", node,
        "--state-root-hash", state_root,
        "--seed-uref", events_uref,
        "--dictionary-item-key", str(idx),
    )
    arr = j["result"]["stored_value"]["CLValue"]["parsed"]
    ev = decode_event_u8_list(arr)
    if not ev or ev.get("name") != "event_Transfer":
        continue

    if ev.get("to") == deployer:
        balance += int(ev["value"])
    if ev.get("from") == deployer:
        balance -= int(ev["value"])

print(f"{label}:{balance}")
PY
}

wc_before="$(balance_from_events WCSPR "$WCSPR_CONTRACT_HASH" "$state_before")"
wc_after="$(balance_from_events WCSPR "$WCSPR_CONTRACT_HASH" "$state_after")"
ec_before="$(balance_from_events ECTO "$ECTO_CONTRACT_HASH" "$state_before")"
ec_after="$(balance_from_events ECTO "$ECTO_CONTRACT_HASH" "$state_after")"

wb="${wc_before#*:}"; wa="${wc_after#*:}"
eb="${ec_before#*:}"; ea="${ec_after#*:}"

wc_delta="$(python3 - <<PY
wb=int("$wb")
wa=int("$wa")
print(wa-wb)
PY
)"
ec_delta="$(python3 - <<PY
eb=int("$eb")
ea=int("$ea")
print(ea-eb)
PY
)"

cat <<EOF
Swap id: $swap_id
State root (before swap): $state_before
State root (after swap):  $state_after

Balances for $DEPLOYER_ACCOUNT_HASH (raw, token decimals=18):
  WCSPR before: $wb
  WCSPR after:  $wa
  WCSPR delta:  $wc_delta

  ECTO before:  $eb
  ECTO after:   $ea
  ECTO delta:   $ec_delta
EOF
