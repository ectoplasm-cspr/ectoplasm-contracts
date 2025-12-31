//! Incentive Manager
//! 
//! Main coordinator for all incentive mechanisms across the protocol.
//! Integrates gas discounts and LP boost rewards.
//! Manages treasury and emission schedules.

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use super::gas_discount::GasDiscountManagerContractRef;
use super::lp_rewards_distributor::LpRewardsDistributorContractRef;
use crate::token::Cep18TokenContractRef;

/// Protocol statistics
#[odra::odra_type]
#[derive(Default)]
pub struct ProtocolStats {
    /// Total value locked across all protocols
    pub total_tvl: U256,
    /// Total gas subsidized
    pub total_gas_subsidized: U256,
    /// Total LP rewards distributed
    pub total_lp_rewards: U256,
    /// Number of active users
    pub active_users: u32,
    /// Last update timestamp
    pub last_update: u64,
}

/// User participation metrics
#[odra::odra_type]
pub struct UserMetrics {
    /// User address
    pub user: Address,
    /// Has LST position (sCSPR)
    pub has_lst: bool,
    /// Has yield position (aECTO)
    pub has_yield: bool,
    /// Has DEX LP position
    pub has_dex_lp: bool,
    /// Is borrower
    pub is_borrower: bool,
    /// Gas discount tier
    pub gas_tier: u8,
    /// LP boost multiplier
    pub lp_boost: U256,
    /// Total rewards earned
    pub total_rewards: U256,
}

/// Incentive Manager contract
#[odra::module]
pub struct IncentiveManager {
    /// Gas discount manager address
    gas_discount_manager: Var<Address>,
    /// Rewards distributor address
    rewards_distributor: Var<Address>,
    /// Treasury address
    treasury: Var<Address>,
    /// Admin address
    admin: Var<Address>,
    /// Protocol stats
    protocol_stats: Var<ProtocolStats>,
    /// User metrics (user -> UserMetrics)
    user_metrics: Mapping<Address, UserMetrics>,
    /// Registered users
    registered_users: Mapping<u32, Address>,
    /// User count
    user_count: Var<u32>,
    /// Treasury allocation percentages (scaled by 100)
    /// 40% gas subsidy, 30% LP rewards, 20% development, 10% reserves
    gas_subsidy_allocation: Var<u8>,
    lp_rewards_allocation: Var<u8>,
    development_allocation: Var<u8>,
    reserves_allocation: Var<u8>,
    /// Total treasury balance
    treasury_balance: Var<U256>,
}

#[odra::module]
impl IncentiveManager {
    /// Initialize the incentive manager
    pub fn init(
        &mut self,
        gas_discount_manager_address: Address,
        rewards_distributor_address: Address,
        treasury_address: Address,
    ) {
        let caller = self.env().caller();
        
        self.gas_discount_manager.set(gas_discount_manager_address);
        self.rewards_distributor.set(rewards_distributor_address);
        self.treasury.set(treasury_address);
        self.admin.set(caller);
        
        // Initialize protocol stats
        self.protocol_stats.set(ProtocolStats {
            total_tvl: U256::zero(),
            total_gas_subsidized: U256::zero(),
            total_lp_rewards: U256::zero(),
            active_users: 0,
            last_update: self.env().get_block_time(),
        });
        
        self.user_count.set(0);
        self.treasury_balance.set(U256::zero());
        
        // Set default treasury allocations
        self.gas_subsidy_allocation.set(40); // 40%
        self.lp_rewards_allocation.set(30);  // 30%
        self.development_allocation.set(20); // 20%
        self.reserves_allocation.set(10);    // 10%
    }
    
    /// Register a user's participation in the protocol
    /// This should be called when users interact with any protocol component
    pub fn register_user_activity(
        &mut self,
        user: Address,
        has_lst: bool,
        has_yield: bool,
        has_dex_lp: bool,
        is_borrower: bool,
    ) {
        // Get or create user metrics
        let mut metrics = self.user_metrics.get(&user).unwrap_or(UserMetrics {
            user,
            has_lst: false,
            has_yield: false,
            has_dex_lp: false,
            is_borrower: false,
            gas_tier: 0,
            lp_boost: U256::from(10u128.pow(18)), // 1.0x default
            total_rewards: U256::zero(),
        });
        
        // Check if this is a new user
        let is_new_user = !metrics.has_lst && !metrics.has_yield && !metrics.has_dex_lp;
        
        // Update metrics
        metrics.has_lst = has_lst;
        metrics.has_yield = has_yield;
        metrics.has_dex_lp = has_dex_lp;
        metrics.is_borrower = is_borrower;
        
        // Update gas tier from gas discount manager
        if let Some(gas_manager_address) = self.gas_discount_manager.get() {
            let mut gas_manager = GasDiscountManagerContractRef::new(self.env(), gas_manager_address);
            metrics.gas_tier = gas_manager.get_user_tier(user);
        }
        
        // Update LP boost from rewards distributor
        if let Some(rewards_address) = self.rewards_distributor.get() {
            let rewards_dist = LpRewardsDistributorContractRef::new(self.env(), rewards_address);
            let boost_factors = rewards_dist.get_boost_factors(user);
            metrics.lp_boost = boost_factors.total_multiplier;
        }
        
        self.user_metrics.set(&user, metrics);
        
        // If new user, add to registry
        if is_new_user {
            let count = self.user_count.get_or_default();
            self.registered_users.set(&count, user);
            self.user_count.set(count + 1);
            
            // Update active users count
            let mut stats = self.protocol_stats.get_or_default();
            stats.active_users = count + 1;
            self.protocol_stats.set(stats);
        }
        
        self.env().emit_event(UserActivityRegistered {
            user,
            has_lst,
            has_yield,
            has_dex_lp,
            is_borrower,
            timestamp: self.env().get_block_time(),
        });
    }
    
    /// Process a DEX transaction with gas discount
    /// Called by DEX router before executing swaps/liquidity operations
    pub fn process_dex_transaction(
        &mut self,
        user: Address,
        estimated_gas: U256,
    ) -> U256 {
        let gas_manager_address = match self.gas_discount_manager.get() {
            Some(addr) => addr,
            None => return U256::zero(),
        };
        
        let mut gas_manager = GasDiscountManagerContractRef::new(self.env(), gas_manager_address);
        
        // Calculate subsidy
        let subsidy = gas_manager.calculate_subsidy(user, estimated_gas);
        
        if subsidy > U256::zero() {
            // Record the subsidy
            gas_manager.record_subsidy(user, subsidy);
            
            // Update protocol stats
            let mut stats = self.protocol_stats.get_or_default();
            stats.total_gas_subsidized = stats.total_gas_subsidized + subsidy;
            self.protocol_stats.set(stats);
            
            // Update user metrics
            if let Some(mut metrics) = self.user_metrics.get(&user) {
                metrics.total_rewards = metrics.total_rewards + subsidy;
                self.user_metrics.set(&user, metrics);
            }
        }
        
        subsidy
    }
    
    /// Allocate treasury funds to different pools
    pub fn allocate_treasury_funds(&mut self, amount: U256) {
        self.only_admin();
        
        let gas_allocation = self.gas_subsidy_allocation.get_or_default();
        let lp_allocation = self.lp_rewards_allocation.get_or_default();
        let dev_allocation = self.development_allocation.get_or_default();
        let reserves_allocation = self.reserves_allocation.get_or_default();
        
        // Calculate amounts
        let gas_amount = amount * U256::from(gas_allocation) / U256::from(100);
        let lp_amount = amount * U256::from(lp_allocation) / U256::from(100);
        let dev_amount = amount * U256::from(dev_allocation) / U256::from(100);
        let reserves_amount = amount * U256::from(reserves_allocation) / U256::from(100);
        
        // Note: In production, you would transfer these amounts to respective pools
        // For now, we just emit events
        
        self.env().emit_event(TreasuryAllocated {
            total_amount: amount,
            gas_subsidy: gas_amount,
            lp_rewards: lp_amount,
            development: dev_amount,
            reserves: reserves_amount,
            timestamp: self.env().get_block_time(),
        });
    }
    
    /// Calculate total APY for a user across all protocol components
    pub fn calculate_total_apy(&self, user: Address) -> U256 {
        let metrics = self.user_metrics.get(&user);
        if metrics.is_none() {
            return U256::zero();
        }
        
        let metrics = metrics.unwrap();
        let mut total_apy = U256::zero();
        
        // LST staking APY (~8%)
        if metrics.has_lst {
            total_apy = total_apy + U256::from(8);
        }
        
        // Yield protocol APY (~8-12% depending on utilization)
        if metrics.has_yield {
            total_apy = total_apy + U256::from(10); // Average 10%
        }
        
        // DEX LP APY with boost (~15% base * boost multiplier)
        if metrics.has_dex_lp {
            let base_lp_apy = U256::from(15);
            let scale = U256::from(10u128.pow(18));
            let boosted_lp_apy = base_lp_apy * metrics.lp_boost / scale;
            total_apy = total_apy + boosted_lp_apy;
        }
        
        // Gas savings (estimate ~1-2% additional value)
        if metrics.gas_tier > 0 {
            total_apy = total_apy + U256::from(1);
        }
        
        total_apy
    }
    
    /// Get comprehensive user dashboard data
    pub fn get_user_dashboard(&self, user: Address) -> UserDashboard {
        let metrics = self.user_metrics.get(&user).unwrap_or(UserMetrics {
            user,
            has_lst: false,
            has_yield: false,
            has_dex_lp: false,
            is_borrower: false,
            gas_tier: 0,
            lp_boost: U256::from(10u128.pow(18)),
            total_rewards: U256::zero(),
        });
        
        let total_apy = self.calculate_total_apy(user);
        
        // Get gas discount percentage
        let gas_discount = if metrics.gas_tier > 0 {
            match metrics.gas_tier {
                1 => 10,
                2 => 25,
                3 => 40,
                4 => 60,
                _ => 0,
            }
        } else {
            0
        };
        
        UserDashboard {
            user,
            metrics,
            total_apy,
            gas_discount_percent: gas_discount,
            protocol_stats: self.protocol_stats.get_or_default(),
        }
    }
    
    // ========================================
    // View Functions
    // ========================================
    
    /// Get protocol statistics
    pub fn get_protocol_stats(&self) -> ProtocolStats {
        self.protocol_stats.get_or_default()
    }
    
    /// Get user metrics
    pub fn get_user_metrics(&self, user: Address) -> Option<UserMetrics> {
        self.user_metrics.get(&user)
    }
    
    /// Get treasury balance
    pub fn get_treasury_balance(&self) -> U256 {
        self.treasury_balance.get_or_default()
    }
    
    /// Get total active users
    pub fn get_active_users_count(&self) -> u32 {
        self.user_count.get_or_default()
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    /// Update treasury allocation percentages
    pub fn update_treasury_allocation(
        &mut self,
        gas_subsidy: u8,
        lp_rewards: u8,
        development: u8,
        reserves: u8,
    ) {
        self.only_admin();
        
        // Ensure total is 100%
        if gas_subsidy + lp_rewards + development + reserves != 100 {
            self.env().revert(DexError::InvalidConfiguration);
        }
        
        self.gas_subsidy_allocation.set(gas_subsidy);
        self.lp_rewards_allocation.set(lp_rewards);
        self.development_allocation.set(development);
        self.reserves_allocation.set(reserves);
        
        self.env().emit_event(AllocationUpdated {
            gas_subsidy,
            lp_rewards,
            development,
            reserves,
            updated_by: self.env().caller(),
        });
    }
    
    /// Update protocol stats (called periodically by keeper or admin)
    pub fn update_protocol_stats(
        &mut self,
        total_tvl: U256,
    ) {
        self.only_admin();
        
        let mut stats = self.protocol_stats.get_or_default();
        stats.total_tvl = total_tvl;
        stats.last_update = self.env().get_block_time();
        
        self.protocol_stats.set(stats);
    }
    
    /// Deposit funds to treasury
    pub fn deposit_to_treasury(&mut self, amount: U256) {
        let caller = self.env().caller();
        
        // In production, transfer tokens here
        // For now, just update balance
        let current = self.treasury_balance.get_or_default();
        self.treasury_balance.set(current + amount);
        
        self.env().emit_event(TreasuryDeposit {
            amount,
            deposited_by: caller,
            timestamp: self.env().get_block_time(),
        });
    }
    
    // ========================================
    // Helper Functions
    // ========================================
    
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
// Data Structures
// ========================================

/// User dashboard data
#[odra::odra_type]
pub struct UserDashboard {
    pub user: Address,
    pub metrics: UserMetrics,
    pub total_apy: U256,
    pub gas_discount_percent: u8,
    pub protocol_stats: ProtocolStats,
}

// ========================================
// Events
// ========================================

#[odra::event]
pub struct UserActivityRegistered {
    pub user: Address,
    pub has_lst: bool,
    pub has_yield: bool,
    pub has_dex_lp: bool,
    pub is_borrower: bool,
    pub timestamp: u64,
}

#[odra::event]
pub struct TreasuryAllocated {
    pub total_amount: U256,
    pub gas_subsidy: U256,
    pub lp_rewards: U256,
    pub development: U256,
    pub reserves: U256,
    pub timestamp: u64,
}

#[odra::event]
pub struct AllocationUpdated {
    pub gas_subsidy: u8,
    pub lp_rewards: u8,
    pub development: u8,
    pub reserves: u8,
    pub updated_by: Address,
}

#[odra::event]
pub struct TreasuryDeposit {
    pub amount: U256,
    pub deposited_by: Address,
    pub timestamp: u64,
}

use crate::errors::DexError;

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    #[test]
    fn test_user_registration() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        
        let gas_manager = env.get_account(10);
        let rewards_dist = env.get_account(11);
        let treasury = env.get_account(12);
        
        env.set_caller(admin);
        let init_args = IncentiveManagerInitArgs {
            gas_discount_manager_address: gas_manager,
            rewards_distributor_address: rewards_dist,
            treasury_address: treasury,
        };
        
        let mut manager = IncentiveManager::deploy(&env, init_args);
        
        let user = env.get_account(1);
        
        // Register user activity
        manager.register_user_activity(user, true, true, true, false);
        
        // Check metrics
        let metrics = manager.get_user_metrics(user);
        assert!(metrics.is_some());
        
        let metrics = metrics.unwrap();
        assert_eq!(metrics.has_lst, true);
        assert_eq!(metrics.has_yield, true);
        assert_eq!(metrics.has_dex_lp, true);
        
        // Check active users count
        let count = manager.get_active_users_count();
        assert_eq!(count, 1);
    }
    
    #[test]
    fn test_apy_calculation() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        
        let gas_manager = env.get_account(10);
        let rewards_dist = env.get_account(11);
        let treasury = env.get_account(12);
        
        env.set_caller(admin);
        let init_args = IncentiveManagerInitArgs {
            gas_discount_manager_address: gas_manager,
            rewards_distributor_address: rewards_dist,
            treasury_address: treasury,
        };
        
        let mut manager = IncentiveManager::deploy(&env, init_args);
        
        let user = env.get_account(1);
        
        // Register user with all protocol participation
        manager.register_user_activity(user, true, true, true, false);
        
        // Calculate total APY
        let total_apy = manager.calculate_total_apy(user);
        
        // Should be: 8% (LST) + 10% (Yield) + 15% (LP base) + 1% (gas) = 34%
        assert!(total_apy >= U256::from(30)); // At least 30%
    }
}
