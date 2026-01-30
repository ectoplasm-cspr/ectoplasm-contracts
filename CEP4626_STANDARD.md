# CEP-4626: Tokenized Vault Standard for Casper

## 🎯 Purpose

CEP-4626 is a **standardized interface for tokenized vaults** on Casper Network, adapted from Ethereum's ERC-4626. It provides a consistent API for yield-bearing tokens, making it easier to build and integrate DeFi protocols.

## 🚀 Why We Need This

### The Problem
Without a standard:
- ❌ Every vault has different interfaces
- ❌ Integrations require custom adapters
- ❌ Error-prone implementations
- ❌ Wasted development time
- ❌ Poor composability

### The Solution
With CEP-4626:
- ✅ **One standard interface** for all vaults
- ✅ **Easy integration** across protocols
- ✅ **Consistent behavior** users can trust
- ✅ **Reusable components** save time
- ✅ **Better composability** in DeFi

## 📦 What It Standardizes

### Core Concept
A **vault** is a smart contract that:
1. Accepts deposits of an underlying token (e.g., CSPR, ECTO)
2. Issues **shares** representing ownership
3. Shares increase in value as yield accrues
4. Users can redeem shares for underlying tokens + yield

### Example: Liquid Staking
```
User deposits: 1000 CSPR
Vault mints: 1000 sCSPR shares (1:1 initially)

After 1 year of staking rewards:
1000 sCSPR = 1100 CSPR (10% yield)

User redeems: 1000 sCSPR
User receives: 1100 CSPR (original + rewards)
```

## 🔧 Standard Interface

### Metadata
- `asset()` - Get underlying token address
- `total_assets()` - Get total assets managed (including yield)

### Conversions
- `convert_to_shares(assets)` - Assets → Shares
- `convert_to_assets(shares)` - Shares → Assets

### Deposits
- `deposit(assets, receiver)` - Deposit assets, mint shares
- `mint(shares, receiver)` - Mint exact shares, deposit required assets

### Withdrawals
- `withdraw(assets, receiver, owner)` - Withdraw exact assets, burn shares
- `redeem(shares, receiver, owner)` - Redeem exact shares, receive assets

### Previews (Simulations)
- `preview_deposit(assets)` - Simulate deposit
- `preview_mint(shares)` - Simulate mint
- `preview_withdraw(assets)` - Simulate withdrawal
- `preview_redeem(shares)` - Simulate redemption

### Limits
- `max_deposit(receiver)` - Maximum deposit allowed
- `max_mint(receiver)` - Maximum mint allowed
- `max_withdraw(owner)` - Maximum withdrawal allowed
- `max_redeem(owner)` - Maximum redemption allowed

### Events
- `Deposit` - Emitted on deposit/mint
- `Withdraw` - Emitted on withdraw/redeem

## 💡 Use Cases in Our Project

### 1. sCSPR (Liquid Staking Vault)
```rust
// Underlying asset: CSPR
// Share token: sCSPR
// Yield source: Staking rewards

vault.deposit(cspr_amount, user) // Returns sCSPR shares
// sCSPR value grows with staking rewards
vault.redeem(scspr_amount, user, user) // Returns CSPR + rewards
```

### 2. aECTO (Lending Pool Vault)
```rust
// Underlying asset: ECTO
// Share token: aECTO
// Yield source: Borrowing interest

vault.deposit(ecto_amount, user) // Returns aECTO shares
// aECTO value grows with interest
vault.redeem(aecto_amount, user, user) // Returns ECTO + interest
```

### 3. Future Vaults
- Yield aggregators
- LP token vaults
- Strategy vaults
- Multi-asset vaults

## 📊 Exchange Rate Example

```
Initial State:
├─ Total Assets: 0
├─ Total Shares: 0
└─ Rate: 1:1

After First Deposit (1000 CSPR):
├─ Total Assets: 1000 CSPR
├─ Total Shares: 1000 sCSPR
└─ Rate: 1:1 (1 sCSPR = 1 CSPR)

After Yield (100 CSPR rewards):
├─ Total Assets: 1100 CSPR
├─ Total Shares: 1000 sCSPR (unchanged)
└─ Rate: 1.1:1 (1 sCSPR = 1.1 CSPR)

Second User Deposits (1100 CSPR):
├─ Receives: 1000 sCSPR (1100 / 1.1)
├─ Total Assets: 2200 CSPR
├─ Total Shares: 2000 sCSPR
└─ Rate: 1.1:1 (maintained)
```

## 🔒 Security Features

### Rounding Protection
- Always round **down** to protect the vault
- Prevents vault from being undercollateralized
- Users may receive slightly less, but vault stays safe

### Limit Functions
- Prevent deposits beyond capacity
- Enforce per-user limits
- Return 0 when operations are disabled

### Preview Functions
- Let users simulate before executing
- Include fees in previews
- No surprises for users

## 🏗️ Implementation Status

### ✅ Completed
- [x] CEP-4626 trait definition
- [x] Event definitions
- [x] Helper functions for calculations
- [x] Comprehensive documentation
- [x] Module structure

### 🔄 Next Steps
1. **Refactor sCSPR** to implement CEP-4626
2. **Build aECTO** with CEP-4626 from the start
3. **Create tests** for CEP-4626 compliance
4. **Add examples** for integrators

## 📝 Quick Reference

### For Vault Implementers
```rust
impl Cep4626Vault for MyVault {
    fn asset(&self) -> Address { /* underlying token */ }
    fn total_assets(&self) -> U256 { /* total managed */ }
    fn convert_to_shares(&self, assets: U256) -> U256 { /* conversion */ }
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256 { /* deposit logic */ }
    // ... implement all required functions
}
```

### For Integrators
```rust
// Deposit
let shares = vault.deposit(amount, user);

// Check value
let assets = vault.convert_to_assets(shares);

// Withdraw
let assets_received = vault.redeem(shares, user, user);
```

## 🌐 Benefits for Ecosystem

### For Users
- Consistent experience across all vaults
- Easy to understand and predict
- Better tooling and interfaces
- Safer operations

### For Developers
- Standard to follow
- Reduced integration work
- Reusable components
- Better testing

### For Protocols
- Easy to aggregate vaults
- Composable with other protocols
- Reduced maintenance
- Improved security

## 📚 Resources

- **Full Documentation**: `src/cep4626/README.md`
- **Trait Definition**: `src/cep4626/vault.rs`
- **Events**: `src/cep4626/events.rs`
- **ERC-4626 Spec**: https://eips.ethereum.org/EIPS/eip-4626

## 🎯 Impact on Our Project

### Phase 1 (LST)
- sCSPR will be **refactored** to CEP-4626
- Provides standard interface for liquid staking
- Easier integration with aggregators

### Phase 2 (Lending)
- aECTO will be **built with** CEP-4626
- Standard interface for lending pools
- Composable with other DeFi protocols

### Phase 3 (Yield Farming)
- LP vaults can use CEP-4626
- Standard interface for staking
- Easy to build aggregators

## 🚀 Next Actions

1. **Commit CEP-4626 standard** to repository
2. **Refactor sCSPR** to implement CEP-4626
3. **Build aECTO** with CEP-4626
4. **Create integration examples**
5. **Write compliance tests**

---

**CEP-4626 brings standardization to Casper DeFi, enabling faster development, better security, and greater composability.** 🎉
