# Local Casper Blockchain Deployment & Verification Guide

This guide details how to set up a local testing environment, deploy the Ectoplasm DEX contracts, and verify their functionality using `casper-nctl` and the provided scripts.

## 1. Prerequisites

Ensure you have the following tools installed:
- **Docker**: For running the local Casper node.
- **Rust & Cargo**: For building the smart contracts.
- **Odra CLI**: `cargo install odra-cli` (v2.5.0 compatible).
- **Casper Client**: `casper-client` (v2.0+ or v5.0+ recommended) for interacting with the node.
- **jq**: For parsing JSON output in scripts.

## 2. Start Local Network (NCTL)

Run the following Docker command to start a standalone local Casper network. This container (`makesoftware/casper-nctl`) pre-funds accounts and sets up a node.

```bash
docker run --rm -it --name mynctl -d \
  -p 11101:11101 \
  -p 14101:14101 \
  -p 18101:18101 \
  makesoftware/casper-nctl
```

- **Port 11101**: RPC API (Main interface for client)
- **Port 14101**: REST API
- **Port 18101**: SSE (Event stream)

Verify the node is running:
```bash
casper-client get-node-status --node-address http://127.0.0.1:11101
```

## 3. Configure Keys & Environment

To deploy contracts, you need a funded account. The NCTL container generates these for you.

### 3.1 Extract Keys
Copy the **Faucet** account keys (which holds unlimited funds for testing) from the docker container to your project's `keys/` directory.

```bash
mkdir -p keys
docker cp mynctl:/home/casper/casper-nctl/assets/net-1/faucet/secret_key.pem keys/secret_key.pem
docker cp mynctl:/home/casper/casper-nctl/assets/net-1/faucet/public_key.pem keys/public_key.pem
```

### 3.2 Update .env
Ensure your `.env` file matches the local node configuration.

**File: `.env`**
```env
NODE_ADDRESS=http://127.0.0.1:11101
CHAIN_NAME=casper-net-1
SECRET_KEY_PATH=keys/secret_key.pem
# Derive this hash using: casper-client account-address --public-key keys/public_key.pem
DEPLOYER_ACCOUNT_HASH=account-hash-<YOUR_GENERATED_HASH>
# Gas Settings
PAYMENT_FACTORY=1500000000000  # 1500 CSPR (Required for PairFactory)
```

## 4. Build Contracts

Compile the Odra contracts to WASM modules. This uses the configuration in `Odra.toml`.

```bash
cargo odra build
```

**Output**: New verify `.wasm` files are created in the `wasm/` directory (e.g., `Factory.wasm`, `PairFactory.wasm`, `Router.wasm`, `LpToken.wasm`).

## 5. Deploy & Verify

Use the automated `scripts/deploy-new.sh` script to deploy all contracts and execute initial verification steps.

```bash
# Deploys Tokens, PairFactory, Factory, Router, and calls create_pair
./scripts/deploy-new.sh --create-pairs
```
<!-- ./scripts/deploy-new.sh --build --create-pairs -->


### What the script does:
1.  **Deploys Tokens**: `WCSPR`, `ECTO`, `USDC`, etc.
2.  **Deploys PairFactory**: Installs the factory logic generator.
3.  **Deploys Factory**: Installs the main Factory and links it to `PairFactory`.
4.  **Deploys Router**: Installs the Router linked to `Factory` and `WCSPR`.
5.  **Verifies `create_pair`**: Calls `Factory.create_pair` for multiple token combinations directly on the blockchain.

**Success Condition**: The script finishes with `Done.` and exit code `0`.

## 6. Verification Scripts

We provide `scripts/test-dex.sh` to verifying complex interactions:

```bash
./scripts/test-dex.sh
```

This script performs:
1.  **Approvals**: Approves Router to spend Tokens (WCSPR, ECTO).
2.  **Add Liquidity**: Adds initial liquidity to the WCSPR-ECTO pair.
3.  **Swap**: Swaps WCSPR for ECTO.

## 7. Manual Transaction Execution & Verification

If you need to execute manual transactions (e.g., for debugging), use the `casper-client`.

### Example: Deploying a contract manually
```bash
casper-client put-transaction session \
    --node-address "http://127.0.0.1:11101" \
    --chain-name "casper-net-1" \
    --secret-key "keys/secret_key.pem" \
    --wasm-path "wasm/YourContract.wasm" \
    --payment-amount "100000000000" \
    --gas-price-tolerance "1" \
    --standard-payment true \
    --install-upgrade \
    --session-arg "odra_cfg_package_hash_key_name:string:'your_pkg_hash'" \
    --session-arg "odra_cfg_allow_key_override:bool:'true'" \
    --session-arg "odra_cfg_is_upgradable:bool:'true'" \
    --session-arg "odra_cfg_is_upgrade:bool:'false'"
```

### Verify Transaction Status
Get the transaction hash from the output and check its status:

```bash
casper-client get-transaction \
  --node-address http://127.0.0.1:11101 \
  <TRANSACTION_HASH>
```

Look for `"execution_result": { "Success": ... }`.

### Query Contract State
To check state (e.g., see if a Pair was actually registered in the Factory):

```bash
# 1. Get the deployer's account to find the verification contract hash
casper-client get-account \
  --node-address http://127.0.0.1:11101 \
  --account-identifier <DEPLOYER_ACCOUNT_HASH>
```
