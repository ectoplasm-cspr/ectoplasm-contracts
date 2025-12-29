//! Error types for Yield Farming

use odra::prelude::*;

#[odra::odra_error]
pub enum FarmingError {
    /// Insufficient balance
    InsufficientBalance = 1,
    /// Zero amount not allowed
    ZeroAmount = 2,
    /// Pool not found
    PoolNotFound = 3,
    /// Pool already exists
    PoolAlreadyExists = 4,
    /// Unauthorized access
    Unauthorized = 5,
    /// Contract paused
    ContractPaused = 6,
    /// Invalid reward rate
    InvalidRewardRate = 7,
    /// No rewards to claim
    NoRewardsToClaim = 8,
    /// Pool not active
    PoolNotActive = 9,
}
