# 🚀 Ectoplasm Protocol - Quick Start Guide

Get up and running with the Ectoplasm Protocol in 5 minutes!

## Step 1: Create Your Wallet

Run the wallet setup script:

```bash
./scripts/setup-wallet.sh
```

This will:
- ✅ Install `casper-client` if needed
- ✅ Generate a new key pair
- ✅ Save keys to `~/casper-keys/`
- ✅ Display your account hash

**Important:** Save your account hash - you'll need it for the faucet!

## Step 2: Get Testnet Funds

### Option A: Casper Wallet (Easiest)

1. Install [Casper Wallet](https://www.casperwallet.io/) browser extension
2. Import your keys from `~/casper-keys/secret_key.pem`
3. Visit [Testnet Faucet](https://testnet.cspr.live/tools/faucet)
4. Click "Request tokens" (you'll get ~1000 CSPR)

### Option B: Manual Request

1. Copy your account hash from the wallet setup output
2. Visit [Testnet Faucet](https://testnet.cspr.live/tools/faucet)
3. Paste your account hash and request tokens

⚠️ **Note:** You can only request testnet funds ONCE per account!

## Step 3: Verify Your Balance

```bash
./scripts/check-balance.sh
```

You should see ~1000 CSPR (1,000,000,000,000 motes)

## Step 4: Build the Contracts

```bash
# Build all contracts
cargo odra build

# Or build specific contracts
cargo build --release
```

Compiled WASM files will be in `target/wasm32-unknown-unknown/release/`

## Step 5: Deploy to Testnet

We have existing deployment plans. Check them out:

- `DEPLOYMENT_ACTION_PLAN.md` - Main deployment guide
- `DEPLOYMENT_PLAN.md` - Detailed deployment steps
- `DEPLOYMENT_PLAN_PART2.md` - Additional deployment info

### Quick Deploy (coming soon)

```bash
# Deploy all contracts (script in progress)
./scripts/deploy-all.sh
```

## What's Deployed?

Based on existing deployments, we have:

✅ **DEX Layer**
- Factory Contract
- Router Contract
- WCSPR (LP Token)

✅ **Tokens**
- ECTO (Ectoplasm Token)
- USDC (USD Coin)
- WETH (Wrapped Ether)
- WBTC (Wrapped Bitcoin)

🚧 **Coming Soon**
- LST Layer (sCSPR staking)
- Yield Protocol (aECTO lending)
- Incentive System (gas discounts + LP boosts)

## Useful Commands

```bash
# Check balance
./scripts/check-balance.sh

# View account info
cat ~/casper-keys/account_info.txt

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

## Network Information

- **Network:** Casper Testnet
- **Chain Name:** `casper-test`
- **Node:** `http://65.21.235.122:7777`
- **Explorer:** https://testnet.cspr.live/
- **Faucet:** https://testnet.cspr.live/tools/faucet

## Need Help?

- 📖 [Full README](./README.md)
- 📋 [Deployment Plans](./DEPLOYMENT_ACTION_PLAN.md)
- 🐛 [GitHub Issues](https://github.com/your-org/ectoplasm-contracts/issues)
- 💬 [Discord](https://discord.gg/ectoplasm)

## Security Reminders

⚠️ **NEVER share your `secret_key.pem` file!**
⚠️ **This is TESTNET ONLY - not for real funds**
⚠️ **Always backup your keys securely**

---

**Ready to deploy?** Continue to [DEPLOYMENT_ACTION_PLAN.md](./DEPLOYMENT_ACTION_PLAN.md) for detailed deployment instructions!
