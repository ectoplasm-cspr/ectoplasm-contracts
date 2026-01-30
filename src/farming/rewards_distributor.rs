//! Rewards Distributor - Calculates and distributes ECTO rewards

use odra::prelude::*;
use odra::casper_types::U256;

/// Rewards distributor (simple placeholder)
#[odra::module]
pub struct RewardsDistributor {
    /// Total rewards distributed
    total_distributed: Var<U256>,
}

#[odra::module]
impl RewardsDistributor {
    pub fn init(&mut self) {
        self.total_distributed.set(U256::zero());
    }
    
    pub fn get_total_distributed(&self) -> U256 {
        self.total_distributed.get_or_default()
    }
}
