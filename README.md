# 🌊 Ectoplasm Protocol - Integrated DeFi Ecosystem for Casper Network

**The first fully-integrated DeFi protocol on Casper 2.0** combining Liquid Staking (LST), Automated Market Making (DEX), and Yield Generation with native gas optimization and cross-protocol incentives.

## 🎯 Vision

Ectoplasm creates a **capital-efficient DeFi flywheel** where:
- Stake CSPR to earn validator rewards (sCSPR)
- Provide liquidity with productive assets
- Earn boosted yields through protocol synergies
- Get gas discounts for holding native tokens
- Borrow stablecoins against your collateral

**All built on Casper 2.0's native features** - leveraging direct auction access, fee elimination, and reserved transaction slots.

## 🏗️ Protocol Architecture

Ectoplasm consists of **three integrated layers** working in harmony:

```
┌─────────────────────────────────────────────────────────────┐
│                    INCENTIVE LAYER                          │
│  Gas Discounts • LP Boosts • Rewards Distribution          │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌──────────────────────┐  ↔  ┌──────────────────────────────┐
│   LST PROTOCOL       │     │   YIELD PROTOCOL             │
│  • Stake CSPR        │     │  • Lend ECTO                 │
│  • Get sCSPR         │     │  • Borrow against collateral │
│  • Earn 8% APY       │     │  • Get aECTO (CEP-4626)      │
└──────────────────────┘     └──────────────────────────────┘
            ↕                              ↕
┌─────────────────────────────────────────────────────────────┐
│                    DEX LAYER (AMM)                          │
│  sCSPR/ECTO • sCSPR/CSPR • ECTO/USDC Pairs                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 🪙 Three-Token Ecosystem

### **sCSPR** - Liquid Staking Token (CEP-4626 Compliant)
- Represents staked CSPR earning validator rewards (~8% APY)
- **CEP-4626 Tokenized Vault** - standardized interface for composability
- Can be used as collateral, LP, or traded while earning staking yield
- Direct integration with Casper's native auction system
- 7-era (~16 hour) unstaking period

### **ECTO** - Native Stablecoin
- Soft-pegged to $1 USD
- Primary trading pair on the DEX
- Can be borrowed against sCSPR and other collateral
- Over-collateralized (150%+ LTV) for stability

### **aECTO** - Interest-Bearing ECTO (CEP-4626 Compliant)
- **CEP-4626 Tokenized Vault** representing deposited ECTO
- Automatically accrues interest from borrowers
- Composable with other DeFi protocols
- Redeemable 1:1 for ECTO + earned interest

---

## 🔥 Layer 1: Liquid Staking (LST)

### **Staking Manager** (`lst::staking_manager::StakingManager`)
Core contract for liquid staking operations with **CEP-4626 compliance**.

**Features:**
- ✅ **CEP-4626 Tokenized Vault Standard** - Full compliance for composability
- Stake CSPR directly to Casper validators via native auction access
- Mint sCSPR tokens representing staked position
- Dynamic exchange rate based on accumulated rewards
- Validator whitelist management
- Unstaking queue with 7-era withdrawal period

**Key Functions (CEP-4626):**
```rust
// Standard CEP-4626 Interface
deposit(assets: U256, receiver: Address) -> U256  // Stake CSPR, get sCSPR
mint(shares: U256, receiver: Address) -> U256     // Mint exact sCSPR amount
withdraw(assets: U256, receiver: Address, owner: Address) -> U256  // Unstake
redeem(shares: U256, receiver: Address, owner: Address) -> U256    // Burn sCSPR

// View Functions
total_assets() -> U256           // Total CSPR staked
total_supply() -> U256           // Total sCSPR minted
convert_to_shares(assets: U256) -> U256   // CSPR → sCSPR
convert_to_assets(shares: U256) -> U256   // sCSPR → CSPR

// LST-Specific
stake(validator: Address, cspr_amount: U256) -> U256
unstake(scspr_amount: U256) -> u64  // Returns request_id
withdraw_unstaked(request_id: u64)
distribute_rewards(rewards_amount: U256)
```

### **sCSPR Token** (`lst::scspr_token::ScsprToken`)
CEP-18 token representing liquid staked CSPR.

**Features:**
- Standard CEP-18 interface (transfer, approve, etc.)
- Appreciates in value as staking rewards accrue
- Composable with all DeFi protocols
- Can be used as collateral while earning yield

---

## 💰 Layer 2: Yield Protocol (Aave-like Lending)

### **Lending Pool** (`lending::lending_pool::LendingPool`)
Core lending protocol for ECTO deposits and borrowing.

**Features:**
- Deposit ECTO to earn interest
- Borrow ECTO against approved collateral (sCSPR, WETH, WBTC)
- Variable interest rates based on utilization
- Over-collateralized positions (150%+ LTV)
- Automated liquidations for undercollateralized positions
- Reserve factor for protocol sustainability

**Key Functions:**
```rust
deposit(amount: U256) -> U256           // Deposit ECTO, get aECTO
withdraw(amount: U256) -> U256          // Withdraw ECTO
borrow(amount: U256, collateral_asset: Address)  // Borrow against collateral
repay(amount: U256)                     // Repay borrowed ECTO
liquidate(borrower: Address, debt_to_cover: U256, collateral_asset: Address)

// View Functions
get_borrow_rate() -> U256               // Current borrow APY
get_supply_rate() -> U256               // Current supply APY
get_utilization_rate() -> U256          // % of ECTO borrowed
get_borrow_position(user: Address) -> BorrowPosition
```

### **aECTO Vault** (`lending::aecto_vault::AectoVault`)
**CEP-4626 compliant** interest-bearing token for ECTO deposits.

**Features:**
- ✅ **CEP-4626 Tokenized Vault Standard** - Full compliance
- Automatically accrues interest from borrowers
- Redeemable for ECTO + earned interest
- Composable with other protocols
- Can be used for gas discounts

**Key Functions (CEP-4626):**
```rust
// Standard CEP-4626 Interface
deposit(assets: U256, receiver: Address) -> U256  // Deposit ECTO, get aECTO
mint(shares: U256, receiver: Address) -> U256
withdraw(assets: U256, receiver: Address, owner: Address) -> U256
redeem(shares: U256, receiver: Address, owner: Address) -> U256

// View Functions
total_assets() -> U256           // Total ECTO deposited
convert_to_shares(assets: U256) -> U256   // ECTO → aECTO
convert_to_assets(shares: U256) -> U256   // aECTO → ECTO
```

### **Collateral Manager** (`lending::collateral_manager::CollateralManager`)
Manages collateral deposits and health factors.

**Features:**
- Multi-collateral support (sCSPR, WETH, WBTC, etc.)
- Real-time health factor calculation
- Liquidation threshold monitoring
- Collateral value tracking via price oracle

### **Price Oracle** (`lending::price_oracle::PriceOracle`)
Provides real-time price feeds for collateral assets.

---

## 🔄 Layer 3: DEX (Automated Market Maker)

### **Factory Contract** (`dex::factory::Factory`)
Creates and manages trading pairs.

**Features:**
- Create new token pairs (Uniswap V2 style)
- Track all existing pairs
- Protocol fee management (0.05% to treasury)
- Pair registry

**Key Functions:**
```rust
create_pair(token_a: Address, token_b: Address) -> Address
get_pair(token_a: Address, token_b: Address) -> Option<Address>
all_pairs_length() -> u32
set_fee_to(address: Address)  // Admin only
```

### **Pair Contract** (`dex::pair::Pair`)
Constant product AMM for token swaps.

**Features:**
- Constant product formula (x × y = k)
- 0.3% swap fee (0.25% to LPs, 0.05% to protocol)
- LP token minting/burning
- Price oracle (TWAP)
- Reentrancy protection
- Flash swap support

**Key Functions:**
```rust
mint(to: Address) -> U256               // Mint LP tokens
burn(to: Address) -> (U256, U256)       // Burn LP tokens, get tokens back
swap(amount0_out: U256, amount1_out: U256, to: Address)
get_reserves() -> (U256, U256, u64)     // Get current reserves + timestamp
sync()                                   // Sync reserves with balances
```

### **Router Contract** (`dex::router::Router`)
User-facing interface for all DEX operations.

**Features:**
- Add/remove liquidity with slippage protection
- Single and multi-hop swaps
- Exact input or exact output swaps
- Deadline protection
- Automatic pair creation
- Quote functions for price discovery

**Key Functions:**
```rust
add_liquidity(
    token_a: Address, token_b: Address,
    amount_a_desired: U256, amount_b_desired: U256,
    amount_a_min: U256, amount_b_min: U256,
    to: Address, deadline: u64
) -> (U256, U256, U256)  // Returns (amount_a, amount_b, liquidity)

remove_liquidity(
    token_a: Address, token_b: Address,
    liquidity: U256,
    amount_a_min: U256, amount_b_min: U256,
    to: Address, deadline: u64
) -> (U256, U256)

swap_exact_tokens_for_tokens(
    amount_in: U256, amount_out_min: U256,
    path: Vec<Address>, to: Address, deadline: u64
) -> Vec<U256>  // Returns amounts for each hop

get_amounts_out(amount_in: U256, path: Vec<Address>) -> Vec<U256>
```

---

## 🎁 Layer 4: Incentive System

### **Gas Discount Manager** (`incentives::gas_discount::GasDiscountManager`)
Provides tiered gas discounts based on native token holdings.

**Discount Tiers:**
```
Tier 0: No holdings        → 0% discount
Tier 1: 100+ sCSPR         → 10% discount
Tier 2: 500+ sCSPR         → 25% discount
Tier 3: 2,000+ sCSPR       → 40% discount
Tier 4: 10,000+ sCSPR      → 60% discount

OR (alternative qualification):
Tier 1: 1,000+ aECTO       → 10% discount
Tier 2: 5,000+ aECTO       → 25% discount
Tier 3: 20,000+ aECTO      → 40% discount
Tier 4: 100,000+ aECTO     → 60% discount
```

**How It Works:**
- Checks user's sCSPR and aECTO balance before DEX transactions
- Subsidizes gas costs from protocol treasury
- Leverages Casper 2.0's Fee Elimination (when activated)
- Funded by protocol fees (0.05% of swaps)

### **LP Boost System** (`incentives::rewards_distributor::RewardsDistributor`)
Provides boosted yields for LPs based on protocol participation.

**Boost Multipliers:**
```
Base LP APR: 1.0x (just providing liquidity)

Boosts:
+ 0.3x: Hold aECTO (deposited in yield protocol)
+ 0.5x: Active borrower (borrowing ECTO)
+ 0.2x: Hold sCSPR (supporting network security)

Max Multiplier: 2.0x
```

**Example:**
- Base trading fee APR: 15%
- User holds aECTO + sCSPR
- Multiplier: 1.0 + 0.3 + 0.2 = 1.5x
- **Effective APR: 22.5%**

### **Incentive Manager** (`incentives::incentive_manager::IncentiveManager`)
Coordinates all incentive mechanisms across the protocol.

**Features:**
- Calculate and distribute LP boost rewards
- Manage gas subsidy pool
- Track user participation across all layers
- Emission schedule management
- Treasury management

---

## 🔥 The Flywheel Effect

### **How Components Create Synergy:**

```
1. Stake CSPR → Get sCSPR (Earning 8% staking yield)
   ↓
2. Provide sCSPR/ECTO Liquidity (Earning trading fees + boost)
   ↓
3. Deposit ECTO → Get aECTO (Earning interest from borrowers)
   ↓
4. Use sCSPR as Collateral → Borrow ECTO (Leverage position)
   ↓
5. Add Borrowed ECTO to LP (Compounding yields)
   ↓
6. All Holdings Give Gas Discounts (Cheaper operations)
   ↓
7. Compound & Rebalance (Repeat the cycle)
```

**Result:** Users earn **multiple yield streams simultaneously** while reducing costs.

---

## 💎 Unique Advantages

### **1. Capital Efficiency**
- sCSPR earns staking yield (~8%) **while** being used as LP or collateral
- aECTO earns interest **while** providing gas discounts
- **No idle capital** - everything is productive

### **2. Casper 2.0 Native Integration**
- ✅ Direct auction access for staking (no custodial risk)
- ✅ Fee elimination benefits (when activated)
- ✅ Reserved transaction slots for power users
- ✅ **Cannot be easily replicated on other chains**

### **3. CEP-4626 Compliance**
- ✅ **sCSPR** and **aECTO** are fully CEP-4626 compliant
- ✅ Standardized interface for composability
- ✅ Easy integration with future Casper DeFi protocols
- ✅ Familiar interface for developers from other ecosystems

### **4. Risk Management**
- Over-collateralized borrowing (150%+ LTV)
- Automated liquidation mechanisms
- Insurance fund for black swan events
- Gradual unstaking prevents bank runs

### **5. User Experience**
- Gas discounts reduce friction
- Clear boost multipliers incentivize engagement
- One-click strategies (coming soon)
- Transparent APY calculations

---

## 📊 Example User Journeys

### **Conservative User (Low Risk)**
```
Deposit: 10,000 ECTO → Get aECTO
Earnings: 8% APY from borrowers
Bonus: Tier 2 gas discount (25% off)
Net APY: ~8.5% (including gas savings)
Risk: Minimal (no liquidation risk)
```

### **Moderate User (Medium Risk)**
```
Stake: 5,000 CSPR → Get sCSPR (8% staking APY)
LP: Provide 5,000 sCSPR + 5,000 ECTO liquidity
Earnings: 15% APY from trading fees
Boost: 1.2x (holds sCSPR)
Net APY: ~26% (8% staking + 18% boosted LP)
Risk: Impermanent loss
```

### **Aggressive User (High Risk, High Reward)**
```
Stake: 10,000 CSPR → Get sCSPR
Borrow: 6,000 ECTO against sCSPR (60% LTV)
LP: Provide sCSPR/ECTO liquidity with borrowed ECTO
Deposit: Remaining ECTO in yield protocol
Boost: 2.0x (max multiplier)
Net APY: ~45-60% (leveraged position)
Risk: Liquidation if sCSPR drops or ECTO depegs
```

## 📁 Project Structure

```
ectoplasm-contracts/
├── Cargo.toml              # Rust dependencies
├── Odra.toml               # Odra contract configuration
├── CEP4626_STANDARD.md     # CEP-4626 standard documentation
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── errors.rs           # Global error definitions
│   ├── events.rs           # Global event definitions
│   ├── math.rs             # AMM math utilities
│   ├── token.rs            # CEP-18 token implementation
│   ├── tokens.rs           # Test token implementations
│   │
│   ├── cep4626/            # CEP-4626 Tokenized Vault Standard
│   │   ├── mod.rs          # CEP-4626 trait definitions
│   │   ├── vault.rs        # Base vault implementation
│   │   └── events.rs       # CEP-4626 events
│   │
│   ├── lst/                # Liquid Staking Token Protocol
│   │   ├── mod.rs          # LST module exports
│   │   ├── staking_manager.rs   # Main staking contract (CEP-4626)
│   │   ├── scspr_token.rs  # sCSPR token (CEP-18)
│   │   ├── errors.rs       # LST-specific errors
│   │   ├── events.rs       # LST events
│   │   └── tests.rs        # LST integration tests
│   │
│   ├── lending/            # Yield Protocol (Aave-like)
│   │   ├── mod.rs          # Lending module exports
│   │   ├── lending_pool.rs # Main lending pool contract
│   │   ├── aecto_vault.rs  # aECTO vault (CEP-4626)
│   │   ├── collateral_manager.rs  # Collateral management
│   │   ├── interest_rate.rs       # Interest rate strategy
│   │   ├── liquidation.rs  # Liquidation engine
│   │   ├── price_oracle.rs # Price oracle
│   │   ├── errors.rs       # Lending-specific errors
│   │   └── events.rs       # Lending events
│   │
│   ├── dex/                # Decentralized Exchange (AMM)
│   │   ├── mod.rs          # DEX module exports
│   │   ├── factory.rs      # Pair factory contract
│   │   ├── pair.rs         # Trading pair contract
│   │   ├── router.rs       # User-facing router
│   │   └── tests.rs        # DEX integration tests
│   │
│   └── incentives/         # Cross-Protocol Incentive System
│       ├── mod.rs          # Incentives module exports
│       ├── incentive_manager.rs   # Main coordinator
│       ├── gas_discount.rs        # Gas discount logic
│       └── rewards_distributor.rs # LP boost rewards
│
└── bin/
    ├── build_contract.rs   # Contract build script
    ├── build_schema.rs     # Schema generation
    └── cli.rs              # CLI interface
```

## 🚀 Getting Started

## ✅ On-chain verification (Casper v5)

See `VERIFICATION_COMMANDS.md` for copy/paste commands to verify transactions (mint/approve/add_liquidity/swap/remove_liquidity) and to prove swap balance deltas from on-chain Transfer events.

### Prerequisites

- **Rust** (nightly toolchain) - `rustup default nightly`
- **Cargo** - Comes with Rust
- **Odra CLI** - `cargo install odra-cli`
- **Casper Node** (for local testing) - Optional

### Installation

1. **Clone the repository:**
```bash
git clone https://github.com/your-org/ectoplasm-contracts
cd ectoplasm-contracts
```

2. **Install dependencies:**
```bash
cargo build --release
```

3. **Run tests:**
```bash
cargo test
```

### Building Contracts

Build all contracts for deployment:
```bash
cargo odra build
```

This generates WASM binaries in `target/wasm32-unknown-unknown/release/`

### Running Tests

Run the full test suite:
```bash
cargo odra test
```

Run specific module tests:
```bash
cargo test --package ectoplasm-contracts --lib lst::tests
cargo test --package ectoplasm-contracts --lib lending::tests
cargo test --package ectoplasm-contracts --lib dex::tests
```

## 🚀 Testnet Deployments

### LST Protocol (Deployed: Jan 28, 2026) ✅

Liquid Staking Token contracts are live on **Casper Testnet**:

#### sCSPR Token (Liquid Staking Token)
- **Package Hash:** `hash-0ebf8c78074ff3bed238a55e9f992074677cef6ed9339d367b559b7b86fbbe9d`
- **Contract Hash:** `hash-eb7a325b5d6281b901c0bd2b8f1989392c3a9d344183a7dabd1de970c7bcb01b`
- **Deploy TX:** `7a1960c985b0d7a0dae088ceb16699c97489573bc9dbf04adfb923d6841304e9`
- **Explorer:** [View on Testnet](https://testnet.cspr.live/deploy/7a1960c985b0d7a0dae088ceb16699c97489573bc9dbf04adfb923d6841304e9)
- **Features:** CEP-4626 compliant, stake CSPR to receive sCSPR tokens

#### Staking Manager
- **Package Hash:** `hash-bea004ced0141f9e84d95ad1ccf1a5b55311b86f1626bd91ebf42db5ae740ad4`
- **Contract Hash:** `hash-da7515b1ecfa57b4bd45402d331a3b39ce9f0e7c54cc99e940ebdf234b10eb51`
- **Deploy TX:** `f4c97b5c0cf8b652fd3d3b9cc9a9927e08e575857c42da61acb3adc847669034`
- **Explorer:** [View on Testnet](https://testnet.cspr.live/deploy/f4c97b5c0cf8b652fd3d3b9cc9a9927e08e575857c42da61acb3adc847669034)
- **Features:** Simplified staking (no validator params), 1 CSPR minimum, 7-era unstaking

**Key Improvements:**
- ✅ Removed validator logic for simplified architecture
- ✅ All 4 comprehensive tests passing
- ✅ Ready for production use

### Incentive Layer (Deployed: Dec 30, 2024)

All incentive contracts are live on **Casper Testnet**:

#### GasDiscountManager
- **Deploy Hash:** `7f00eae6da7beb8a1776b352877aa8d4233dd43ca6bda4cafec1902282db44eb`
- **Explorer:** [View on Testnet](https://testnet.cspr.live/deploy/7f00eae6da7beb8a1776b352877aa8d4233dd43ca6bda4cafec1902282db44eb)
- **Features:** Tiered gas discounts (0-60%) based on sCSPR/aECTO holdings

#### LpRewardsDistributor
- **Deploy Hash:** `a10bca5c11f7cbf247cc55efda6fb1dd0051f21ac2923d5bcc69479933215f6e`
- **Explorer:** [View on Testnet](https://testnet.cspr.live/deploy/a10bca5c11f7cbf247cc55efda6fb1dd0051f21ac2923d5bcc69479933215f6e)
- **Features:** LP boost system with multipliers up to 2.0x

#### IncentiveManager
- **Deploy Hash:** `8a7ab864bbff32e32c6a90a3b01819a785c534ecb06851f84215e401c76e3246`
- **Explorer:** [View on Testnet](https://testnet.cspr.live/deploy/8a7ab864bbff32e32c6a90a3b01819a785c534ecb06851f84215e401c76e3246)
- **Features:** Main coordinator for all incentive mechanisms

### Deploy Your Own

To deploy the incentive contracts:
```bash
# 1. Setup wallet and get testnet funds
./scripts/setup-wallet.sh

# 2. Build contracts
cargo odra build

# 3. Deploy to testnet
./scripts/deploy-incentives.sh
```

**Note:** Each contract deployment costs approximately 100-150 CSPR on testnet.

---

## 📖 Usage Examples

### Example 1: Stake CSPR and Get sCSPR (CEP-4626)

```rust
use odra::prelude::*;
use ectoplasm_contracts::lst::StakingManagerContractRef;

// Connect to staking manager
let staking_manager = StakingManagerContractRef::new(env, staking_manager_address);

// Stake 1000 CSPR using CEP-4626 deposit interface
let cspr_amount = U256::from(1000) * U256::from(10u128.pow(9)); // 1000 CSPR (9 decimals)
let scspr_received = staking_manager.deposit(
    cspr_amount,
    user_address  // Receiver of sCSPR
);

println!("Staked {} CSPR, received {} sCSPR", cspr_amount, scspr_received);
// Now earning ~8% APY from validator rewards!
```

### Example 2: Deposit ECTO in Yield Protocol (CEP-4626)

```rust
use ectoplasm_contracts::lending::AectoVaultContractRef;

// Connect to aECTO vault
let aecto_vault = AectoVaultContractRef::new(env, aecto_vault_address);

// Deposit 5000 ECTO using CEP-4626 deposit interface
let ecto_amount = U256::from(5000) * U256::from(10u128.pow(18)); // 5000 ECTO (18 decimals)
let aecto_received = aecto_vault.deposit(
    ecto_amount,
    user_address  // Receiver of aECTO
);

println!("Deposited {} ECTO, received {} aECTO", ecto_amount, aecto_received);
// Now earning interest from borrowers!
```

### Example 3: Provide Liquidity on DEX

```rust
use ectoplasm_contracts::dex::RouterContractRef;

// Connect to router
let router = RouterContractRef::new(env, router_address);

// Add sCSPR/ECTO liquidity
let scspr_amount = U256::from(1000) * U256::from(10u128.pow(9));
let ecto_amount = U256::from(1000) * U256::from(10u128.pow(18));

let (amount_a, amount_b, liquidity) = router.add_liquidity(
    scspr_token_address,
    ecto_token_address,
    scspr_amount,        // amount_a_desired
    ecto_amount,         // amount_b_desired
    scspr_amount * 95 / 100,  // amount_a_min (5% slippage)
    ecto_amount * 95 / 100,   // amount_b_min (5% slippage)
    user_address,        // LP token recipient
    deadline,            // Unix timestamp
);

println!("Added liquidity: {} sCSPR + {} ECTO = {} LP tokens", amount_a, amount_b, liquidity);
// Now earning trading fees + boost rewards!
```

### Example 4: Swap Tokens on DEX

```rust
// Swap 100 ECTO for sCSPR
let ecto_in = U256::from(100) * U256::from(10u128.pow(18));
let path = vec![ecto_token_address, scspr_token_address];

let amounts = router.swap_exact_tokens_for_tokens(
    ecto_in,             // Exact input amount
    U256::zero(),        // Min output (set properly in production!)
    path,                // Swap path
    user_address,        // Recipient
    deadline,
);

println!("Swapped {} ECTO for {} sCSPR", amounts[0], amounts[1]);
// Gas discount applied if you hold sCSPR or aECTO!
```

### Example 5: Borrow ECTO Against sCSPR Collateral

```rust
use ectoplasm_contracts::lending::LendingPoolContractRef;
use ectoplasm_contracts::lending::CollateralManagerContractRef;

// First, deposit sCSPR as collateral
let collateral_manager = CollateralManagerContractRef::new(env, collateral_manager_address);
collateral_manager.deposit_collateral(
    scspr_token_address,
    U256::from(1000) * U256::from(10u128.pow(9))  // 1000 sCSPR
);

// Then borrow ECTO (up to 60% LTV)
let lending_pool = LendingPoolContractRef::new(env, lending_pool_address);
let borrow_amount = U256::from(600) * U256::from(10u128.pow(18)); // 600 ECTO

lending_pool.borrow(
    borrow_amount,
    scspr_token_address  // Collateral asset
);

println!("Borrowed {} ECTO against sCSPR collateral", borrow_amount);
// Your sCSPR is still earning staking rewards while used as collateral!
```

### Example 6: Complete Flywheel Strategy

```rust
// Step 1: Stake CSPR → Get sCSPR (earning 8% APY)
let scspr_amount = staking_manager.deposit(cspr_amount, user_address);

// Step 2: Use half for liquidity, half as collateral
let scspr_for_lp = scspr_amount / 2;
let scspr_for_collateral = scspr_amount / 2;

// Step 3: Deposit sCSPR as collateral
collateral_manager.deposit_collateral(scspr_token_address, scspr_for_collateral);

// Step 4: Borrow ECTO (60% LTV)
let borrowed_ecto = lending_pool.borrow(
    scspr_for_collateral * 60 / 100,  // 60% of collateral value
    scspr_token_address
);

// Step 5: Provide sCSPR/ECTO liquidity (earning trading fees + boost)
let (_, _, lp_tokens) = router.add_liquidity(
    scspr_token_address,
    ecto_token_address,
    scspr_for_lp,
    borrowed_ecto,
    scspr_for_lp * 95 / 100,
    borrowed_ecto * 95 / 100,
    user_address,
    deadline,
);

println!("Flywheel activated! Earning multiple yield streams:");
println!("- Staking rewards on all sCSPR (~8% APY)");
println!("- Trading fees on LP position (~15% APY)");
println!("- Boost multiplier from holding sCSPR (1.2x)");
println!("- Gas discounts on all DEX operations");
println!("Total effective APY: ~35-40%");
```

---

## ✅ CEP-4626 Tokenized Vault Standard

Ectoplasm Protocol implements the **CEP-4626 Tokenized Vault Standard** for both **sCSPR** and **aECTO**, ensuring maximum composability and interoperability.

### Why CEP-4626?

1. **Standardization** - Familiar interface for developers from other ecosystems (ERC-4626)
2. **Composability** - Easy integration with other DeFi protocols
3. **Flexibility** - Supports both deposit/withdraw and mint/redeem patterns
4. **Transparency** - Clear conversion rates between assets and shares

### CEP-4626 Interface

Both `StakingManager` (sCSPR) and `AectoVault` (aECTO) implement:

```rust
trait Cep4626Vault {
    // Deposit/Withdraw
    fn deposit(assets: U256, receiver: Address) -> U256;
    fn withdraw(assets: U256, receiver: Address, owner: Address) -> U256;
    
    // Mint/Redeem
    fn mint(shares: U256, receiver: Address) -> U256;
    fn redeem(shares: U256, receiver: Address, owner: Address) -> U256;
    
    // View Functions
    fn total_assets() -> U256;
    fn total_supply() -> U256;
    fn convert_to_shares(assets: U256) -> U256;
    fn convert_to_assets(shares: U256) -> U256;
    fn max_deposit(receiver: Address) -> U256;
    fn max_mint(receiver: Address) -> U256;
    fn max_withdraw(owner: Address) -> U256;
    fn max_redeem(owner: Address) -> U256;
}
```

See [CEP4626_STANDARD.md](./CEP4626_STANDARD.md) for full specification.

## 🔢 AMM Mathematics

### Constant Product Formula
The DEX uses the constant product formula: `x * y = k`

Where:
- `x` = reserve of token0
- `y` = reserve of token1
- `k` = constant product (invariant)

### Swap Formula (with 0.3% fee)
```
amount_out = (amount_in * 997 * reserve_out) / (reserve_in * 1000 + amount_in * 997)
```

### Liquidity Calculation
**First deposit:**
```
liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
```

**Subsequent deposits:**
```
liquidity = min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
```

## ⚠️ Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 1 | InsufficientLiquidity | Not enough liquidity in pool |
| 2 | InsufficientInputAmount | Input amount too low |
| 3 | InsufficientOutputAmount | Output amount too low |
| 4 | InvalidPair | Invalid token pair |
| 5 | PairExists | Pair already exists |
| 6 | PairNotFound | Pair does not exist |
| 7 | ZeroAddress | Zero address provided |
| 8 | IdenticalAddresses | Same token addresses |
| 9 | InsufficientAmount | Amount too low |
| 10 | TransferFailed | Token transfer failed |
| 11 | DeadlineExpired | Transaction deadline passed |
| 12 | ExcessiveSlippage | Slippage too high |
| 13 | Overflow | Arithmetic overflow |
| 14 | Underflow | Arithmetic underflow |
| 15 | DivisionByZero | Division by zero |
| 16 | Unauthorized | Unauthorized access |
| 17 | InvalidPath | Invalid swap path |
| 18 | KInvariantViolated | K invariant check failed |
| 19 | InsufficientLiquidityMinted | Not enough LP tokens minted |
| 20 | InsufficientLiquidityBurned | Not enough LP tokens burned |
| 21 | Locked | Reentrancy detected |
| 22 | InvalidFee | Invalid fee value |

## 📡 Events

### PairCreated
Emitted when a new pair is created.
```rust
PairCreated {
    token0: Address,
    token1: Address,
    pair: Address,
    pair_count: u32,
}
```

### LiquidityAdded
Emitted when liquidity is added.
```rust
LiquidityAdded {
    provider: Address,
    pair: Address,
    amount0: U256,
    amount1: U256,
    liquidity: U256,
}
```

### LiquidityRemoved
Emitted when liquidity is removed.
```rust
LiquidityRemoved {
    provider: Address,
    pair: Address,
    amount0: U256,
    amount1: U256,
    liquidity: U256,
}
```

### Swap
Emitted when a swap occurs.
```rust
Swap {
    sender: Address,
    pair: Address,
    amount0_in: U256,
    amount1_in: U256,
    amount0_out: U256,
    amount1_out: U256,
    to: Address,
}
```

### Sync
Emitted when reserves are updated.
```rust
Sync {
    pair: Address,
    reserve0: U256,
    reserve1: U256,
}
```

## 🔒 Security Considerations

1. **Reentrancy Protection**: All state-changing functions use a lock mechanism
2. **Slippage Protection**: Users can set minimum output amounts
3. **Deadline Protection**: Transactions expire after deadline
4. **K Invariant Check**: Ensures constant product is maintained
5. **Minimum Liquidity**: 1000 LP tokens locked forever to prevent division by zero

## 🛠️ Development

### Adding New Features

1. Create new module in `src/dex/`
2. Export from `src/dex/mod.rs`
3. Add contract to `Odra.toml`
4. Write tests in `src/dex/tests.rs`

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## 📄 License

This project is licensed under the MIT License.

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## 📚 Resources

- [Casper Network Documentation](https://docs.casper.network/)
- [Odra Framework](https://odra.dev/)
- [CSPR.cloud API](https://docs.cspr.cloud/)
- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)
