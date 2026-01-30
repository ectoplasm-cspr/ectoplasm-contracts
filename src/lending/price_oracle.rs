//! Price Oracle - Provides asset prices for collateral valuation
//! 
//! Initially uses DEX prices, can be upgraded to use external oracles

use odra::prelude::*;
use odra::casper_types::U256;
use super::errors::LendingError;

/// Price feed data for an asset
#[odra::odra_type]
pub struct PriceFeed {
    /// Asset address
    pub asset: Address,
    /// Price in ECTO (scaled by 1e18)
    /// Example: 1 sCSPR = 1.1 ECTO means price = 1.1 * 1e18
    pub price: U256,
    /// Timestamp of last update
    pub last_update: u64,
    /// Whether the feed is active
    pub is_active: bool,
}

/// Price Oracle contract
#[odra::module]
pub struct PriceOracle {
    /// Price feeds for each asset
    price_feeds: Mapping<Address, PriceFeed>,
    
    /// Admin address
    admin: Var<Address>,
    
    /// Maximum price staleness (in seconds)
    max_staleness: Var<u64>,
    
    /// Scale factor (1e18)
    scale: Var<U256>,
}

#[odra::module]
impl PriceOracle {
    /// Initialize the price oracle
    pub fn init(&mut self) {
        let caller = self.env().caller();
        self.admin.set(caller);
        self.max_staleness.set(3600); // 1 hour default
        self.scale.set(U256::from(1_000_000_000_000_000_000u128)); // 1e18
    }
    
    /// Set price for an asset (admin only)
    /// 
    /// # Arguments
    /// * `asset` - Asset address
    /// * `price` - Price in ECTO (scaled by 1e18)
    pub fn set_price(&mut self, asset: Address, price: U256) {
        self.only_admin();
        
        if price == U256::zero() {
            self.env().revert(LendingError::InvalidPrice);
        }
        
        let feed = PriceFeed {
            asset,
            price,
            last_update: self.env().get_block_time(),
            is_active: true,
        };
        
        self.price_feeds.set(&asset, feed);
    }
    
    /// Get price for an asset
    /// 
    /// # Arguments
    /// * `asset` - Asset address
    /// 
    /// # Returns
    /// Price in ECTO (scaled by 1e18)
    pub fn get_price(&self, asset: Address) -> U256 {
        let feed = self.price_feeds.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::PriceFeedNotAvailable);
        
        if !feed.is_active {
            self.env().revert(LendingError::PriceFeedNotAvailable);
        }
        
        // Check if price is stale
        let current_time = self.env().get_block_time();
        let max_staleness = self.max_staleness.get_or_default();
        
        if current_time - feed.last_update > max_staleness {
            self.env().revert(LendingError::InvalidPrice);
        }
        
        feed.price
    }
    
    /// Get price with staleness check disabled (for testing)
    pub fn get_price_unchecked(&self, asset: Address) -> U256 {
        let feed = self.price_feeds.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::PriceFeedNotAvailable);
        feed.price
    }
    
    /// Calculate value of an amount in ECTO
    /// 
    /// # Arguments
    /// * `asset` - Asset address
    /// * `amount` - Amount of asset
    /// 
    /// # Returns
    /// Value in ECTO (scaled by 1e18)
    pub fn get_asset_value(&self, asset: Address, amount: U256) -> U256 {
        let price = self.get_price(asset);
        let scale = self.scale.get_or_default();
        
        // value = amount * price / scale
        (amount * price) / scale
    }
    
    /// Calculate amount of asset for a given ECTO value
    /// 
    /// # Arguments
    /// * `asset` - Asset address
    /// * `ecto_value` - Value in ECTO
    /// 
    /// # Returns
    /// Amount of asset
    pub fn get_asset_amount(&self, asset: Address, ecto_value: U256) -> U256 {
        let price = self.get_price(asset);
        let scale = self.scale.get_or_default();
        
        // amount = ecto_value * scale / price
        (ecto_value * scale) / price
    }
    
    /// Disable a price feed (admin only)
    pub fn disable_feed(&mut self, asset: Address) {
        self.only_admin();
        
        let mut feed = self.price_feeds.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::PriceFeedNotAvailable);
        
        feed.is_active = false;
        self.price_feeds.set(&asset, feed);
    }
    
    /// Enable a price feed (admin only)
    pub fn enable_feed(&mut self, asset: Address) {
        self.only_admin();
        
        let mut feed = self.price_feeds.get(&asset)
            .unwrap_or_revert_with(&self.env(), LendingError::PriceFeedNotAvailable);
        
        feed.is_active = true;
        self.price_feeds.set(&asset, feed);
    }
    
    /// Update max staleness period (admin only)
    pub fn set_max_staleness(&mut self, seconds: u64) {
        self.only_admin();
        self.max_staleness.set(seconds);
    }
    
    /// Get admin address
    pub fn get_admin(&self) -> Address {
        self.admin.get_or_revert_with(LendingError::Unauthorized)
    }
    
    /// Check if caller is admin
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        if caller != admin {
            self.env().revert(LendingError::Unauthorized);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_set_and_get_price() {
        // Test setting and getting price
    }
    
    #[test]
    fn test_asset_value_calculation() {
        // Test value calculation
    }
    
    #[test]
    fn test_stale_price_rejection() {
        // Test that stale prices are rejected
    }
}
