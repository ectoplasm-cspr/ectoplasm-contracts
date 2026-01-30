//! Collateral Manager - Manages collateral deposits and health factors
//! 
//! Handles:
//! - Collateral configuration (LTV, liquidation thresholds)
//! - User collateral deposits and withdrawals
//! - Health factor calculations
//! - Collateral valuation

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use super::errors::LendingError;
use super::events::*;
use super::price_oracle::PriceOracleContractRef;
use crate::token::Cep18TokenContractRef;

/// Collateral configuration for an asset
#[odra::odra_type]
pub struct CollateralConfig {
    /// Asset address
    pub asset: Address,
    /// Loan-to-value ratio (scaled by 1e18)
    /// Example: 80% = 0.80 * 1e18 = 800000000000000000
    pub ltv: U256,
    /// Liquidation threshold (scaled by 1e18)
    /// Example: 85% = 0.85 * 1e18 = 850000000000000000
    pub liquidation_threshold: U256,
    /// Liquidation bonus for liquidators (scaled by 1e18)
    /// Example: 5% = 0.05 * 1e18 = 50000000000000000
    pub liquidation_bonus: U256,
    /// Whether collateral is enabled
    pub is_enabled: bool,
}

/// User's collateral position
#[odra::odra_type]
pub struct CollateralPosition {
    /// User address
    pub user: Address,
    /// Collateral asset
    pub asset: Address,
    /// Amount deposited
    pub amount: U256,
}

/// Collateral Manager contract
#[odra::module]
pub struct CollateralManager {
    /// Collateral configurations
    collateral_configs: Mapping<Address, CollateralConfig>,
    
    /// User collateral balances: (user, asset) -> amount
    user_collateral: Mapping<(Address, Address), U256>,
    
    /// List of collateral assets for a user
    user_collateral_assets: Mapping<(Address, u32), Address>,
    
    /// Number of collateral assets for a user
    user_collateral_count: Mapping<Address, u32>,
    
    /// Price oracle reference
    price_oracle: Var<Address>,
    
    /// Admin address
    admin: Var<Address>,
    
    /// Scale factor (1e18)
    scale: Var<U256>,
    
    /// Minimum health factor (scaled by 1e18)
    /// Example: 1.0 = 1e18
    min_health_factor: Var<U256>,
}

#[odra::module]
impl CollateralManager {
    /// Initialize the collateral manager
    /// 
    /// # Arguments
    /// * `price_oracle_address` - Address of the price oracle
    pub fn init(&mut self, price_oracle_address: Address) {
        let caller = self.env().caller();
        self.admin.set(caller);
        self.price_oracle.set(price_oracle_address);
        self.scale.set(U256::from(1_000_000_000_000_000_000u128)); // 1e18
        self.min_health_factor.set(U256::from(1_000_000_000_000_000_000u128)); // 1.0
    }
    
    // ========================================
    // Collateral Configuration (Admin)
    // ========================================
    
    /// Add a new collateral type
    /// 
    /// # Arguments
    /// * `asset` - Collateral asset address
    /// * `ltv` - Loan-to-value ratio (scaled by 1e18)
    /// * `liquidation_threshold` - Liquidation threshold (scaled by 1e18)
    /// * `liquidation_bonus` - Liquidation bonus (scaled by 1e18)
    pub fn add_collateral(
        &mut self,
        asset: Address,
        ltv: U256,
        liquidation_threshold: U256,
        liquidation_bonus: U256,
    ) {
        self.only_admin();
        
        // Validate parameters
        let scale = self.scale.get_or_default();
        if ltv > scale || liquidation_threshold > scale {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        if ltv > liquidation_threshold {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        
        let config = CollateralConfig {
            asset,
            ltv,
            liquidation_threshold,
            liquidation_bonus,
            is_enabled: true,
        };
        
        self.collateral_configs.set(&asset, config);
        
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        self.env().emit_event(CollateralAdded {
            asset,
            ltv,
            liquidation_threshold,
            liquidation_bonus,
            added_by: admin,
        });
    }
    
    /// Update collateral parameters
    pub fn update_collateral(
        &mut self,
        asset: Address,
        ltv: U256,
        liquidation_threshold: U256,
        liquidation_bonus: U256,
    ) {
        self.only_admin();
        
        let mut config = self.collateral_configs.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral);
        
        // Validate parameters
        let scale = self.scale.get_or_default();
        if ltv > scale || liquidation_threshold > scale {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        if ltv > liquidation_threshold {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        
        config.ltv = ltv;
        config.liquidation_threshold = liquidation_threshold;
        config.liquidation_bonus = liquidation_bonus;
        
        self.collateral_configs.set(&asset, config);
        
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        self.env().emit_event(CollateralUpdated {
            asset,
            ltv,
            liquidation_threshold,
            liquidation_bonus,
            updated_by: admin,
        });
    }
    
    /// Enable/disable a collateral type
    pub fn set_collateral_enabled(&mut self, asset: Address, enabled: bool) {
        self.only_admin();
        
        let mut config = self.collateral_configs.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral);
        
        config.is_enabled = enabled;
        self.collateral_configs.set(&asset, config);
    }
    
    // ========================================
    // Collateral Deposits/Withdrawals
    // ========================================
    
    /// Deposit collateral
    /// 
    /// # Arguments
    /// * `asset` - Collateral asset address
    /// * `amount` - Amount to deposit
    pub fn deposit_collateral(&mut self, asset: Address, amount: U256) {
        let caller = self.env().caller();
        
        if amount == U256::zero() {
            self.env().revert(LendingError::ZeroAmount);
        }
        
        // Check if collateral is supported and enabled
        let config = self.collateral_configs.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral);
        
        if !config.is_enabled {
            self.env().revert(LendingError::CollateralDisabled);
        }
        
        // Transfer collateral from user to contract
        let mut token = Cep18TokenContractRef::new(self.env(), asset);
        token.transfer_from(caller, Address::from(self.env().self_address()), amount);
        
        // Update user's collateral balance
        let current_balance = self.user_collateral.get(&(caller, asset)).unwrap_or(U256::zero());
        let new_balance = current_balance + amount;
        self.user_collateral.set(&(caller, asset), new_balance);
        
        // Add to user's collateral asset list if first deposit
        if current_balance == U256::zero() {
            let count = self.user_collateral_count.get(&caller).unwrap_or(0);
            self.user_collateral_assets.set(&(caller, count), asset);
            self.user_collateral_count.set(&caller, count + 1);
        }
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(CollateralDeposited {
            user: caller,
            asset,
            amount,
            timestamp,
        });
    }
    
    /// Withdraw collateral
    /// 
    /// # Arguments
    /// * `asset` - Collateral asset address
    /// * `amount` - Amount to withdraw
    /// * `user_debt` - User's current debt (for health factor check)
    pub fn withdraw_collateral(&mut self, asset: Address, amount: U256, user_debt: U256) {
        let caller = self.env().caller();
        
        if amount == U256::zero() {
            self.env().revert(LendingError::ZeroAmount);
        }
        
        // Check user has sufficient collateral
        let current_balance = self.user_collateral.get(&(caller, asset))
            .unwrap_or_revert_with(&self.env(), LendingError::InsufficientCollateralDeposit);
        
        if current_balance < amount {
            self.env().revert(LendingError::InsufficientCollateralDeposit);
        }
        
        // Calculate health factor after withdrawal
        let new_balance = current_balance - amount;
        self.user_collateral.set(&(caller, asset), new_balance);
        
        let health_factor = self.calculate_health_factor_internal(caller, user_debt);
        let min_health = self.min_health_factor.get_or_default();
        
        if user_debt > U256::zero() && health_factor < min_health {
            // Revert the change
            self.user_collateral.set(&(caller, asset), current_balance);
            self.env().revert(LendingError::CannotWithdrawCollateral);
        }
        
        // Transfer collateral back to user
        let mut token = Cep18TokenContractRef::new(self.env(), asset);
        token.transfer(caller, amount);
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(CollateralWithdrawn {
            user: caller,
            asset,
            amount,
            timestamp,
        });
    }
    
    // ========================================
    // Health Factor Calculations
    // ========================================
    
    /// Calculate user's health factor
    /// 
    /// Health Factor = (Collateral Value * Liquidation Threshold) / Debt
    /// 
    /// # Arguments
    /// * `user` - User address
    /// * `debt` - User's debt amount
    /// 
    /// # Returns
    /// Health factor (scaled by 1e18). Returns U256::MAX if no debt.
    pub fn calculate_health_factor(&self, user: Address, debt: U256) -> U256 {
        self.calculate_health_factor_internal(user, debt)
    }
    
    fn calculate_health_factor_internal(&self, user: Address, debt: U256) -> U256 {
        if debt == U256::zero() {
            return U256::MAX;
        }
        
        let collateral_value = self.get_user_collateral_value_with_threshold(user);
        
        if collateral_value == U256::zero() {
            return U256::zero();
        }
        
        let scale = self.scale.get_or_default();
        // health_factor = (collateral_value * scale) / debt
        (collateral_value * scale) / debt
    }
    
    /// Get user's total collateral value in ECTO
    pub fn get_user_collateral_value(&self, user: Address) -> U256 {
        let oracle_address = self.price_oracle.get_or_revert_with(LendingError::OracleNotInitialized);
        let oracle = PriceOracleContractRef::new(self.env(), oracle_address);
        
        let count = self.user_collateral_count.get(&user).unwrap_or(0);
        let mut total_value = U256::zero();
        
        for i in 0..count {
            if let Some(asset) = self.user_collateral_assets.get(&(user, i)) {
                if let Some(amount) = self.user_collateral.get(&(user, asset)) {
                    if amount > U256::zero() {
                        let value = oracle.get_asset_value(asset, amount);
                        total_value = total_value + value;
                    }
                }
            }
        }
        
        total_value
    }
    
    /// Get user's collateral value weighted by liquidation threshold
    fn get_user_collateral_value_with_threshold(&self, user: Address) -> U256 {
        let oracle_address = self.price_oracle.get_or_revert_with(LendingError::OracleNotInitialized);
        let oracle = PriceOracleContractRef::new(self.env(), oracle_address);
        let scale = self.scale.get_or_default();
        
        let count = self.user_collateral_count.get(&user).unwrap_or(0);
        let mut total_value = U256::zero();
        
        for i in 0..count {
            if let Some(asset) = self.user_collateral_assets.get(&(user, i)) {
                if let Some(amount) = self.user_collateral.get(&(user, asset)) {
                    if amount > U256::zero() {
                        let config = self.collateral_configs.get(&asset)
                            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral);
                        
                        let value = oracle.get_asset_value(asset, amount);
                        let weighted_value = (value * config.liquidation_threshold) / scale;
                        total_value = total_value + weighted_value;
                    }
                }
            }
        }
        
        total_value
    }
    
    /// Get maximum borrow amount for user based on LTV
    pub fn get_max_borrow_amount(&self, user: Address) -> U256 {
        let oracle_address = self.price_oracle.get_or_revert_with(LendingError::OracleNotInitialized);
        let oracle = PriceOracleContractRef::new(self.env(), oracle_address);
        let scale = self.scale.get_or_default();
        
        let count = self.user_collateral_count.get(&user).unwrap_or(0);
        let mut max_borrow = U256::zero();
        
        for i in 0..count {
            if let Some(asset) = self.user_collateral_assets.get(&(user, i)) {
                if let Some(amount) = self.user_collateral.get(&(user, asset)) {
                    if amount > U256::zero() {
                        let config = self.collateral_configs.get(&asset)
                            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral);
                        
                        let value = oracle.get_asset_value(asset, amount);
                        let borrow_power = (value * config.ltv) / scale;
                        max_borrow = max_borrow + borrow_power;
                    }
                }
            }
        }
        
        max_borrow
    }
    
    /// Get user's collateral balance for an asset
    pub fn get_user_collateral(&self, user: Address, asset: Address) -> U256 {
        self.user_collateral.get(&(user, asset)).unwrap_or(U256::zero())
    }
    
    /// Get collateral configuration
    pub fn get_collateral_config(&self, asset: Address) -> CollateralConfig {
        self.collateral_configs.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::UnsupportedCollateral)
    }
    
    /// Check if user can be liquidated
    pub fn can_liquidate(&self, user: Address, debt: U256) -> bool {
        if debt == U256::zero() {
            return false;
        }
        
        let health_factor = self.calculate_health_factor(user, debt);
        let min_health = self.min_health_factor.get_or_default();
        
        health_factor < min_health
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        if caller != admin {
            self.env().revert(LendingError::Unauthorized);
        }
    }
}
