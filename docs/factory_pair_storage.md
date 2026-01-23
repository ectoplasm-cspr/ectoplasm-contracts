# Factory Pair Storage & Lookup Guide

This document explains how token pairs are stored in the Factory contract and how to query them using `casper-client`.

## Prerequisites

Source the deployment environment:
```bash
source scripts/deploy-new.out.env
```

Get current state root:
```bash
STATE_ROOT=$(casper-client get-state-root-hash --node-address http://localhost:11101 | jq -r '.result.state_root_hash')
```

---

## Factory Contract Structure

### Named Keys (Odra Storage)

Query Factory contract to see named keys:
```bash
casper-client query-global-state \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --key "$FACTORY_CONTRACT_HASH" \
  | jq '.result.stored_value.Contract.named_keys'
```

**Output:**
| Key | Type | Purpose |
|-----|------|---------|
| `__events` | Dictionary URef | PairCreated events |
| `__events_length` | U32 URef | Event count (= pair count) |
| `state` | Dictionary URef | Odra module state |

### Odra Module Fields

```rust
pub struct Factory {
    fee_to_setter: Var<Address>,                         // Admin address
    pair_factory:  Var<Address>,                         // PairFactory contract
    fee_to:        Var<Option<Address>>,                 // Fee recipient
    pairs:         Mapping<(Address, Address), Address>, // (token0, token1) -> pair
    all_pairs:     Mapping<u32, Address>,                // index -> pair
    all_pairs_length: Var<u32>,                          // Total pair count
}
```

---

## Query Total Pairs Count

```bash
# Find events_length URef from named_keys, then query it
EVENTS_LEN_UREF="uref-7759f0a60258e0081efd9809a7dc1aea42707e09186debdb6f01283004277671-007"

casper-client query-global-state \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --key "$EVENTS_LEN_UREF" \
  | jq '.result.stored_value.CLValue.parsed'
```

**Output:** `7` (number of pairs created)

---

## Query PairCreated Events

Each pair creation emits an event stored in the `__events` dictionary.

### Get Events Dictionary URef
```bash
EVENTS_UREF="uref-4eb7b399318d6c0494cf46301abcc73eebff5e53e0e5c9c94f8498597720281a-007"
```

### Query Single Event
```bash
casper-client get-dictionary-item \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --seed-uref "$EVENTS_UREF" \
  --dictionary-item-key "0"
```

### Decode All Events (Python)
```bash
for idx in 0 1 2 3 4 5 6; do
  casper-client get-dictionary-item \
    --node-address http://localhost:11101 \
    --state-root-hash "$STATE_ROOT" \
    --seed-uref "$EVENTS_UREF" \
    --dictionary-item-key "$idx" 2>&1 | \
  sed -n '/^[[:space:]]*[{[]/,$p' | python3 -c "
import sys, json
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
print(f'Pair {pair_count}: hash-{token0[:8]}... <-> hash-{token1[:8]}... => Pair hash-{pair[:12]}...')
"
done
```

**Example Output:**
```
Pair 1: hash-1cdb5bb4... <-> hash-7431b5d4... => Pair hash-37ca74ca4746...
Pair 2: hash-58a2591b... <-> hash-7431b5d4... => Pair hash-8f9b02f92829...
Pair 3: hash-275498ac... <-> hash-7431b5d4... => Pair hash-16f778aea43d...
...
```

---

## Event Data Structure

The `PairCreated` event is encoded as `List<U8>`:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | name_len | Event name length (little endian) |
| 4 | name_len | name | "event_PairCreated" |
| +0 | 1 | tag0 | Address type (1 = hash) |
| +1 | 32 | token0 | First token contract hash |
| +33 | 1 | tag1 | Address type |
| +34 | 32 | token1 | Second token contract hash |
| +66 | 1 | tag2 | Address type |
| +67 | 32 | pair | Pair contract hash |
| +99 | 4 | pair_count | Sequential pair number |

---

## How Pair Lookup Works

### Token Sorting
Factory always sorts tokens before storage to ensure `(A, B)` and `(B, A)` map to the same pair:

```rust
fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
    if token_a < token_b { (token_a, token_b) } else { (token_b, token_a) }
}
```

### Lookup Flow
```
User: swap(WCSPR -> ECTO)
         │
         ▼
Router.swap_exact_tokens_for_tokens(path=[WCSPR, ECTO])
         │
         ▼
Factory.get_pair(WCSPR_CONTRACT_HASH, ECTO_CONTRACT_HASH)
         │
         ├── Sorts: (ECTO < WCSPR) → key = (ECTO, WCSPR)
         │
         ├── Lookup: pairs[(ECTO, WCSPR)] → Pair address
         │
         ▼
Router executes swap on Pair contract
```

---

## Entry Points

| Entry Point | Args | Returns | Description |
|-------------|------|---------|-------------|
| `create_pair` | `token_a: Key, token_b: Key` | `Key` | Create new pair |
| `get_pair` | `token_a: Key, token_b: Key` | `Option<Key>` | Lookup pair |
| `pair_exists` | `token_a: Key, token_b: Key` | `bool` | Check existence |
| `all_pairs_at` | `index: u32` | `Option<Key>` | Get pair by index |
| `all_pairs_length` | - | `u32` | Total pairs |

---

## Quick Reference

```bash
# Source environment
source scripts/deploy-new.out.env

# Get state root
STATE_ROOT=$(casper-client get-state-root-hash --node-address http://localhost:11101 | jq -r '.result.state_root_hash')

# Query Factory contract
casper-client query-global-state \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --key "$FACTORY_CONTRACT_HASH"

# Get pair count
casper-client query-global-state \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --key "$EVENTS_LEN_UREF" | jq '.result.stored_value.CLValue.parsed'

# Query specific event
casper-client get-dictionary-item \
  --node-address http://localhost:11101 \
  --state-root-hash "$STATE_ROOT" \
  --seed-uref "$EVENTS_UREF" \
  --dictionary-item-key "0"
```



pub struct Factory {
    fee_to: Var<Option<Address>>,        // Index 0-1 (Option takes 2 indices!)
    fee_to_setter: Var<Address>,         // Index 2
    pair_factory: Var<Address>,          // Index 3
    pairs: Mapping<...>,                 // Index 4
    all_pairs: Mapping<u32, Address>,    // Index 5 ✅
    all_pairs_length: Var<u32>,          // Index 6 ✅
}

Key Discovery: Var<Option<T>> takes 2 storage indices instead of 1!