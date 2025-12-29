//! Staking Pool - Stake LP tokens to earn ECTO rewards
//! 
//! Users stake LP tokens (e.g., sCSPR/ECTO LP) and earn ECTO rewards
//! based on their share of the pool and time staked.

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use super::errors::FarmingError;
use super::events::*;
use crate::token::Cep18TokenContractRef;

/// Pool information
#[odra::odra_type]
pub struct PoolInfo {
    /// Pool ID
    pub pool_id: u32,
    /// LP token address
    pub lp_token: Address,
    /// Reward rate (ECTO per second per staked token)
    pub reward_rate: U256,
    /// Total staked in pool
    pub total_staked: U256,
    /// Last update timestamp
    pub last_update: u64,
    /// Accumulated reward per token
    pub reward_per_token_stored: U256,
    /// Is pool active
    pub is_active: bool,
}

/// User stake information
#[odra::odra_type]
pub struct UserStake {
    /// Amount staked
    pub amount: U256,
    /// Reward debt (for reward calculation)
    pub reward_debt: U256,
    /// Pending rewards
    pub pending_rewards: U256,
    /// Last update timestamp
    pub last_update: u64,
}

/// Staking Pool contract
#[odra::module]
pub struct StakingPool {
    /// ECTO reward token address
    reward_token: Var<Address>,
    /// Pool information by pool ID
    pools: Mapping<u32, PoolInfo>,
    /// User stakes: (user, pool_id) -> UserStake
    user_stakes: Mapping<(Address, u32), UserStake>,
    /// Next pool ID
    next_pool_id: Var<u32>,
    /// Admin address
    admin: Var<Address>,
    /// Paused state
    paused: Var<bool>,
}

#[odra::module]
impl StakingPool {
    /// Initialize the staking pool
    pub fn init(&mut self, reward_token_address: Address) {
        let caller = self.env().caller();
        self.reward_token.set(reward_token_address);
        self.next_pool_id.set(0);
        self.admin.set(caller);
        self.paused.set(false);
    }
    
    // ========================================
    // Pool Management (Admin)
    // ========================================
    
    /// Create a new staking pool
    /// 
    /// # Arguments
    /// * `lp_token` - LP token address to stake
    /// * `reward_rate` - ECTO rewards per second per staked token (scaled by 1e18)
    pub fn create_pool(&mut self, lp_token: Address, reward_rate: U256) -> u32 {
        self.only_admin();
        
        if reward_rate == U256::zero() {
            self.env().revert(FarmingError::InvalidRewardRate);
        }
        
        let pool_id = self.next_pool_id.get_or_default();
        
        let pool = PoolInfo {
            pool_id,
            lp_token,
            reward_rate,
            total_staked: U256::zero(),
            last_update: self.env().get_block_time(),
            reward_per_token_stored: U256::zero(),
            is_active: true,
        };
        
        self.pools.set(&pool_id, pool);
        self.next_pool_id.set(pool_id + 1);
        
        let admin = self.admin.get_or_revert_with(FarmingError::Unauthorized);
        self.env().emit_event(PoolCreated {
            pool_id,
            lp_token,
            reward_rate,
            created_by: admin,
        });
        
        pool_id
    }
    
    /// Update pool reward rate
    pub fn update_reward_rate(&mut self, pool_id: u32, new_rate: U256) {
        self.only_admin();
        
        let mut pool = self.pools.get(&pool_id)
            .unwrap_or_revert_with(&self.env(), FarmingError::PoolNotFound);
        
        self.update_pool_rewards(pool_id);
        
        let old_rate = pool.reward_rate;
        pool.reward_rate = new_rate;
        self.pools.set(&pool_id, pool);
        
        let admin = self.admin.get_or_revert_with(FarmingError::Unauthorized);
        self.env().emit_event(RewardRateUpdated {
            pool_id,
            old_rate,
            new_rate,
            updated_by: admin,
        });
    }
    
    /// Set pool active status
    pub fn set_pool_active(&mut self, pool_id: u32, active: bool) {
        self.only_admin();
        
        let mut pool = self.pools.get(&pool_id)
            .unwrap_or_revert_with(&self.env(), FarmingError::PoolNotFound);
        
        pool.is_active = active;
        self.pools.set(&pool_id, pool);
    }
    
    // ========================================
    // Staking Functions
    // ========================================
    
    /// Stake LP tokens
    pub fn stake(&mut self, pool_id: u32, amount: U256) {
        self.ensure_not_paused();
        
        if amount == U256::zero() {
            self.env().revert(FarmingError::ZeroAmount);
        }
        
        let caller = self.env().caller();
        
        // Get pool
        let pool = self.pools.get(&pool_id)
            .unwrap_or_revert_with(&self.env(), FarmingError::PoolNotFound);
        
        if !pool.is_active {
            self.env().revert(FarmingError::PoolNotActive);
        }
        
        // Update pool rewards
        self.update_pool_rewards(pool_id);
        
        // Update user rewards
        self.update_user_rewards(caller, pool_id);
        
        // Transfer LP tokens from user
        let mut lp_token = Cep18TokenContractRef::new(self.env(), pool.lp_token);
        lp_token.transfer_from(caller, Address::from(self.env().self_address()), amount);
        
        // Update user stake
        let mut user_stake = self.user_stakes.get(&(caller, pool_id))
            .unwrap_or(UserStake {
                amount: U256::zero(),
                reward_debt: U256::zero(),
                pending_rewards: U256::zero(),
                last_update: self.env().get_block_time(),
            });
        
        user_stake.amount = user_stake.amount + amount;
        user_stake.last_update = self.env().get_block_time();
        self.user_stakes.set(&(caller, pool_id), user_stake);
        
        // Update pool total
        let mut pool = self.pools.get(&pool_id).unwrap();
        pool.total_staked = pool.total_staked + amount;
        self.pools.set(&pool_id, pool);
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Staked {
            user: caller,
            pool_id,
            amount,
            timestamp,
        });
    }
    
    /// Unstake LP tokens
    pub fn unstake(&mut self, pool_id: u32, amount: U256) {
        self.ensure_not_paused();
        
        if amount == U256::zero() {
            self.env().revert(FarmingError::ZeroAmount);
        }
        
        let caller = self.env().caller();
        
        // Update pool rewards
        self.update_pool_rewards(pool_id);
        
        // Update user rewards
        self.update_user_rewards(caller, pool_id);
        
        // Get user stake
        let mut user_stake = self.user_stakes.get(&(caller, pool_id))
            .unwrap_or_revert_with(&self.env(), FarmingError::InsufficientBalance);
        
        if user_stake.amount < amount {
            self.env().revert(FarmingError::InsufficientBalance);
        }
        
        // Update user stake
        user_stake.amount = user_stake.amount - amount;
        user_stake.last_update = self.env().get_block_time();
        self.user_stakes.set(&(caller, pool_id), user_stake);
        
        // Get pool info before updating
        let mut pool = self.pools.get(&pool_id).unwrap();
        let lp_token_address = pool.lp_token;
        
        // Update pool total
        pool.total_staked = pool.total_staked - amount;
        self.pools.set(&pool_id, pool);
        
        // Transfer LP tokens back to user
        let mut lp_token = Cep18TokenContractRef::new(self.env(), lp_token_address);
        lp_token.transfer(caller, amount);
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Unstaked {
            user: caller,
            pool_id,
            amount,
            timestamp,
        });
    }
    
    /// Claim pending rewards
    pub fn claim_rewards(&mut self, pool_id: u32) {
        self.ensure_not_paused();
        
        let caller = self.env().caller();
        
        // Update pool rewards
        self.update_pool_rewards(pool_id);
        
        // Update user rewards
        self.update_user_rewards(caller, pool_id);
        
        // Get user stake
        let mut user_stake = self.user_stakes.get(&(caller, pool_id))
            .unwrap_or_revert_with(&self.env(), FarmingError::NoRewardsToClaim);
        
        let rewards = user_stake.pending_rewards;
        
        if rewards == U256::zero() {
            self.env().revert(FarmingError::NoRewardsToClaim);
        }
        
        // Reset pending rewards
        user_stake.pending_rewards = U256::zero();
        self.user_stakes.set(&(caller, pool_id), user_stake);
        
        // Transfer ECTO rewards to user
        let reward_token_address = self.reward_token.get_or_revert_with(FarmingError::Unauthorized);
        let mut reward_token = Cep18TokenContractRef::new(self.env(), reward_token_address);
        reward_token.transfer(caller, rewards);
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(RewardsClaimed {
            user: caller,
            pool_id,
            reward_amount: rewards,
            timestamp,
        });
    }
    
    // ========================================
    // Internal Functions
    // ========================================
    
    fn update_pool_rewards(&mut self, pool_id: u32) {
        let mut pool = self.pools.get(&pool_id).unwrap();
        
        if pool.total_staked == U256::zero() {
            pool.last_update = self.env().get_block_time();
            self.pools.set(&pool_id, pool);
            return;
        }
        
        let current_time = self.env().get_block_time();
        let time_elapsed = current_time - pool.last_update;
        
        // Calculate rewards: reward_rate * time_elapsed
        let rewards = pool.reward_rate * U256::from(time_elapsed);
        
        // Update reward per token
        let reward_per_token_increase = (rewards * U256::from(1_000_000_000_000_000_000u128)) / pool.total_staked;
        pool.reward_per_token_stored = pool.reward_per_token_stored + reward_per_token_increase;
        pool.last_update = current_time;
        
        self.pools.set(&pool_id, pool);
    }
    
    fn update_user_rewards(&mut self, user: Address, pool_id: u32) {
        let pool = self.pools.get(&pool_id).unwrap();
        let mut user_stake = self.user_stakes.get(&(user, pool_id))
            .unwrap_or(UserStake {
                amount: U256::zero(),
                reward_debt: U256::zero(),
                pending_rewards: U256::zero(),
                last_update: self.env().get_block_time(),
            });
        
        if user_stake.amount > U256::zero() {
            // Calculate pending rewards
            let reward_per_token_delta = pool.reward_per_token_stored - user_stake.reward_debt;
            let new_rewards = (user_stake.amount * reward_per_token_delta) / U256::from(1_000_000_000_000_000_000u128);
            user_stake.pending_rewards = user_stake.pending_rewards + new_rewards;
        }
        
        user_stake.reward_debt = pool.reward_per_token_stored;
        self.user_stakes.set(&(user, pool_id), user_stake);
    }
    
    // ========================================
    // View Functions
    // ========================================
    
    pub fn get_pool_info(&self, pool_id: u32) -> Option<PoolInfo> {
        self.pools.get(&pool_id)
    }
    
    pub fn get_user_stake(&self, user: Address, pool_id: u32) -> Option<UserStake> {
        self.user_stakes.get(&(user, pool_id))
    }
    
    pub fn get_pending_rewards(&self, user: Address, pool_id: u32) -> U256 {
        let user_stake = self.user_stakes.get(&(user, pool_id));
        if let Some(stake) = user_stake {
            stake.pending_rewards
        } else {
            U256::zero()
        }
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    pub fn pause(&mut self) {
        self.only_admin();
        self.paused.set(true);
    }
    
    pub fn unpause(&mut self) {
        self.only_admin();
        self.paused.set(false);
    }
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(FarmingError::Unauthorized);
        if caller != admin {
            self.env().revert(FarmingError::Unauthorized);
        }
    }
    
    fn ensure_not_paused(&self) {
        if self.paused.get_or_default() {
            self.env().revert(FarmingError::ContractPaused);
        }
    }
}
