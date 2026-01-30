//! Liquidation Engine - Handles liquidation of undercollateralized positions
//! 
//! Liquidators can repay a portion of a borrower's debt and receive
//! collateral at a discount (liquidation bonus).

use odra::prelude::*;
use odra::casper_types::U256;
use super::errors::LendingError;
use super::events::*;

/// Liquidation parameters
#[odra::odra_type]
pub struct LiquidationParams {
    /// Maximum percentage of debt that can be liquidated at once (scaled by 1e18)
    /// Example: 50% = 0.50 * 1e18
    pub max_liquidation_close_factor: U256,
    /// Minimum health factor to trigger liquidation (scaled by 1e18)
    /// Example: 1.0 = 1e18
    pub liquidation_threshold: U256,
}

/// Liquidation Engine contract
#[odra::module]
pub struct LiquidationEngine {
    /// Liquidation parameters
    params: Var<LiquidationParams>,
    /// Admin address
    admin: Var<Address>,
    /// Scale factor (1e18)
    scale: Var<U256>,
}

#[odra::module]
impl LiquidationEngine {
    /// Initialize the liquidation engine
    pub fn init(&mut self) {
        let caller = self.env().caller();
        self.admin.set(caller);
        self.scale.set(U256::from(1_000_000_000_000_000_000u128)); // 1e18
        
        // Default parameters
        let params = LiquidationParams {
            max_liquidation_close_factor: U256::from(500_000_000_000_000_000u128), // 50%
            liquidation_threshold: U256::from(1_000_000_000_000_000_000u128), // 1.0
        };
        self.params.set(params);
    }
    
    /// Calculate liquidation amounts
    /// 
    /// # Arguments
    /// * `debt_to_cover` - Amount of debt liquidator wants to repay
    /// * `total_debt` - Borrower's total debt
    /// * `collateral_value` - Value of collateral in ECTO
    /// * `liquidation_bonus` - Bonus percentage (scaled by 1e18)
    /// 
    /// # Returns
    /// (actual_debt_to_cover, collateral_to_seize)
    pub fn calculate_liquidation_amounts(
        &self,
        debt_to_cover: U256,
        total_debt: U256,
        collateral_value: U256,
        liquidation_bonus: U256,
    ) -> (U256, U256) {
        let params = self.params.get_or_revert_with(LendingError::InvalidConfiguration);
        let scale = self.scale.get_or_default();
        
        // Calculate maximum debt that can be covered (close factor)
        let max_debt_to_cover = (total_debt * params.max_liquidation_close_factor) / scale;
        
        // Actual debt to cover is minimum of requested and maximum
        let actual_debt = if debt_to_cover > max_debt_to_cover {
            max_debt_to_cover
        } else {
            debt_to_cover
        };
        
        // Calculate collateral to seize with bonus
        // collateral_to_seize = debt_to_cover * (1 + liquidation_bonus)
        let bonus_multiplier = scale + liquidation_bonus;
        let collateral_to_seize = (actual_debt * bonus_multiplier) / scale;
        
        // Check if there's enough collateral
        if collateral_to_seize > collateral_value {
            self.env().revert(LendingError::InsufficientCollateralForLiquidation);
        }
        
        (actual_debt, collateral_to_seize)
    }
    
    /// Check if a position can be liquidated
    /// 
    /// # Arguments
    /// * `health_factor` - User's health factor (scaled by 1e18)
    /// 
    /// # Returns
    /// True if health factor is below threshold
    pub fn can_liquidate(&self, health_factor: U256) -> bool {
        let params = self.params.get_or_revert_with(LendingError::InvalidConfiguration);
        health_factor < params.liquidation_threshold
    }
    
    /// Get liquidation parameters
    pub fn get_params(&self) -> LiquidationParams {
        self.params.get_or_revert_with(LendingError::InvalidConfiguration)
    }
    
    /// Update liquidation parameters (admin only)
    pub fn update_params(
        &mut self,
        max_liquidation_close_factor: U256,
        liquidation_threshold: U256,
    ) {
        self.only_admin();
        
        let scale = self.scale.get_or_default();
        if max_liquidation_close_factor > scale {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        
        let params = LiquidationParams {
            max_liquidation_close_factor,
            liquidation_threshold,
        };
        self.params.set(params);
    }
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        if caller != admin {
            self.env().revert(LendingError::Unauthorized);
        }
    }
}
