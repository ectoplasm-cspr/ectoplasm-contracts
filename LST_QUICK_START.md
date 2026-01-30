# LST Quick Start Guide

## Overview

The Liquid Staking Token (LST) system allows users to stake CSPR and receive sCSPR tokens that remain liquid while earning staking rewards.

## Quick Reference

### For Users

#### Stake CSPR
```rust
// Stake CSPR (minimum 100 CSPR)
let cspr_amount = U256::from(1000_000_000_000u64); // 1000 CSPR
let scspr_received = staking_manager.stake(cspr_amount);
```

#### Check Exchange Rate
```rust
// Get current rate (sCSPR per CSPR, scaled by 1e18)
let rate = staking_manager.get_exchange_rate();

// Convert amounts
let scspr_for_1000_cspr = staking_manager.get_scspr_by_cspr(
    U256::from(1000_000_000_000u64)
);
```

#### Unstake sCSPR
```rust
// Initiate unstaking
let scspr_amount = U256::from(500_000_000_000u64); // 500 sCSPR
let request_id = staking_manager.unstake(scspr_amount);

// Check request status
let request = staking_manager.get_unstake_request(request_id).unwrap();
println!("Withdrawable at: {}", request.withdrawable_at);
```

#### Withdraw CSPR
```rust
// After waiting period (~16 hours)
staking_manager.withdraw_unstaked(request_id);
```

#### Use sCSPR in DeFi
```rust
// Transfer sCSPR
scspr_token.transfer(recipient, amount);

// Approve for DEX
scspr_token.approve(router_address, amount);

// Add liquidity on DEX
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

### For Administrators

#### Initialize System
```rust
// 1. Deploy sCSPR token
let scspr_token = ScsprToken::deploy(&env, NoArgs);

// 2. Deploy Staking Manager
let staking_manager = StakingManager::deploy(&env, NoArgs);

// 3. Initialize both contracts
let staking_manager_addr = staking_manager.address();
let scspr_token_addr = scspr_token.address();

scspr_token.init(staking_manager_addr);
staking_manager.init(scspr_token_addr);
```

#### Add Validators
```rust
// Add approved validators
staking_manager.add_validator(validator1_address);
staking_manager.add_validator(validator2_address);

// Check if validator is approved
let is_approved = staking_manager.is_validator_approved(validator1_address);

// Get all validators
let validators = staking_manager.get_validators();
```

#### Distribute Rewards
```rust
// Called periodically (e.g., daily) to update exchange rate
let rewards_earned = U256::from(100_000_000_000u64); // 100 CSPR
staking_manager.distribute_rewards(rewards_earned);
```

#### Update Parameters
```rust
// Update minimum stake (e.g., to 50 CSPR)
staking_manager.set_minimum_stake(U256::from(50_000_000_000u64));

// Update unstaking period (e.g., to 12 hours)
staking_manager.set_unstaking_period(43200);
```

#### Emergency Controls
```rust
// Pause contract
staking_manager.pause();

// Unpause contract
staking_manager.unpause();

// Check pause status
let is_paused = staking_manager.is_paused();
```

## Key Concepts

### Exchange Rate
- **Initial**: 1 sCSPR = 1 CSPR
- **After rewards**: 1 sCSPR > 1 CSPR (value increases)
- **Formula**: `sCSPR = (CSPR × Total sCSPR) / Total CSPR`

### Unstaking Process
1. **Initiate**: Burn sCSPR, create withdrawal request
2. **Wait**: 7 eras (~16 hours) - Casper's unstaking period
3. **Withdraw**: Receive CSPR with accrued value

### Validator Selection
- Only approved validators can receive delegations
- Admin manages validator whitelist
- Stake is distributed across validators

## View Functions

```rust
// Get totals
let total_cspr = staking_manager.get_total_cspr_staked();
let total_scspr = staking_manager.get_total_scspr_supply();

// Get user info
let user_scspr_balance = scspr_token.balance_of(user_address);
let user_requests = staking_manager.get_user_unstake_requests(user_address);

// Get validator info
let validator_stake = staking_manager.get_validator_stake(validator_address);

// Get parameters
let min_stake = staking_manager.get_minimum_stake();
let unstaking_period = staking_manager.get_unstaking_period();
```

## Events to Monitor

```rust
// Staking events
Staked { staker, cspr_amount, scspr_amount, validator, exchange_rate, timestamp }

// Unstaking events
Unstaked { unstaker, scspr_amount, cspr_amount, request_id, exchange_rate, withdrawable_at }

// Withdrawal events
Withdrawn { withdrawer, cspr_amount, request_id, timestamp }

// Reward events
RewardsDistributed { rewards_amount, total_cspr_staked, total_scspr_supply, new_exchange_rate, timestamp }

// Admin events
ValidatorAdded { validator, added_by, timestamp }
ValidatorRemoved { validator, removed_by, timestamp }
ContractPaused { paused_by, timestamp }
ContractUnpaused { unpaused_by, timestamp }
```

## Error Handling

Common errors and solutions:

| Error | Cause | Solution |
|-------|-------|----------|
| `BelowMinimumStake` | Stake amount < 100 CSPR | Increase stake amount |
| `InvalidValidator` | Validator not approved | Choose approved validator |
| `InsufficientScsprBalance` | Not enough sCSPR | Check balance before unstaking |
| `UnstakingPeriodNotComplete` | Trying to withdraw too early | Wait for unstaking period |
| `ContractPaused` | Contract is paused | Wait for admin to unpause |
| `Unauthorized` | Not admin | Use admin account |

## Integration Examples

### With DEX

```rust
// Create sCSPR/ECTO pair
factory.create_pair(scspr_address, ecto_address);

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

### With Lending (Phase 2 - Coming Soon)

```rust
// Use sCSPR as collateral to borrow ECTO
lending_pool.deposit_collateral(scspr_address, scspr_amount);
lending_pool.borrow(ecto_address, ecto_amount);

// Deposit ECTO to earn interest
lending_pool.deposit(ecto_address, ecto_amount);
// Receive aECTO tokens
```

## Constants

```rust
// Minimum stake
const MIN_STAKE: u64 = 100_000_000_000; // 100 CSPR

// Unstaking period
const UNSTAKING_PERIOD: u64 = 57_600; // ~16 hours (7 eras)

// Exchange rate scale
const EXCHANGE_RATE_SCALE: u128 = 1_000_000_000_000_000_000; // 1e18

// Token decimals
const DECIMALS: u8 = 18;
```

## Testing

```bash
# Check compilation
cargo check --lib

# Run tests (when test environment is set up)
cargo test --lib lst::tests

# Build contracts
cargo build --release
```

## Support & Resources

- **Documentation**: See `src/lst/README.md` for detailed docs
- **Phase 1 Summary**: See `PHASE1_COMPLETE.md`
- **Casper Docs**: https://docs.casper.network/
- **Liquid Staking**: https://www.casper.network/news/liquid-staking
- **WiseLending**: https://wiselending.com/

## Next: Phase 2 - Lending Protocol

Coming soon:
- aECTO interest-bearing token
- Lending pool for ECTO
- Collateralized borrowing
- Liquidation engine
- Variable interest rates

Stay tuned! 🚀
