# Contracts Deployment Guide

This guide explains how to deploy the Ectoplasm DEX contracts using the Odra CLI.

## Prerequisites

- Rust and Cargo installed
- Casper Client installed (`casper-client`)
- Contract WASM files built and located in `wasm/` directory

## Environment Setup

The deployment requires an environment configuration file compatible with Odra. We have provided `odra_livenet.env` as a template.

### `odra_livenet.env` Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `ODRA_CASPER_LIVENET_NODE_ADDRESS` | The RPC address of the Casper node | `http://65.109.83.79:7777` |
| `ODRA_CASPER_LIVENET_CHAIN_NAME` | The chain name (e.g., casper-test) | `casper-test` |
| `ODRA_CASPER_LIVENET_SECRET_KEY_PATH` | Path to the deployer's secret key | `/path/to/secret.pem` |
| `ODRA_CASPER_LIVENET_EVENTS_URL` | The Event Stream URL (Critical for deployment verification) | `http://65.109.83.79:9999/events` |
| `DEPLOYER_ACCOUNT_HASH` | The account hash of the deployer (Required for generating output env) | `account-hash-...` |

**Important**: Ensure `ODRA_CASPER_LIVENET_EVENTS_URL` is correctly set. If using the specific testnet node `65.109.83.79`, utilize the `/events` endpoint, not `/events/main`.

## Deployment

To deploy the contracts, run the following command from the project root:

```bash
ODRA_CASPER_LIVENET_ENV=odra_livenet.env cargo run --bin ectoplasm_contracts_cli -- deploy
```

This command will:
1.  Deploy all tokens (WCSPR, ECTO, USDC, WETH, WBTC).
2.  Deploy the `PairFactory` contract.
3.  Deploy the `Factory` contract (initialized with `PairFactory`).
4.  Deploy the `Router` contract (initialized with `Factory` and `WCSPR`).
5.  Wait for all transactions to complete.
6.  **Automatically generate** `scripts/deploy-new.out.env`.

### Deployment Output

After a successful deployment, the file `scripts/deploy-new.out.env` is generated. It contains the environment variables needed for the frontend and other scripts, including:

- `NODE_ADDRESS`
- `CHAIN_NAME`
- `DEPLOYER_ACCOUNT_HASH`
- `..._PACKAGE_HASH` for all contracts
- `..._CONTRACT_HASH` for all contracts

Example content:
```env
NODE_ADDRESS=http://65.109.83.79:7777
CHAIN_NAME=casper-test
DEPLOYER_ACCOUNT_HASH=account-hash-...

PAIR_FACTORY_PACKAGE_HASH=hash-...
FACTORY_PACKAGE_HASH=hash-...
...
PAIR_FACTORY_CONTRACT_HASH=hash-...
FACTORY_CONTRACT_HASH=hash-...
```

## Troubleshooting

- **Timeout / Deploy Error**: If you see a timeout or generic deployment error, check if `ODRA_CASPER_LIVENET_EVENTS_URL` is reachable from your machine.
- **Gas Errors**: If a deployment fails due to execution error, check the transaction on the block explorer using the provided hash. If it ran out of gas, you may need to increase the gas limit in `bin/cli.rs`.
- **Existing Contracts**: The deployment script uses `load_or_deploy`. If a contract is already deployed (and recorded in Odra's state), it will skip redeployment. To force a fresh deployment, modify the `.odra` or state tracking file, or implement a force-deploy flag (not currently standard).
