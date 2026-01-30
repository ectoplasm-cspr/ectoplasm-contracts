//! Events for Yield Farming

use odra::prelude::*;
use odra::casper_types::U256;

/// Event emitted when LP tokens are staked
#[odra::event]
pub struct Staked {
    pub user: Address,
    pub pool_id: u32,
    pub amount: U256,
    pub timestamp: u64,
}

/// Event emitted when LP tokens are unstaked
#[odra::event]
pub struct Unstaked {
    pub user: Address,
    pub pool_id: u32,
    pub amount: U256,
    pub timestamp: u64,
}

/// Event emitted when rewards are claimed
#[odra::event]
pub struct RewardsClaimed {
    pub user: Address,
    pub pool_id: u32,
    pub reward_amount: U256,
    pub timestamp: u64,
}

/// Event emitted when a new pool is created
#[odra::event]
pub struct PoolCreated {
    pub pool_id: u32,
    pub lp_token: Address,
    pub reward_rate: U256,
    pub created_by: Address,
}

/// Event emitted when pool reward rate is updated
#[odra::event]
pub struct RewardRateUpdated {
    pub pool_id: u32,
    pub old_rate: U256,
    pub new_rate: U256,
    pub updated_by: Address,
}
