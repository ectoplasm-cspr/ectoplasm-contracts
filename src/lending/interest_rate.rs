//! Interest Rate Strategy - Variable rate model based on utilization
//! 
//! Implements a two-slope interest rate model similar to Aave:
//! - Base rate: Minimum interest rate
//! - Optimal utilization: Target utilization rate (e.g., 80%)
//! - Slope 1: Rate increase before optimal utilization
//! - Slope 2: Steep rate increase after optimal utilization

use odra::prelude::*;
use odra::casper_types::U256;
use super::errors::LendingError;

/// Interest rate strategy parameters
#[odra::odra_type]
pub struct InterestRateParams {
    /// Base interest rate (annual, scaled by 1e18)
    /// Example: 2% = 0.02 * 1e18 = 20000000000000000
    pub base_rate: U256,
    
    /// Optimal utilization rate (scaled by 1e18)
    /// Example: 80% = 0.80 * 1e18 = 800000000000000000
    pub optimal_utilization: U256,
    
    /// Slope 1: Rate increase per utilization before optimal (scaled by 1e18)
    /// Example: 4% = 0.04 * 1e18 = 40000000000000000
    pub slope1: U256,
    
    /// Slope 2: Rate increase per utilization after optimal (scaled by 1e18)
    /// Example: 75% = 0.75 * 1e18 = 750000000000000000
    pub slope2: U256,
}

/// Interest Rate Strategy contract
#[odra::module]
pub struct InterestRateStrategy {
    /// Interest rate parameters
    params: Var<InterestRateParams>,
    
    /// Scale factor for calculations (1e18)
    scale: Var<U256>,
}

#[odra::module]
impl InterestRateStrategy {
    /// Initialize the interest rate strategy
    /// 
    /// # Arguments
    /// * `base_rate` - Base annual rate (scaled by 1e18)
    /// * `optimal_utilization` - Target utilization (scaled by 1e18)
    /// * `slope1` - Rate increase before optimal (scaled by 1e18)
    /// * `slope2` - Rate increase after optimal (scaled by 1e18)
    pub fn init(
        &mut self,
        base_rate: U256,
        optimal_utilization: U256,
        slope1: U256,
        slope2: U256,
    ) {
        let params = InterestRateParams {
            base_rate,
            optimal_utilization,
            slope1,
            slope2,
        };
        
        self.params.set(params);
        self.scale.set(U256::from(1_000_000_000_000_000_000u128)); // 1e18
    }
    
    /// Calculate borrow rate based on utilization
    /// 
    /// Formula:
    /// - If utilization <= optimal:
    ///   rate = base_rate + (utilization / optimal) * slope1
    /// - If utilization > optimal:
    ///   rate = base_rate + slope1 + ((utilization - optimal) / (1 - optimal)) * slope2
    /// 
    /// # Arguments
    /// * `total_borrows` - Total amount borrowed
    /// * `total_liquidity` - Total liquidity available (deposits - borrows)
    /// 
    /// # Returns
    /// Annual borrow rate (scaled by 1e18)
    pub fn calculate_borrow_rate(
        &self,
        total_borrows: U256,
        total_liquidity: U256,
    ) -> U256 {
        // Calculate utilization rate
        let utilization = self.calculate_utilization_rate(total_borrows, total_liquidity);
        
        if utilization == U256::zero() {
            return self.params.get_or_revert_with(LendingError::InvalidConfiguration).base_rate;
        }
        
        let params = self.params.get_or_revert_with(LendingError::InvalidConfiguration);
        let scale = self.scale.get_or_default();
        
        if utilization <= params.optimal_utilization {
            // Before optimal: base_rate + (utilization / optimal) * slope1
            let rate_increase = (utilization * params.slope1) / params.optimal_utilization;
            params.base_rate + rate_increase
        } else {
            // After optimal: base_rate + slope1 + excess_utilization_ratio * slope2
            let excess_utilization = utilization - params.optimal_utilization;
            let excess_utilization_ratio = (excess_utilization * scale) / (scale - params.optimal_utilization);
            let excess_rate = (excess_utilization_ratio * params.slope2) / scale;
            
            params.base_rate + params.slope1 + excess_rate
        }
    }
    
    /// Calculate supply rate (deposit APY) based on borrow rate
    /// 
    /// Formula: supply_rate = borrow_rate * utilization * (1 - reserve_factor)
    /// 
    /// # Arguments
    /// * `borrow_rate` - Current borrow rate
    /// * `total_borrows` - Total amount borrowed
    /// * `total_liquidity` - Total liquidity available
    /// * `reserve_factor` - Percentage of interest going to reserves (scaled by 1e18)
    /// 
    /// # Returns
    /// Annual supply rate (scaled by 1e18)
    pub fn calculate_supply_rate(
        &self,
        borrow_rate: U256,
        total_borrows: U256,
        total_liquidity: U256,
        reserve_factor: U256,
    ) -> U256 {
        let utilization = self.calculate_utilization_rate(total_borrows, total_liquidity);
        
        if utilization == U256::zero() {
            return U256::zero();
        }
        
        let scale = self.scale.get_or_default();
        
        // supply_rate = borrow_rate * utilization * (1 - reserve_factor)
        let rate_to_pool = (borrow_rate * (scale - reserve_factor)) / scale;
        (rate_to_pool * utilization) / scale
    }
    
    /// Calculate utilization rate
    /// 
    /// Formula: utilization = total_borrows / (total_borrows + total_liquidity)
    /// 
    /// # Arguments
    /// * `total_borrows` - Total amount borrowed
    /// * `total_liquidity` - Total liquidity available
    /// 
    /// # Returns
    /// Utilization rate (scaled by 1e18)
    pub fn calculate_utilization_rate(
        &self,
        total_borrows: U256,
        total_liquidity: U256,
    ) -> U256 {
        if total_borrows == U256::zero() {
            return U256::zero();
        }
        
        let total_deposits = total_borrows + total_liquidity;
        if total_deposits == U256::zero() {
            return U256::zero();
        }
        
        let scale = self.scale.get_or_default();
        (total_borrows * scale) / total_deposits
    }
    
    /// Get current interest rate parameters
    pub fn get_params(&self) -> InterestRateParams {
        self.params.get_or_revert_with(LendingError::InvalidConfiguration)
    }
    
    /// Update interest rate parameters (admin only)
    pub fn update_params(
        &mut self,
        base_rate: U256,
        optimal_utilization: U256,
        slope1: U256,
        slope2: U256,
    ) {
        // TODO: Add admin check
        
        let params = InterestRateParams {
            base_rate,
            optimal_utilization,
            slope1,
            slope2,
        };
        
        self.params.set(params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_utilization_calculation() {
        // Test utilization = 50%
        // borrows = 500, liquidity = 500, total = 1000
        // utilization = 500 / 1000 = 0.5 = 50%
    }
    
    #[test]
    fn test_borrow_rate_before_optimal() {
        // Test rate calculation when utilization < optimal
    }
    
    #[test]
    fn test_borrow_rate_after_optimal() {
        // Test rate calculation when utilization > optimal
    }
    
    #[test]
    fn test_supply_rate() {
        // Test supply rate calculation
    }
}
