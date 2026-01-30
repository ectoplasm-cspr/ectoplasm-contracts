//! Yield Farming - LP token staking with ECTO rewards
//! 
//! Users can stake LP tokens (e.g., sCSPR/ECTO) to earn ECTO rewards

pub mod staking_pool;
pub mod rewards_distributor;
pub mod errors;
pub mod events;

pub use staking_pool::StakingPool;
pub use rewards_distributor::RewardsDistributor;
pub use errors::FarmingError;
pub use events::*;
