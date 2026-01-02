# Casper v5 Verification Commands (Ectoplasm DEX)

This repo ships scripts + copy/paste commands to verify DEX lifecycle transactions on Casper **v5+** (`put-transaction`):

- Deploy → create_pair → mint → approve → add_liquidity → swap → remove_liquidity
- Verify success/failure from receipts (`error_message`)
- Prove balance deltas across a swap from Transfer events

## 0) Prereqs

- `casper-client` (v5+)
- `jq`
- `python3`

## 1) Load environment (local or testnet)

`deploy-new.sh` reads `.env` and produces `scripts/deploy-new.out.env`.

```bash
cd /home/ayush99336/Desktop/Code/hackathon/ectoplasm-contracts/ectoplasm-contracts

# Load your node/account settings + last deploy outputs
set -a
source .env
source scripts/deploy-new.out.env
set +a

# Quick sanity
echo "$NODE_ADDRESS"
echo "$CHAIN_NAME"
echo "$DEPLOYER_ACCOUNT_HASH"
```

## 2) Deploy contracts (+ optionally create pairs)

```bash
# Build wasm (optional) + deploy tokens + dex + create common pairs
scripts/deploy-new.sh --build --create-pairs

# If you already built wasm, skip --build
scripts/deploy-new.sh --create-pairs
```

After it finishes, re-load outputs:

```bash
set -a
source scripts/deploy-new.out.env
set +a
```

## 3) Check a transaction result (Casper v5)

Given a **transaction hash**:

```bash
TX=<tx_hash>

casper-client get-transaction --node-address "$NODE_ADDRESS" "$TX" \
  | sed -n '/^[[:space:]]*[{[]/,$p' \
  | jq -r '
    .result.transaction.execution_info.execution_result.Version2.error_message
    // .result.transaction.execution_info.execution_result.Version1.error_message
    // .result.execution_info.execution_result.Version2.error_message
    // .result.execution_info.execution_result.Version1.error_message
    // empty
  '
```

- If it prints nothing / `null` → success.
- If it prints something like `User error: 3` → revert (see Troubleshooting).

To inspect cost/consumed gas + effects count:

```bash
TX=<tx_hash>

casper-client get-transaction --node-address "$NODE_ADDRESS" "$TX" \
  | sed -n '/^[[:space:]]*[{[]/,$p' \
  | jq '
    .result.transaction.execution_info.execution_result.Version2
    // .result.transaction.execution_info.execution_result.Version1
    // .result.execution_info.execution_result.Version2
    // .result.execution_info.execution_result.Version1
    | {
        consumed: .consumed,
        cost: .cost,
        effects: (.effects | length)
      }
  '
```

Legacy (pre-v5) deploy hash (optional):

```bash
DEPLOY=<deploy_hash>
casper-client get-deploy --node-address "$NODE_ADDRESS" "$DEPLOY" \
  | sed -n '/^[[:space:]]*[{[]/,$p' \
  | jq -r '.result.execution_info.execution_result.error_message // empty'
```

## 4) Prove `create_pair` actually deployed a Pair contract

If you ran `scripts/deploy-new.sh --create-pairs`, `scripts/deploy-new.out.env` contains e.g. `PAIR_TX_WCSPR_ECTO=<tx>`.

Extract the **Pair package hash** written by that `create_pair` transaction:

```bash
TX="$PAIR_TX_WCSPR_ECTO"

casper-client get-transaction --node-address "$NODE_ADDRESS" "$TX" \
  | sed -n '/^[[:space:]]*[{[]/,$p' \
  | jq -r '
    .result.execution_info.execution_result.Version2.effects[]
    | select(.kind|type=="object")
    | select(.kind.Write?.ContractPackage? != null)
    | .key
  ' \
  | head -n 1
```

If this prints `hash-...`, you have an on-chain Pair package created for that pair.

## 5) Run the full AMM lifecycle smoke test

This will:

- mint WCSPR + ECTO
- approve Router to spend both tokens
- add liquidity
- swap
- approve Router to spend LP token (Pair)
- remove liquidity

```bash
scripts/test-dex.sh
```

## 6) Verify swap balance deltas (proof via Transfer events)

`scripts/check-balances-at-swap.sh` computes deployer balance before/after a swap by replaying Transfer events at the parent and current state-roots of the swap block.

It accepts either a **v5 transaction hash** or a legacy **deploy hash**.

```bash
SWAP_TX=<tx_hash>

scripts/check-balances-at-swap.sh "$SWAP_TX"
```

Output includes:

- state root (before swap)
- state root (after swap)
- WCSPR / ECTO balances and deltas (raw U256)

## 7) Common troubleshooting

### `User error: 3` (DexError::InsufficientOutputAmount)

- Meaning: `amount_out_min` was too high for the pool price.
- Fix:
  - For testing, set `amount_out_min` to `0`.
  - Or quote first and set a realistic min (slippage tolerance).

### `User error: 9` (DexError::InsufficientAmount)

- Meaning: one of the `amount_*_min` constraints was too strict for the actual pool ratio.
- Fix:
  - Set `amount_a_min` / `amount_b_min` lower (often `0` for dev testing).

### `User error: 100` (TokenError::InsufficientAllowance)

- Meaning: missing/insufficient approval.
- Fix:
  - Before `add_liquidity` / `swap`: approve **Router** as spender on both input tokens.
  - Before `remove_liquidity`: approve **Router** as spender on the **LP token** (the Pair contract).
  - `scripts/test-dex.sh` does both approvals.

### `Tracking copy error: Value not found`

Common causes in this repo:

- Passing a **contract hash** where the DEX expects a **package hash** (or vice versa).
- Router/Factory flows here use token **package hashes** (`WCSPR_PACKAGE_HASH`, `ECTO_PACKAGE_HASH`) as `Key` arguments.

### Lane / payment issues

- If you see a lane selection error (e.g. “couldn't associate a transaction lane”), payment can be too high for that lane.
- If you see out-of-gas, payment is too low.

This repo defaults (override via env) in `scripts/deploy-new.sh`:

- `PAYMENT_CREATE_PAIR=900000000000`

## 8) Useful env vars (from `scripts/deploy-new.out.env`)

- `*_PACKAGE_HASH` are used as `--contract-package-hash` targets.
- `*_CONTRACT_HASH` are resolved from the package hash (active version).
- `PAIR_TX_*` are the create_pair transaction hashes (useful for extracting Pair package hashes).
