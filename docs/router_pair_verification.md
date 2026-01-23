# Router & Pair State Verification

This document explains how to verify the Router is properly initialized and how to detect pairs on-chain for frontend integration.

## Prerequisites

```bash
source scripts/deploy-new.out.env
STATE_ROOT=$(casper-client get-state-root-hash --node-address $NODE_ADDRESS | jq -r '.result.state_root_hash')
```

---

## 1. Router Initialization

### Contract Structure
```rust
#[odra::module]
pub struct Router {
    factory: Var<Address>,  // Factory contract address
    wcspr: Var<Address>,    // Wrapped CSPR token address
}
```

### Initialization Flow
```
deploy-new.sh
    │
    ├── deploy_router(FACTORY_PKG, WCSPR_PKG)
    │       │
    │       └── Odra auto-calls Router.init(factory, wcspr)
    │
    └── Router.factory = FACTORY_PACKAGE_HASH
        Router.wcspr = WCSPR_PACKAGE_HASH
```

### Verification
The Router's `state` dictionary stores these values. If not initialized:
- Any call to `add_liquidity` or `swap_*` will revert with `DexError::InvalidPair`

**Proof of initialization:** A successful swap transaction confirms Router has valid Factory reference.

---

## 2. Pair Detection (Factory)

### Contract Structure
```rust
#[odra::module]
pub struct Factory {
    pairs: Mapping<(Address, Address), Address>,  // (tokenA, tokenB) → pair
    all_pairs: Mapping<u32, Address>,             // index → pair
    all_pairs_length: Var<u32>,                   // total count
}
```

### How Pairs Are Stored

| Named Key | Type | Purpose |
|-----------|------|---------|
| `__events` | Dictionary | PairCreated events |
| `__events_length` | U32 | Pair count |
| `state` | Dictionary | Odra module fields |

### Query Pair Count
```bash
# Get events length URef from Factory named keys
casper-client query-global-state \
  --node-address $NODE_ADDRESS \
  --state-root-hash "$STATE_ROOT" \
  --key "$FACTORY_CONTRACT_HASH" \
  | jq '.result.stored_value.Contract.named_keys[] | select(.name=="__events_length") | .key'

# Query the count
casper-client query-global-state \
  --node-address $NODE_ADDRESS \
  --state-root-hash "$STATE_ROOT" \
  --key "$EVENTS_LEN_UREF" \
  | jq '.result.stored_value.CLValue.parsed'
```

### List All Pairs (from events)
```bash
for idx in 0 1 2; do
  casper-client get-dictionary-item \
    --node-address $NODE_ADDRESS \
    --state-root-hash "$STATE_ROOT" \
    --seed-uref "$EVENTS_UREF" \
    --dictionary-item-key "$idx"
done
```

---

## 3. Frontend Integration

### Swap Flow
```
┌─────────────────────────────────────────────────────────┐
│ User selects: TokenA → TokenB                           │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Frontend: Call Factory.get_pair(tokenA, tokenB)         │
│           → Returns pair address or None                │
└─────────────────────────────────────────────────────────┘
                         │
            ┌────────────┴────────────┐
            ▼                         ▼
    ┌───────────────┐         ┌───────────────┐
    │ Pair exists   │         │ No pair found │
    │ Show reserves │         │ "Create pair" │
    └───────────────┘         └───────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────┐
│ User clicks "Swap"                                      │
│ Frontend: Router.swap_exact_tokens_for_tokens(          │
│   amount_in, amount_out_min,                            │
│   path=[tokenA, tokenB],                                │
│   to, deadline                                          │
│ )                                                       │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Router internally:                                      │
│   1. Calls Factory.get_pair(tokenA, tokenB)             │
│   2. Gets reserves from Pair contract                   │
│   3. Calculates output amount                           │
│   4. Executes swap on Pair                              │
└─────────────────────────────────────────────────────────┘
```

### Entry Points

| Contract | Entry Point | Args | Returns |
|----------|-------------|------|---------|
| Factory | `get_pair` | `token_a: Key, token_b: Key` | `Option<Key>` |
| Factory | `all_pairs_length` | - | `u32` |
| Router | `factory` | - | `Key` |
| Router | `wcspr` | - | `Key` |

---

## 4. Verification Script

Run `scripts/verify-dex-state.sh` to check:
- Router has `state` dictionary
- Factory has pairs created
- List all pair addresses

```bash
./scripts/verify-dex-state.sh
```

---

## 5. Frontend Pair Configuration

After creating a pair via `Router.add_liquidity`, you need to configure the frontend to recognize it.

### Step 1: Get Pair Address from Factory Events

```bash
# Set environment
source scripts/testnet-deploy.out.env
STATE_ROOT=$(casper-client get-state-root-hash --node-address $NODE_ADDRESS | jq -r '.result.state_root_hash')

# Get __events URef from Factory
EVENTS_UREF=$(casper-client query-global-state \
  --node-address $NODE_ADDRESS \
  --state-root-hash "$STATE_ROOT" \
  --key "$FACTORY_CONTRACT_HASH" | \
  jq -r '.result.stored_value.Contract.named_keys[] | select(.name=="__events") | .key')

# Query pair creation event (index 0 = first pair)
casper-client get-dictionary-item \
  --node-address $NODE_ADDRESS \
  --state-root-hash "$STATE_ROOT" \
  --seed-uref "$EVENTS_UREF" \
  --dictionary-item-key "0" 2>&1 | \
sed -n '/^[[:space:]]*{/,$p' | python3 -c "
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
print(f'token0: hash-{token0}')
print(f'token1: hash-{token1}')
print(f'PAIR_HASH=hash-{pair}')
"
```

### Step 2: Add to Frontend `.env`

Add the pair hash to `ectoplasm-react/.env`:

```bash
# WCSPR-ECTO Pair
WCSPR_ECTO_PAIR_HASH=hash-22b993991e48349c7344f03515bf53573f3b22dea95390d4b8626305a3e682f9
```

### Step 3: Restart Frontend

```bash
# Restart dev server to pick up new env variable
cd ectoplasm-react
npm run dev
```

### How Frontend Uses Pair Hash

The config reads `WCSPR_ECTO_PAIR_HASH` from env in `config/ectoplasm.ts`:

```typescript
const ODRA_CONTRACTS: ContractsConfig = {
  factory: envGet('FACTORY_CONTRACT_HASH') || '',
  router: envGet('ROUTER_CONTRACT_HASH') || '',
  pairs: {
    'WCSPR/ECTO': envGet('WCSPR_ECTO_PAIR_HASH') || '',
  },
};
```

When swapping, `CasperService.getPairAddress()` first checks configured pairs before querying the blockchain.

---

## Key Points

1. **Router stores Factory reference** in its `state` dictionary via `Var<Address>`
2. **Pairs are created automatically** by `Router.add_liquidity` if they don't exist
3. **Factory emits `PairCreated` events** stored in `__events` dictionary
4. **Token sorting is automatic** - `(A,B)` and `(B,A)` return same pair
5. **Successful swap = Router is initialized** - reverts otherwise
