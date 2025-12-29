//! Error definitions for the Liquid Staking Token (LST) system
use odra::prelude::*;

/// Custom errors for the LST contracts
#[odra::odra_error]
pub enum LstError {
    /// Insufficient CSPR balance for staking
    InsufficientCsprBalance = 200,
    
    /// Insufficient sCSPR balance for unstaking
    InsufficientScsprBalance = 201,
    
    /// Minimum stake amount not met
    BelowMinimumStake = 202,
    
    /// Maximum stake amount exceeded
    AboveMaximumStake = 203,
    
    /// Unstaking period not completed
    UnstakingPeriodNotComplete = 204,
    
    /// No withdrawable funds available
    NoWithdrawableFunds = 205,
    
    /// Invalid validator address
    InvalidValidator = 206,
    
    /// Staking operation failed
    StakingFailed = 207,
    
    /// Unstaking operation failed
    UnstakingFailed = 208,
    
    /// Withdrawal operation failed
    WithdrawalFailed = 209,
    
    /// Exchange rate calculation error
    ExchangeRateError = 210,
    
    /// Contract is paused
    ContractPaused = 211,
    
    /// Unauthorized access
    Unauthorized = 212,
    
    /// Invalid amount (zero or negative)
    InvalidAmount = 213,
    
    /// Rewards distribution failed
    RewardsDistributionFailed = 214,
    
    /// Total staked amount overflow
    TotalStakedOverflow = 215,
    
    /// Invalid unstake request ID
    InvalidUnstakeRequestId = 216,
    
    /// Unstake request already processed
    UnstakeRequestAlreadyProcessed = 217,
    
    /// Validator delegation limit reached
    ValidatorDelegationLimitReached = 218,
    
    /// Insufficient contract balance
    InsufficientContractBalance = 219,
    
    /// Transfer to validator failed
    TransferToValidatorFailed = 220,
}
