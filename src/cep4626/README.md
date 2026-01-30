# CEP-4626: Tokenized Vault Standard for Casper

## Overview

CEP-4626 is a Casper adaptation of Ethereum's ERC-4626 standard, providing a standardized API for tokenized vaults representing shares of a single underlying CEP-18 token.

## Motivation

Tokenized vaults (yield-bearing tokens, lending pools, liquid staking) lack standardization on Casper, leading to:
- Diverse implementation details
- Difficult integration for aggregators and protocols
- Error-prone custom adapters
- Wasted development resources

CEP-4626 solves this by providing a **standard interface** that:
- ✅ Lowers integration effort for yield-bearing vaults
- ✅ Creates consistent implementation patterns
- ✅ Enables composability across DeFi protocols
- ✅ Reduces errors and improves security

## Use Cases

### 1. Liquid Staking (sCSPR)
```rust
// User stakes CSPR → receives sCSPR shares
// sCSPR value increases as staking rewards accumulate
vault.deposit(cspr_amount, user_address) // Returns sCSPR shares
```

### 2. Lending Pools (aECTO)
```rust
// User deposits ECTO → receives aECTO shares
// aECTO value increases as interest accrues
vault.deposit(ecto_amount, user_address) // Returns aECTO shares
```

### 3. Yield Aggregators
```rust
// Vault automatically compounds yields from multiple sources
// Users hold shares that represent their portion of total assets
```

### 4. Interest-Bearing Tokens
```rust
// Any token that accrues value over time
// Standard interface for deposits, withdrawals, and conversions
```

## Standard Interface

### Metadata Functions

#### `asset() -> Address`
Returns the address of the underlying CEP-18 token.

```rust
let underlying_token = vault.asset();
```

#### `total_assets() -> U256`
Returns the total amount of underlying assets managed by the vault (including yield).

```rust
let total_managed = vault.total_assets();
```

### Conversion Functions

#### `convert_to_shares(assets: U256) -> U256`
Converts an amount of assets to shares at the current exchange rate.

```rust
let shares = vault.convert_to_shares(U256::from(1000));
```

#### `convert_to_assets(shares: U256) -> U256`
Converts an amount of shares to assets at the current exchange rate.

```rust
let assets = vault.convert_to_assets(U256::from(500));
```

### Deposit Functions

#### `deposit(assets: U256, receiver: Address) -> U256`
Deposits assets and mints shares to the receiver.

```rust
// User must first approve the vault to spend their tokens
underlying_token.approve(vault_address, amount);

// Deposit and receive shares
let shares_minted = vault.deposit(amount, user_address);
```

#### `mint(shares: U256, receiver: Address) -> U256`
Mints exact amount of shares by depositing the required assets.

```rust
// Mint exactly 1000 shares
let assets_deposited = vault.mint(U256::from(1000), user_address);
```

### Withdrawal Functions

#### `withdraw(assets: U256, receiver: Address, owner: Address) -> U256`
Withdraws exact amount of assets by burning the required shares.

```rust
// Withdraw exactly 500 assets
let shares_burned = vault.withdraw(
    U256::from(500),
    receiver_address,
    owner_address
);
```

#### `redeem(shares: U256, receiver: Address, owner: Address) -> U256`
Redeems exact amount of shares for the equivalent assets.

```rust
// Redeem exactly 1000 shares
let assets_received = vault.redeem(
    U256::from(1000),
    receiver_address,
    owner_address
);
```

### Preview Functions

These functions simulate operations without executing them:

```rust
// Preview how many shares you'd get for depositing assets
let expected_shares = vault.preview_deposit(assets);

// Preview how many assets you'd need to mint shares
let required_assets = vault.preview_mint(shares);

// Preview how many shares you'd burn to withdraw assets
let shares_to_burn = vault.preview_withdraw(assets);

// Preview how many assets you'd get for redeeming shares
let expected_assets = vault.preview_redeem(shares);
```

### Limit Functions

Check maximum amounts for operations:

```rust
// Maximum assets that can be deposited
let max_deposit = vault.max_deposit(user_address);

// Maximum shares that can be minted
let max_mint = vault.max_mint(user_address);

// Maximum assets that can be withdrawn
let max_withdraw = vault.max_withdraw(user_address);

// Maximum shares that can be redeemed
let max_redeem = vault.max_redeem(user_address);
```

## Events

### Deposit Event
Emitted when assets are deposited into the vault.

```rust
Deposit {
    sender: Address,      // Who called deposit
    owner: Address,       // Who received the shares
    assets: U256,         // Amount deposited
    shares: U256,         // Amount of shares minted
}
```

### Withdraw Event
Emitted when shares are redeemed from the vault.

```rust
Withdraw {
    sender: Address,      // Who called withdraw/redeem
    receiver: Address,    // Who received the assets
    owner: Address,       // Who owned the shares
    assets: U256,         // Amount withdrawn
    shares: U256,         // Amount of shares burned
}
```

## Exchange Rate Mechanics

The exchange rate between shares and assets changes over time as yield accrues:

### Initial State
```
Total Assets: 0
Total Shares: 0
Exchange Rate: 1:1
```

### After First Deposit (1000 assets)
```
Total Assets: 1000
Total Shares: 1000
Exchange Rate: 1:1
1 share = 1 asset
```

### After Yield Accrues (100 assets earned)
```
Total Assets: 1100 (1000 + 100 yield)
Total Shares: 1000 (unchanged)
Exchange Rate: 1.1:1
1 share = 1.1 assets
```

### Formulas

```rust
// Convert assets to shares
shares = (assets * total_shares) / total_assets

// Convert shares to assets
assets = (shares * total_assets) / total_shares

// Initial deposit (when total_shares == 0)
shares = assets  // 1:1 ratio
```

## Implementation Guidelines

### Requirements

1. **MUST implement CEP-18** for the share token
   - `transfer`, `approve`, `balance_of`, etc.

2. **MUST implement all CEP-4626 functions**
   - Metadata, conversions, deposits, withdrawals, previews, limits

3. **MUST emit events**
   - `Deposit` for deposits/mints
   - `Withdraw` for withdrawals/redeems

4. **MUST handle edge cases**
   - First deposit (0 total shares)
   - Rounding (always round down for user safety)
   - Limits and restrictions

### Best Practices

1. **Rounding**: Always round down to protect the vault
   - `convert_to_shares`: Round down
   - `convert_to_assets`: Round down
   - This ensures vault is never undercollateralized

2. **Fees**: Be transparent about fees
   - Preview functions MUST include fees
   - Conversion functions MUST NOT include fees
   - Document fee structure clearly

3. **Limits**: Implement sensible limits
   - Return 0 if operation is disabled
   - Return U256::MAX if no limit
   - Consider both global and per-user limits

4. **Security**: Follow security best practices
   - Check for reentrancy
   - Validate all inputs
   - Use safe math operations
   - Test edge cases thoroughly

## Example Implementation

Here's a simplified example of a CEP-4626 vault:

```rust
#[odra::module]
pub struct SimpleVault {
    asset_token: Var<Address>,
    total_assets: Var<U256>,
    // Share token is implemented via CEP-18
}

impl Cep4626Vault for SimpleVault {
    fn asset(&self) -> Address {
        self.asset_token.get_or_revert_with(Error::NotInitialized)
    }
    
    fn total_assets(&self) -> U256 {
        self.total_assets.get_or_default()
    }
    
    fn convert_to_shares(&self, assets: U256) -> U256 {
        let total_assets = self.total_assets();
        let total_shares = self.total_supply(); // From CEP-18
        
        if total_shares == U256::zero() {
            assets // 1:1 initial rate
        } else {
            (assets * total_shares) / total_assets
        }
    }
    
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256 {
        let shares = self.convert_to_shares(assets);
        
        // Transfer assets from caller to vault
        let asset_token = self.asset();
        let mut token = Cep18TokenContractRef::new(self.env(), asset_token);
        token.transfer_from(self.env().caller(), self.env().self_address(), assets);
        
        // Update total assets
        let current_total = self.total_assets();
        self.total_assets.set(current_total + assets);
        
        // Mint shares to receiver
        self.mint_shares(receiver, shares);
        
        // Emit event
        self.env().emit_event(Deposit {
            sender: self.env().caller(),
            owner: receiver,
            assets,
            shares,
        });
        
        shares
    }
    
    // ... implement other functions
}
```

## Integration with Existing Contracts

### Updating sCSPR to CEP-4626

The sCSPR token can be refactored to implement CEP-4626:

```rust
impl Cep4626Vault for StakingManager {
    fn asset(&self) -> Address {
        // Return CSPR token address
    }
    
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256 {
        // This is the `stake` function
        self.stake(validator, assets)
    }
    
    fn withdraw(&mut self, assets: U256, receiver: Address, owner: Address) -> U256 {
        // This is the `unstake` function
        // Note: May need to handle async unstaking period
    }
    
    // ... other implementations
}
```

### Creating aECTO with CEP-4626

The aECTO lending pool will implement CEP-4626 from the start:

```rust
#[odra::module]
pub struct AectoVault {
    // Implements CEP-4626 for ECTO deposits
}

impl Cep4626Vault for AectoVault {
    fn asset(&self) -> Address {
        // Return ECTO token address
    }
    
    fn total_assets(&self) -> U256 {
        // Return total ECTO + accrued interest
    }
    
    // ... full CEP-4626 implementation
}
```

## Benefits for the Ecosystem

### For Users
- ✅ Consistent interface across all vaults
- ✅ Easy to understand and use
- ✅ Predictable behavior
- ✅ Better tooling support

### For Developers
- ✅ Standard to implement
- ✅ Reduced integration effort
- ✅ Reusable components
- ✅ Better testing frameworks

### For Protocols
- ✅ Easy vault aggregation
- ✅ Composability with other protocols
- ✅ Reduced adapter maintenance
- ✅ Improved security through standardization

## Comparison with ERC-4626

| Feature | ERC-4626 | CEP-4626 |
|---------|----------|----------|
| Base Token Standard | ERC-20 | CEP-18 |
| Vault Interface | ✅ | ✅ |
| Deposit/Withdraw | ✅ | ✅ |
| Mint/Redeem | ✅ | ✅ |
| Preview Functions | ✅ | ✅ |
| Limit Functions | ✅ | ✅ |
| Events | ✅ | ✅ |
| Async Operations | ❌ | ⚠️ (Optional, for unstaking) |

## Future Extensions

### CEP-7540: Asynchronous Vaults
For operations with waiting periods (like unstaking):
- Request-based withdrawals
- Claim mechanism after waiting period
- Status tracking for pending operations

### CEP-7575: Multi-Asset Vaults
For vaults managing multiple underlying assets:
- Multiple asset support
- Rebalancing mechanisms
- Asset-specific limits

## Testing

Comprehensive tests should cover:

```rust
#[test]
fn test_initial_deposit() {
    // First deposit should be 1:1
}

#[test]
fn test_deposit_after_yield() {
    // Subsequent deposits should use exchange rate
}

#[test]
fn test_withdraw() {
    // Withdrawals should burn correct shares
}

#[test]
fn test_preview_functions() {
    // Previews should match actual operations
}

#[test]
fn test_rounding() {
    // Ensure rounding always favors vault
}

#[test]
fn test_limits() {
    // Test max deposit/mint/withdraw/redeem
}
```

## Security Considerations

1. **Inflation Attacks**: First depositor can manipulate exchange rate
   - Solution: Mint minimum shares on first deposit
   - Solution: Use virtual shares/assets

2. **Rounding Errors**: Accumulated rounding can drain vault
   - Solution: Always round in vault's favor
   - Solution: Use high precision math

3. **Reentrancy**: Malicious tokens can reenter
   - Solution: Follow checks-effects-interactions pattern
   - Solution: Use reentrancy guards

4. **Fee Manipulation**: Fees can be used to extract value
   - Solution: Cap fees at reasonable levels
   - Solution: Make fees transparent and auditable

## Resources

- **ERC-4626 Specification**: https://eips.ethereum.org/EIPS/eip-4626
- **CEP-18 Standard**: Casper's token standard
- **OpenZeppelin Implementation**: Reference for ERC-4626

## Conclusion

CEP-4626 brings standardization to tokenized vaults on Casper, enabling:
- 🚀 Faster development
- 🔒 Better security
- 🔗 Greater composability
- 📈 Improved user experience

By adopting this standard, we create a more robust and interoperable DeFi ecosystem on Casper Network.
