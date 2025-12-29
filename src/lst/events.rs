//! Event definitions for the Liquid Staking Token (LST) system
use odra::prelude::*;
use odra::casper_types::U256;

/// Event emitted when CSPR is staked
#[odra::event]
pub struct Staked {
    /// Address of the staker
    pub staker: Address,
    /// Amount of CSPR staked
    pub cspr_amount: U256,
    /// Amount of sCSPR minted
    pub scspr_amount: U256,
    /// Validator address where CSPR was delegated
    pub validator: Address,
    /// Exchange rate at time of staking (sCSPR per CSPR, scaled by 1e18)
    pub exchange_rate: U256,
    /// Timestamp of the stake
    pub timestamp: u64,
}

/// Event emitted when sCSPR is unstaked
#[odra::event]
pub struct Unstaked {
    /// Address of the unstaker
    pub unstaker: Address,
    /// Amount of sCSPR burned
    pub scspr_amount: U256,
    /// Amount of CSPR to be received (after unstaking period)
    pub cspr_amount: U256,
    /// Unstake request ID
    pub request_id: u64,
    /// Exchange rate at time of unstaking (sCSPR per CSPR, scaled by 1e18)
    pub exchange_rate: U256,
    /// Timestamp when funds will be withdrawable
    pub withdrawable_at: u64,
}

/// Event emitted when unstaked CSPR is withdrawn
#[odra::event]
pub struct Withdrawn {
    /// Address of the withdrawer
    pub withdrawer: Address,
    /// Amount of CSPR withdrawn
    pub cspr_amount: U256,
    /// Unstake request ID
    pub request_id: u64,
    /// Timestamp of the withdrawal
    pub timestamp: u64,
}

/// Event emitted when staking rewards are distributed
#[odra::event]
pub struct RewardsDistributed {
    /// Total rewards distributed in CSPR
    pub rewards_amount: U256,
    /// New total CSPR staked (including rewards)
    pub total_cspr_staked: U256,
    /// Total sCSPR supply
    pub total_scspr_supply: U256,
    /// New exchange rate (sCSPR per CSPR, scaled by 1e18)
    pub new_exchange_rate: U256,
    /// Timestamp of distribution
    pub timestamp: u64,
}

/// Event emitted when the exchange rate is updated
#[odra::event]
pub struct ExchangeRateUpdated {
    /// Old exchange rate
    pub old_rate: U256,
    /// New exchange rate
    pub new_rate: U256,
    /// Total CSPR staked
    pub total_cspr: U256,
    /// Total sCSPR supply
    pub total_scspr: U256,
    /// Timestamp of update
    pub timestamp: u64,
}

/// Event emitted when a validator is added
#[odra::event]
pub struct ValidatorAdded {
    /// Validator address
    pub validator: Address,
    /// Added by (admin address)
    pub added_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when a validator is removed
#[odra::event]
pub struct ValidatorRemoved {
    /// Validator address
    pub validator: Address,
    /// Removed by (admin address)
    pub removed_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when the contract is paused
#[odra::event]
pub struct ContractPaused {
    /// Paused by (admin address)
    pub paused_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when the contract is unpaused
#[odra::event]
pub struct ContractUnpaused {
    /// Unpaused by (admin address)
    pub unpaused_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when minimum stake amount is updated
#[odra::event]
pub struct MinimumStakeUpdated {
    /// Old minimum stake
    pub old_minimum: U256,
    /// New minimum stake
    pub new_minimum: U256,
    /// Updated by (admin address)
    pub updated_by: Address,
}

/// Event emitted when unstaking period is updated
#[odra::event]
pub struct UnstakingPeriodUpdated {
    /// Old unstaking period (in seconds)
    pub old_period: u64,
    /// New unstaking period (in seconds)
    pub new_period: u64,
    /// Updated by (admin address)
    pub updated_by: Address,
}
