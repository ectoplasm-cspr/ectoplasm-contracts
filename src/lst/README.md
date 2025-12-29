# Liquid Staking Token (LST) System

## Overview

The LST system enables liquid staking on Casper Network, allowing users to stake CSPR and receive sCSPR (Staked CSPR) tokens that represent their staked position while remaining liquid and composable in DeFi applications.

## Architecture

### Components

1. **sCSPR Token** (`scspr_token.rs`)
   - CEP-18 compatible liquid staking token
   - Represents staked CSPR with accrued rewards
   - Fully transferable and composable
   - Can be used as collateral in lending protocols
   - Can be traded on DEX

2. **Staking Manager** (`staking_manager.rs`)
   - Core contract managing staking operations
   - Handles stake/unstake/withdraw flows
   - Manages exchange rate between CSPR and sCSPR
   - Tracks validator delegations
   - Distributes staking rewards

3. **Events** (`events.rs`)
   - Comprehensive event logging for all operations
   - Enables off-chain tracking and analytics

4. **Errors** (`errors.rs`)
   - Custom error types for LST operations
   - Clear error messages for debugging

## Key Features

### 🔄 Liquid Staking
- **Stake CSPR** → Receive sCSPR tokens
- **sCSPR remains liquid** → Use in DeFi while earning rewards
- **Exchange rate grows** → As rewards accumulate, 1 sCSPR becomes worth more CSPR

### 💰 Reward Accrual
- Staking rewards automatically increase the value of sCSPR
- No need to claim rewards manually
- Exchange rate updates reflect accumulated rewards

### ⏱️ Unstaking Process
- **Initiate unstaking** → Burn sCSPR, create withdrawal request
- **Wait 7 eras** (~16 hours) → Casper's unstaking period
- **Withdraw** → Receive CSPR with accrued rewards

### 🎯 Validator Management
- Admin can add/remove approved validators
- Stake distribution across multiple validators
- Track delegation per validator

### 🛡️ Security Features
- Pausable contract for emergency situations
- Admin-controlled parameters
- Minimum stake requirements
- Reentrancy protection

## Usage

### For Users

#### Staking CSPR

```rust
// 1. Choose an approved validator
let validator = staking_manager.get_validators()[0];

// 2. Stake CSPR (minimum 100 CSPR)
let scspr_amount = staking_manager.stake(validator);
// Receives sCSPR tokens at current exchange rate
```

#### Unstaking sCSPR

```rust
// 1. Initiate unstaking
let scspr_to_unstake = U256::from(500_000_000_000u64); // 500 sCSPR
let request_id = staking_manager.unstake(scspr_to_unstake);

// 2. Wait for unstaking period (~16 hours)

// 3. Withdraw CSPR
staking_manager.withdraw(request_id);
// Receives CSPR with accrued rewards
```

#### Checking Exchange Rate

```rust
// Get current exchange rate (sCSPR per CSPR, scaled by 1e18)
let exchange_rate = staking_manager.get_exchange_rate();

// Convert between CSPR and sCSPR
let cspr_amount = U256::from(1000_000_000_000u64); // 1000 CSPR
let scspr_equivalent = staking_manager.get_scspr_by_cspr(cspr_amount);

let scspr_amount = U256::from(500_000_000_000u64); // 500 sCSPR
let cspr_equivalent = staking_manager.get_cspr_by_scspr(scspr_amount);
```

#### Using sCSPR in DeFi

```rust
// sCSPR is a standard CEP-18 token, so it can be:

// 1. Transferred
scspr_token.transfer(recipient, amount);

// 2. Used in DEX
router.add_liquidity(scspr_address, ecto_address, ...);
router.swap_exact_tokens_for_tokens(amount_in, amount_out_min, path, ...);

// 3. Used as collateral in lending (Phase 2)
lending_pool.deposit(scspr_address, amount);
lending_pool.borrow(ecto_address, amount, scspr_address);
```

### For Administrators

#### Adding Validators

```rust
// Add an approved validator for delegation
staking_manager.add_validator(validator_address);
```

#### Distributing Rewards

```rust
// Called periodically to update exchange rate with earned rewards
let rewards_earned = U256::from(100_000_000_000u64); // 100 CSPR
staking_manager.distribute_rewards(rewards_earned);
```

#### Managing Parameters

```rust
// Update minimum stake amount
staking_manager.set_minimum_stake(U256::from(50_000_000_000u64)); // 50 CSPR

// Update unstaking period
staking_manager.set_unstaking_period(43200); // 12 hours

// Pause/unpause contract
staking_manager.pause();
staking_manager.unpause();
```

## Exchange Rate Mechanics

The exchange rate between sCSPR and CSPR is dynamic and increases over time as rewards accumulate:

### Initial State
- **Total CSPR staked**: 0
- **Total sCSPR supply**: 0
- **Exchange rate**: 1:1 (1 sCSPR = 1 CSPR)

### After First Stake (1000 CSPR)
- **Total CSPR staked**: 1000 CSPR
- **Total sCSPR supply**: 1000 sCSPR
- **Exchange rate**: 1:1

### After Rewards (100 CSPR earned)
- **Total CSPR staked**: 1100 CSPR (1000 + 100 rewards)
- **Total sCSPR supply**: 1000 sCSPR (unchanged)
- **Exchange rate**: 0.909 sCSPR per CSPR
- **Value**: 1 sCSPR = 1.1 CSPR

### Formula

```
Exchange Rate (sCSPR per CSPR) = Total sCSPR Supply / Total CSPR Staked

CSPR Amount = (sCSPR Amount × Total CSPR Staked) / Total sCSPR Supply
sCSPR Amount = (CSPR Amount × Total sCSPR Supply) / Total CSPR Staked
```

## Integration with DEX

The sCSPR token can be seamlessly integrated with the existing DEX:

### Creating Liquidity Pools

```rust
// Create sCSPR/ECTO pair
factory.create_pair(scspr_address, ecto_address);

// Add liquidity
router.add_liquidity(
    scspr_address,
    ecto_address,
    scspr_amount,
    ecto_amount,
    min_scspr,
    min_ecto,
    provider,
    deadline
);
```

### Trading

```rust
// Swap CSPR → sCSPR → ECTO
let path = vec![cspr_address, scspr_address, ecto_address];
router.swap_exact_tokens_for_tokens(
    amount_in,
    amount_out_min,
    path,
    recipient,
    deadline
);
```

## Security Considerations

### Access Control
- Only Staking Manager can mint/burn sCSPR
- Only admin can manage validators and parameters
- Only admin can pause/unpause contracts

### Validation
- Minimum stake requirements prevent dust attacks
- Validator approval system ensures quality delegations
- Unstaking period enforced by Casper protocol

### Emergency Procedures
- Contract can be paused to halt operations
- Admin can update parameters if needed
- Unstake requests are immutable once created

## Events

All operations emit events for transparency and off-chain tracking:

- `Staked` - When CSPR is staked
- `Unstaked` - When sCSPR is unstaked
- `Withdrawn` - When CSPR is withdrawn
- `RewardsDistributed` - When rewards are added
- `ExchangeRateUpdated` - When rate changes
- `ValidatorAdded/Removed` - Validator management
- `ContractPaused/Unpaused` - Emergency controls

## Future Enhancements

### Phase 2: Aave-like Lending Protocol
- Use sCSPR as collateral
- Borrow ECTO stablecoin against sCSPR
- Earn interest on sCSPR deposits
- Liquidation mechanisms

### Phase 3: Yield Farming
- Stake LP tokens (sCSPR/ECTO)
- Earn ECTO rewards
- Boost APY for liquidity providers

### Phase 4: Advanced Features
- Auto-compounding rewards
- Validator performance tracking
- Slashing protection
- Cross-chain bridges

## Testing

Run tests with:

```bash
cargo test --package ectoplasm-contracts --lib lst::tests
```

## Deployment

1. Deploy sCSPR token contract
2. Deploy Staking Manager contract with sCSPR address
3. Initialize sCSPR token with Staking Manager address
4. Add approved validators
5. Set minimum stake and unstaking period
6. Unpause contracts (if paused)

## Constants

- **Minimum Stake**: 100 CSPR (configurable)
- **Unstaking Period**: 57,600 seconds (~16 hours, 7 eras)
- **Exchange Rate Scale**: 1e18 (for precision)
- **Token Decimals**: 18 (matches CSPR)

## Support

For questions or issues, please refer to:
- Casper Network Documentation: https://docs.casper.network/
- WiseLending: https://wiselending.com/
- Casper Liquid Staking: https://www.casper.network/news/liquid-staking
