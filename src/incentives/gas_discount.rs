//! Gas Discount Manager
//! 
//! Provides tiered gas discounts based on native token holdings (sCSPR and aECTO).
//! Leverages Casper 2.0's fee elimination features when available.
//! 
//! Discount Tiers:
//! - Tier 0: No holdings → 0% discount
//! - Tier 1: 100+ sCSPR or 1,000+ aECTO → 10% discount
//! - Tier 2: 500+ sCSPR or 5,000+ aECTO → 25% discount
//! - Tier 3: 2,000+ sCSPR or 20,000+ aECTO → 40% discount
//! - Tier 4: 10,000+ sCSPR or 100,000+ aECTO → 60% discount

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use crate::token::Cep18TokenContractRef;

/// Gas discount tier
#[odra::odra_type]
pub struct DiscountTier {
    /// Tier level (0-4)
    pub tier: u8,
    /// Discount percentage (0-60)
    pub discount_percent: u8,
    /// Minimum sCSPR balance required
    pub min_scspr: U256,
    /// Minimum aECTO balance required (alternative)
    pub min_aecto: U256,
}

/// User's discount information
#[odra::odra_type]
pub struct UserDiscount {
    /// User address
    pub user: Address,
    /// Current tier
    pub tier: u8,
    /// Discount percentage
    pub discount_percent: u8,
    /// Last check timestamp
    pub last_check: u64,
}

/// Gas Discount Manager contract
#[odra::module]
pub struct GasDiscountManager {
    /// sCSPR token address
    scspr_token: Var<Address>,
    /// aECTO token address
    aecto_token: Var<Address>,
    /// Treasury address for gas subsidies
    treasury: Var<Address>,
    /// Admin address
    admin: Var<Address>,
    /// Discount tiers (tier_level -> DiscountTier)
    tiers: Mapping<u8, DiscountTier>,
    /// User discount cache (user -> UserDiscount)
    user_discounts: Mapping<Address, UserDiscount>,
    /// Whether gas discounts are enabled
    enabled: Var<bool>,
    /// Total gas subsidized (in motes)
    total_subsidized: Var<U256>,
    /// Cache validity period (seconds)
    cache_validity: Var<u64>,
}

#[odra::module]
impl GasDiscountManager {
    /// Initialize the gas discount manager
    pub fn init(
        &mut self,
        scspr_token_address: Address,
        aecto_token_address: Address,
        treasury_address: Address,
    ) {
        let caller = self.env().caller();
        
        self.scspr_token.set(scspr_token_address);
        self.aecto_token.set(aecto_token_address);
        self.treasury.set(treasury_address);
        self.admin.set(caller);
        self.enabled.set(true);
        self.total_subsidized.set(U256::zero());
        self.cache_validity.set(300); // 5 minutes default
        
        // Initialize discount tiers
        self.initialize_tiers();
    }
    
    /// Initialize the default discount tiers
    fn initialize_tiers(&mut self) {
        // Tier 0: No discount
        self.tiers.set(&0, DiscountTier {
            tier: 0,
            discount_percent: 0,
            min_scspr: U256::zero(),
            min_aecto: U256::zero(),
        });
        
        // Tier 1: 10% discount
        // 100 sCSPR (9 decimals) or 1,000 aECTO (18 decimals)
        self.tiers.set(&1, DiscountTier {
            tier: 1,
            discount_percent: 10,
            min_scspr: U256::from(100) * U256::from(10u128.pow(9)),
            min_aecto: U256::from(1000) * U256::from(10u128.pow(18)),
        });
        
        // Tier 2: 25% discount
        // 500 sCSPR or 5,000 aECTO
        self.tiers.set(&2, DiscountTier {
            tier: 2,
            discount_percent: 25,
            min_scspr: U256::from(500) * U256::from(10u128.pow(9)),
            min_aecto: U256::from(5000) * U256::from(10u128.pow(18)),
        });
        
        // Tier 3: 40% discount
        // 2,000 sCSPR or 20,000 aECTO
        self.tiers.set(&3, DiscountTier {
            tier: 3,
            discount_percent: 40,
            min_scspr: U256::from(2000) * U256::from(10u128.pow(9)),
            min_aecto: U256::from(20000) * U256::from(10u128.pow(18)),
        });
        
        // Tier 4: 60% discount
        // 10,000 sCSPR or 100,000 aECTO
        self.tiers.set(&4, DiscountTier {
            tier: 4,
            discount_percent: 60,
            min_scspr: U256::from(10000) * U256::from(10u128.pow(9)),
            min_aecto: U256::from(100000) * U256::from(10u128.pow(18)),
        });
    }
    
    /// Get the discount tier for a user
    /// Checks both sCSPR and aECTO balances and returns the highest tier
    pub fn get_user_tier(&mut self, user: Address) -> u8 {
        if !self.enabled.get_or_default() {
            return 0;
        }
        
        // Check cache first
        if let Some(cached) = self.user_discounts.get(&user) {
            let current_time = self.env().get_block_time();
            let cache_validity = self.cache_validity.get_or_default();
            
            if current_time - cached.last_check < cache_validity {
                return cached.tier;
            }
        }
        
        // Get user balances
        let scspr_balance = self.get_scspr_balance(user);
        let aecto_balance = self.get_aecto_balance(user);
        
        // Determine tier (check from highest to lowest)
        let tier = if scspr_balance >= self.tiers.get(&4).unwrap().min_scspr 
                      || aecto_balance >= self.tiers.get(&4).unwrap().min_aecto {
            4
        } else if scspr_balance >= self.tiers.get(&3).unwrap().min_scspr 
                  || aecto_balance >= self.tiers.get(&3).unwrap().min_aecto {
            3
        } else if scspr_balance >= self.tiers.get(&2).unwrap().min_scspr 
                  || aecto_balance >= self.tiers.get(&2).unwrap().min_aecto {
            2
        } else if scspr_balance >= self.tiers.get(&1).unwrap().min_scspr 
                  || aecto_balance >= self.tiers.get(&1).unwrap().min_aecto {
            1
        } else {
            0
        };
        
        // Update cache
        let discount_percent = self.tiers.get(&tier).unwrap().discount_percent;
        self.user_discounts.set(&user, UserDiscount {
            user,
            tier,
            discount_percent,
            last_check: self.env().get_block_time(),
        });
        
        tier
    }
    
    /// Get the discount percentage for a user
    pub fn get_discount_percent(&mut self, user: Address) -> u8 {
        let tier = self.get_user_tier(user);
        self.tiers.get(&tier).unwrap().discount_percent
    }
    
    /// Calculate the gas subsidy amount for a transaction
    /// Returns the amount to subsidize from treasury
    pub fn calculate_subsidy(&mut self, user: Address, gas_cost: U256) -> U256 {
        if !self.enabled.get_or_default() {
            return U256::zero();
        }
        
        let discount_percent = self.get_discount_percent(user);
        
        if discount_percent == 0 {
            return U256::zero();
        }
        
        // Calculate subsidy: gas_cost * discount_percent / 100
        let subsidy = gas_cost * U256::from(discount_percent) / U256::from(100);
        
        subsidy
    }
    
    /// Record a gas subsidy (called by integrated contracts)
    /// This tracks total subsidies for analytics
    pub fn record_subsidy(&mut self, user: Address, amount: U256) {
        if !self.enabled.get_or_default() {
            return;
        }
        
        let total = self.total_subsidized.get_or_default();
        self.total_subsidized.set(total + amount);
        
        // Emit event
        self.env().emit_event(GasSubsidyApplied {
            user,
            amount,
            timestamp: self.env().get_block_time(),
        });
    }
    
    // ========================================
    // Helper Functions
    // ========================================
    
    /// Get user's sCSPR balance
    fn get_scspr_balance(&self, user: Address) -> U256 {
        let scspr_address = match self.scspr_token.get() {
            Some(addr) => addr,
            None => return U256::zero(),
        };
        
        let token = Cep18TokenContractRef::new(self.env(), scspr_address);
        token.balance_of(user)
    }
    
    /// Get user's aECTO balance
    fn get_aecto_balance(&self, user: Address) -> U256 {
        let aecto_address = match self.aecto_token.get() {
            Some(addr) => addr,
            None => return U256::zero(),
        };
        
        let token = Cep18TokenContractRef::new(self.env(), aecto_address);
        token.balance_of(user)
    }
    
    // ========================================
    // View Functions
    // ========================================
    
    /// Get tier information
    pub fn get_tier_info(&self, tier: u8) -> Option<DiscountTier> {
        self.tiers.get(&tier)
    }
    
    /// Get user's cached discount info
    pub fn get_user_discount_info(&self, user: Address) -> Option<UserDiscount> {
        self.user_discounts.get(&user)
    }
    
    /// Get total gas subsidized
    pub fn get_total_subsidized(&self) -> U256 {
        self.total_subsidized.get_or_default()
    }
    
    /// Check if discounts are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.get_or_default()
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    /// Update a discount tier
    pub fn update_tier(
        &mut self,
        tier: u8,
        discount_percent: u8,
        min_scspr: U256,
        min_aecto: U256,
    ) {
        self.only_admin();
        
        if tier > 4 {
            self.env().revert(DexError::InvalidConfiguration);
        }
        
        if discount_percent > 100 {
            self.env().revert(DexError::InvalidConfiguration);
        }
        
        self.tiers.set(&tier, DiscountTier {
            tier,
            discount_percent,
            min_scspr,
            min_aecto,
        });
        
        self.env().emit_event(TierUpdated {
            tier,
            discount_percent,
            updated_by: self.env().caller(),
        });
    }
    
    /// Enable or disable gas discounts
    pub fn set_enabled(&mut self, enabled: bool) {
        self.only_admin();
        self.enabled.set(enabled);
        
        self.env().emit_event(DiscountsToggled {
            enabled,
            toggled_by: self.env().caller(),
        });
    }
    
    /// Update cache validity period
    pub fn set_cache_validity(&mut self, seconds: u64) {
        self.only_admin();
        self.cache_validity.set(seconds);
    }
    
    /// Update treasury address
    pub fn set_treasury(&mut self, new_treasury: Address) {
        self.only_admin();
        self.treasury.set(new_treasury);
    }
    
    // Note: Odra Mapping doesn't support remove()
    // Cache will be invalidated after cache_validity period expires
    // /// Clear user's discount cache (force recalculation)
    // pub fn clear_user_cache(&mut self, user: Address) {
    //     self.only_admin();
    //     // self.user_discounts.remove(&user);
    // }
    
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
// Events
// ========================================

/// Emitted when a gas subsidy is applied
#[odra::event]
pub struct GasSubsidyApplied {
    pub user: Address,
    pub amount: U256,
    pub timestamp: u64,
}

/// Emitted when a tier is updated
#[odra::event]
pub struct TierUpdated {
    pub tier: u8,
    pub discount_percent: u8,
    pub updated_by: Address,
}

/// Emitted when discounts are enabled/disabled
#[odra::event]
pub struct DiscountsToggled {
    pub enabled: bool,
    pub toggled_by: Address,
}

// Import error type
use crate::errors::DexError;

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    #[test]
    fn test_tier_calculation() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let user = env.get_account(1);
        
        // Deploy mock tokens
        let scspr_token = env.get_account(10);
        let aecto_token = env.get_account(11);
        let treasury = env.get_account(12);
        
        env.set_caller(admin);
        let init_args = GasDiscountManagerInitArgs {
            scspr_token_address: scspr_token,
            aecto_token_address: aecto_token,
            treasury_address: treasury,
        };
        
        let manager = GasDiscountManager::deploy(&env, init_args);
        
        // Test tier 0 (no holdings)
        let tier = manager.get_user_tier(user);
        assert_eq!(tier, 0);
        
        let discount = manager.get_discount_percent(user);
        assert_eq!(discount, 0);
    }
    
    #[test]
    fn test_subsidy_calculation() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let user = env.get_account(1);
        
        let scspr_token = env.get_account(10);
        let aecto_token = env.get_account(11);
        let treasury = env.get_account(12);
        
        env.set_caller(admin);
        let init_args = GasDiscountManagerInitArgs {
            scspr_token_address: scspr_token,
            aecto_token_address: aecto_token,
            treasury_address: treasury,
        };
        
        let manager = GasDiscountManager::deploy(&env, init_args);
        
        // Test 0% discount
        let gas_cost = U256::from(1000);
        let subsidy = manager.calculate_subsidy(user, gas_cost);
        assert_eq!(subsidy, U256::zero());
    }
}
