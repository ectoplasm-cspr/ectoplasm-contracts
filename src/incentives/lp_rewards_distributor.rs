//! Rewards Distributor
//! 
//! Manages LP boost rewards based on protocol participation.
//! Calculates boost multipliers and distributes additional rewards to LPs.
//! 
//! Boost Multipliers:
//! - Base: 1.0x (just providing liquidity)
//! - +0.3x: Hold aECTO (deposited in yield protocol)
//! - +0.5x: Active borrower (borrowing ECTO)
//! - +0.2x: Hold sCSPR (supporting network security)
//! - Max: 2.0x total multiplier

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use crate::token::Cep18TokenContractRef;

/// LP position with boost information
#[odra::odra_type]
pub struct LpPosition {
    /// User address
    pub user: Address,
    /// Pair address
    pub pair: Address,
    /// LP token amount
    pub lp_amount: U256,
    /// Base APR (from trading fees)
    pub base_apr: U256,
    /// Boost multiplier (scaled by 1e18, e.g., 1.5e18 = 1.5x)
    pub boost_multiplier: U256,
    /// Effective APR after boost
    pub effective_apr: U256,
    /// Last update timestamp
    pub last_update: u64,
}

/// User's boost factors
#[odra::odra_type]
pub struct BoostFactors {
    /// Has aECTO deposits
    pub has_aecto: bool,
    /// Is active borrower
    pub is_borrower: bool,
    /// Has sCSPR holdings
    pub has_scspr: bool,
    /// Total multiplier (scaled by 1e18)
    pub total_multiplier: U256,
}

/// LP Rewards Distributor contract
#[odra::module]
pub struct LpRewardsDistributor {
    /// sCSPR token address
    scspr_token: Var<Address>,
    /// aECTO token address
    aecto_token: Var<Address>,
    /// Lending pool address (to check borrower status)
    lending_pool: Var<Address>,
    /// Reward token address (ECTO)
    reward_token: Var<Address>,
    /// Admin address
    admin: Var<Address>,
    /// LP positions (user + pair -> LpPosition)
    lp_positions: Mapping<(Address, Address), LpPosition>,
    /// Total rewards distributed
    total_rewards_distributed: Var<U256>,
    /// Rewards pool balance
    rewards_pool: Var<U256>,
    /// Minimum aECTO for boost
    min_aecto_for_boost: Var<U256>,
    /// Minimum sCSPR for boost
    min_scspr_for_boost: Var<U256>,
    /// Whether boosts are enabled
    enabled: Var<bool>,
    /// Base multiplier (1e18 = 1.0x)
    base_multiplier: Var<U256>,
    /// aECTO boost (0.3e18 = 0.3x)
    aecto_boost: Var<U256>,
    /// Borrower boost (0.5e18 = 0.5x)
    borrower_boost: Var<U256>,
    /// sCSPR boost (0.2e18 = 0.2x)
    scspr_boost: Var<U256>,
}

#[odra::module]
impl LpRewardsDistributor {
    /// Initialize the rewards distributor
    pub fn init(
        &mut self,
        scspr_token_address: Address,
        aecto_token_address: Address,
        lending_pool_address: Address,
        reward_token_address: Address,
    ) {
        let caller = self.env().caller();
        
        self.scspr_token.set(scspr_token_address);
        self.aecto_token.set(aecto_token_address);
        self.lending_pool.set(lending_pool_address);
        self.reward_token.set(reward_token_address);
        self.admin.set(caller);
        
        self.total_rewards_distributed.set(U256::zero());
        self.rewards_pool.set(U256::zero());
        self.enabled.set(true);
        
        // Set minimum balances for boosts
        // 1,000 aECTO (18 decimals)
        self.min_aecto_for_boost.set(U256::from(1000) * U256::from(10u128.pow(18)));
        // 100 sCSPR (9 decimals)
        self.min_scspr_for_boost.set(U256::from(100) * U256::from(10u128.pow(9)));
        
        // Set boost multipliers (scaled by 1e18)
        let scale = U256::from(10u128.pow(18));
        self.base_multiplier.set(scale); // 1.0x
        self.aecto_boost.set(scale * U256::from(3) / U256::from(10)); // 0.3x
        self.borrower_boost.set(scale * U256::from(5) / U256::from(10)); // 0.5x
        self.scspr_boost.set(scale * U256::from(2) / U256::from(10)); // 0.2x
    }
    
    /// Register or update an LP position
    pub fn register_lp_position(
        &mut self,
        user: Address,
        pair: Address,
        lp_amount: U256,
        base_apr: U256,
    ) {
        if !self.enabled.get_or_default() {
            return;
        }
        
        // Calculate boost multiplier
        let boost_factors = self.calculate_boost_factors(user);
        let boost_multiplier = boost_factors.total_multiplier;
        
        // Calculate effective APR
        let scale = U256::from(10u128.pow(18));
        let effective_apr = base_apr * boost_multiplier / scale;
        
        // Store position
        let position = LpPosition {
            user,
            pair,
            lp_amount,
            base_apr,
            boost_multiplier,
            effective_apr,
            last_update: self.env().get_block_time(),
        };
        
        self.lp_positions.set(&(user, pair), position);
        
        // Emit event
        self.env().emit_event(LpPositionRegistered {
            user,
            pair,
            lp_amount,
            boost_multiplier,
            effective_apr,
            timestamp: self.env().get_block_time(),
        });
    }
    
    /// Calculate boost factors for a user
    pub fn calculate_boost_factors(&self, user: Address) -> BoostFactors {
        if !self.enabled.get_or_default() {
            let scale = U256::from(10u128.pow(18));
            return BoostFactors {
                has_aecto: false,
                is_borrower: false,
                has_scspr: false,
                total_multiplier: scale, // 1.0x base
            };
        }
        
        // Check aECTO holdings
        let aecto_balance = self.get_aecto_balance(user);
        let min_aecto = self.min_aecto_for_boost.get_or_default();
        let has_aecto = aecto_balance >= min_aecto;
        
        // Check sCSPR holdings
        let scspr_balance = self.get_scspr_balance(user);
        let min_scspr = self.min_scspr_for_boost.get_or_default();
        let has_scspr = scspr_balance >= min_scspr;
        
        // Check if user is borrower
        let is_borrower = self.is_active_borrower(user);
        
        // Calculate total multiplier
        let mut total_multiplier = self.base_multiplier.get_or_default();
        
        if has_aecto {
            total_multiplier = total_multiplier + self.aecto_boost.get_or_default();
        }
        
        if is_borrower {
            total_multiplier = total_multiplier + self.borrower_boost.get_or_default();
        }
        
        if has_scspr {
            total_multiplier = total_multiplier + self.scspr_boost.get_or_default();
        }
        
        // Cap at 2.0x
        let max_multiplier = U256::from(2) * U256::from(10u128.pow(18));
        if total_multiplier > max_multiplier {
            total_multiplier = max_multiplier;
        }
        
        BoostFactors {
            has_aecto,
            is_borrower,
            has_scspr,
            total_multiplier,
        }
    }
    
    /// Claim accumulated rewards for an LP position
    pub fn claim_rewards(&mut self, pair: Address) -> U256 {
        let caller = self.env().caller();
        
        let position = self.lp_positions.get(&(caller, pair));
        if position.is_none() {
            self.env().revert(DexError::InvalidPair);
        }
        
        let position = position.unwrap();
        
        // Calculate rewards since last update
        let current_time = self.env().get_block_time();
        let time_elapsed = current_time - position.last_update;
        
        // Calculate rewards based on effective APR
        // rewards = (lp_amount * effective_apr * time_elapsed) / (365 days * 1e18)
        let seconds_per_year = U256::from(365 * 24 * 60 * 60);
        let scale = U256::from(10u128.pow(18));
        
        let rewards = position.lp_amount
            * position.effective_apr
            * U256::from(time_elapsed)
            / (seconds_per_year * scale);
        
        if rewards == U256::zero() {
            return U256::zero();
        }
        
        // Check rewards pool has enough balance
        let pool_balance = self.rewards_pool.get_or_default();
        if rewards > pool_balance {
            self.env().revert(DexError::InsufficientLiquidity);
        }
        
        // Update rewards pool
        self.rewards_pool.set(pool_balance - rewards);
        
        // Update total distributed
        let total = self.total_rewards_distributed.get_or_default();
        self.total_rewards_distributed.set(total + rewards);
        
        // Update position timestamp
        let mut updated_position = position;
        updated_position.last_update = current_time;
        self.lp_positions.set(&(caller, pair), updated_position);
        
        // Transfer rewards to user
        let reward_token_address = self.reward_token.get().expect("Reward token not set");
        let mut reward_token = Cep18TokenContractRef::new(self.env(), reward_token_address);
        reward_token.transfer(caller, rewards);
        
        // Emit event
        self.env().emit_event(RewardsClaimed {
            user: caller,
            pair,
            amount: rewards,
            timestamp: current_time,
        });
        
        rewards
    }
    
    /// Update an existing LP position (e.g., when LP amount changes)
    pub fn update_lp_position(
        &mut self,
        user: Address,
        pair: Address,
        new_lp_amount: U256,
    ) {
        let position = self.lp_positions.get(&(user, pair));
        if position.is_none() {
            // If no position exists, register a new one with default base APR
            self.register_lp_position(user, pair, new_lp_amount, U256::from(15)); // 15% default
            return;
        }
        
        let mut position = position.unwrap();
        
        // Recalculate boost
        let boost_factors = self.calculate_boost_factors(user);
        let boost_multiplier = boost_factors.total_multiplier;
        
        let scale = U256::from(10u128.pow(18));
        let effective_apr = position.base_apr * boost_multiplier / scale;
        
        // Update position
        position.lp_amount = new_lp_amount;
        position.boost_multiplier = boost_multiplier;
        position.effective_apr = effective_apr;
        position.last_update = self.env().get_block_time();
        
        self.lp_positions.set(&(user, pair), position);
    }
    
    // Note: Odra Mapping doesn't support remove()
    // To "remove" a position, set lp_amount to zero using update_lp_position
    // /// Remove an LP position
    // pub fn remove_lp_position(&mut self, user: Address, pair: Address) {
    //     // self.lp_positions.remove(&(user, pair));
    //     
    //     self.env().emit_event(LpPositionRemoved {
    //         user,
    //         pair,
    //         timestamp: self.env().get_block_time(),
    //     });
    // }
    
    // ========================================
    // Helper Functions
    // ========================================
    
    /// Get user's aECTO balance
    fn get_aecto_balance(&self, user: Address) -> U256 {
        let aecto_address = match self.aecto_token.get() {
            Some(addr) => addr,
            None => return U256::zero(),
        };
        
        let token = Cep18TokenContractRef::new(self.env(), aecto_address);
        token.balance_of(user)
    }
    
    /// Get user's sCSPR balance
    fn get_scspr_balance(&self, user: Address) -> U256 {
        let scspr_address = match self.scspr_token.get() {
            Some(addr) => addr,
            None => return U256::zero(),
        };
        
        let token = Cep18TokenContractRef::new(self.env(), scspr_address);
        token.balance_of(user)
    }
    
    /// Check if user is an active borrower
    fn is_active_borrower(&self, user: Address) -> bool {
        let lending_pool_address = match self.lending_pool.get() {
            Some(addr) => addr,
            None => return false,
        };
        
        // Call lending pool to check if user has active borrow position
        // For now, we'll use a simple external call
        // In production, you'd use the proper ContractRef
        let lending_pool = LendingPoolContractRef::new(self.env(), lending_pool_address);
        
        match lending_pool.get_borrow_position(user) {
            Some(position) => position.principal > U256::zero(),
            None => false,
        }
    }
    
    // ========================================
    // View Functions
    // ========================================
    
    /// Get LP position for a user and pair
    pub fn get_lp_position(&self, user: Address, pair: Address) -> Option<LpPosition> {
        self.lp_positions.get(&(user, pair))
    }
    
    /// Get boost factors for a user
    pub fn get_boost_factors(&self, user: Address) -> BoostFactors {
        self.calculate_boost_factors(user)
    }
    
    /// Get total rewards distributed
    pub fn get_total_rewards_distributed(&self) -> U256 {
        self.total_rewards_distributed.get_or_default()
    }
    
    /// Get rewards pool balance
    pub fn get_rewards_pool_balance(&self) -> U256 {
        self.rewards_pool.get_or_default()
    }
    
    /// Calculate pending rewards for a user
    pub fn get_pending_rewards(&self, user: Address, pair: Address) -> U256 {
        let position = self.lp_positions.get(&(user, pair));
        if position.is_none() {
            return U256::zero();
        }
        
        let position = position.unwrap();
        
        let current_time = self.env().get_block_time();
        let time_elapsed = current_time - position.last_update;
        
        let seconds_per_year = U256::from(365 * 24 * 60 * 60);
        let scale = U256::from(10u128.pow(18));
        
        let rewards = position.lp_amount
            * position.effective_apr
            * U256::from(time_elapsed)
            / (seconds_per_year * scale);
        
        rewards
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    /// Add rewards to the pool
    pub fn add_rewards(&mut self, amount: U256) {
        let caller = self.env().caller();
        
        // Transfer tokens from caller to contract
        let reward_token_address = self.reward_token.get().expect("Reward token not set");
        let mut reward_token = Cep18TokenContractRef::new(self.env(), reward_token_address);
        reward_token.transfer_from(caller, self.env().self_address(), amount);
        
        // Update pool balance
        let current_balance = self.rewards_pool.get_or_default();
        self.rewards_pool.set(current_balance + amount);
        
        self.env().emit_event(RewardsAdded {
            amount,
            added_by: caller,
            timestamp: self.env().get_block_time(),
        });
    }
    
    /// Update boost parameters
    pub fn update_boost_params(
        &mut self,
        aecto_boost: U256,
        borrower_boost: U256,
        scspr_boost: U256,
    ) {
        self.only_admin();
        
        self.aecto_boost.set(aecto_boost);
        self.borrower_boost.set(borrower_boost);
        self.scspr_boost.set(scspr_boost);
        
        self.env().emit_event(BoostParamsUpdated {
            aecto_boost,
            borrower_boost,
            scspr_boost,
            updated_by: self.env().caller(),
        });
    }
    
    /// Update minimum balances for boosts
    pub fn update_min_balances(&mut self, min_aecto: U256, min_scspr: U256) {
        self.only_admin();
        
        self.min_aecto_for_boost.set(min_aecto);
        self.min_scspr_for_boost.set(min_scspr);
    }
    
    /// Enable or disable boosts
    pub fn set_enabled(&mut self, enabled: bool) {
        self.only_admin();
        self.enabled.set(enabled);
        
        self.env().emit_event(BoostsToggled {
            enabled,
            toggled_by: self.env().caller(),
        });
    }
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = match self.admin.get() {
            Some(addr) => addr,
            None => self.env().revert(DexError::Unauthorized),
        };
        if caller != admin {
            self.env().revert(DexError::Unauthorized);
        }
    }
}

// ========================================
// External Contract References
// ========================================

#[odra::external_contract]
trait LendingPool {
    fn get_borrow_position(&self, user: Address) -> Option<BorrowPosition>;
}

#[odra::odra_type]
pub struct BorrowPosition {
    pub user: Address,
    pub principal: U256,
    pub interest_accrued: U256,
    pub last_update: u64,
}

// ========================================
// Events
// ========================================

#[odra::event]
pub struct LpPositionRegistered {
    pub user: Address,
    pub pair: Address,
    pub lp_amount: U256,
    pub boost_multiplier: U256,
    pub effective_apr: U256,
    pub timestamp: u64,
}

#[odra::event]
pub struct LpPositionRemoved {
    pub user: Address,
    pub pair: Address,
    pub timestamp: u64,
}

#[odra::event]
pub struct RewardsClaimed {
    pub user: Address,
    pub pair: Address,
    pub amount: U256,
    pub timestamp: u64,
}

#[odra::event]
pub struct RewardsAdded {
    pub amount: U256,
    pub added_by: Address,
    pub timestamp: u64,
}

#[odra::event]
pub struct BoostParamsUpdated {
    pub aecto_boost: U256,
    pub borrower_boost: U256,
    pub scspr_boost: U256,
    pub updated_by: Address,
}

#[odra::event]
pub struct BoostsToggled {
    pub enabled: bool,
    pub toggled_by: Address,
}

use crate::errors::DexError;

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    #[test]
    fn test_boost_calculation() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        
        let scspr_token = env.get_account(10);
        let aecto_token = env.get_account(11);
        let lending_pool = env.get_account(12);
        let reward_token = env.get_account(13);
        
        env.set_caller(admin);
        let init_args = RewardsDistributorInitArgs {
            scspr_token_address: scspr_token,
            aecto_token_address: aecto_token,
            lending_pool_address: lending_pool,
            reward_token_address: reward_token,
        };
        
        let distributor = RewardsDistributor::deploy(&env, init_args);
        
        // Test base multiplier (no boosts)
        let user = env.get_account(1);
        let factors = distributor.get_boost_factors(user);
        
        let scale = U256::from(10u128.pow(18));
        assert_eq!(factors.total_multiplier, scale); // 1.0x base
        assert_eq!(factors.has_aecto, false);
        assert_eq!(factors.is_borrower, false);
        assert_eq!(factors.has_scspr, false);
    }
}
