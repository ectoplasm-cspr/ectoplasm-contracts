//! TokenFactory - Deploys new LaunchToken + BondingCurve pairs
//!
//! This factory creates new token launches by deploying:
//! 1. A LaunchToken contract
//! 2. A BondingCurve contract linked to that token

use odra::prelude::*;
use odra::casper_types::U512;
use super::errors::LaunchpadError;

/// LaunchInfo struct for storing launch data
/// Note: We use separate mappings due to CLTyped constraints
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchInfo {
    pub token: Address,
    pub curve: Address,
    pub creator: Address,
    pub name: String,
    pub symbol: String,
    pub curve_type: u8,
    pub status: u8,
    pub created_at: u64,
}

/// TokenFactory contract for creating new launches
#[odra::module]
pub struct TokenFactory {
    /// Total number of launches created
    launch_count: Var<u64>,
    /// Mapping: launch_id -> token_address
    launch_tokens: Mapping<u64, Address>,
    /// Mapping: launch_id -> curve_address
    launch_curves: Mapping<u64, Address>,
    /// Mapping: launch_id -> creator_address
    launch_creators: Mapping<u64, Address>,
    /// Mapping: launch_id -> curve_type
    launch_curve_types: Mapping<u64, u8>,
    /// Mapping: launch_id -> status
    launch_statuses: Mapping<u64, u8>,
    /// Mapping: launch_id -> created_at
    launch_created_at: Mapping<u64, u64>,
    /// DEX Router address for graduation
    dex_router: Var<Address>,
    /// DEX Factory address for pair creation
    dex_factory: Var<Address>,
    /// Admin address (can update settings)
    admin: Var<Address>,
    /// Default graduation threshold (in motes) - 50,000 CSPR
    default_graduation_threshold: Var<U512>,
    /// Default creator fee (basis points) - 100 = 1%
    default_creator_fee_bps: Var<u64>,
    /// Default deadline (days) - 30 days
    default_deadline_days: Var<u64>,
}

#[odra::module]
impl TokenFactory {
    /// Initialize the TokenFactory
    pub fn init(&mut self, dex_router: Address, dex_factory: Address) {
        let caller = self.env().caller();
        self.admin.set(caller);
        self.dex_router.set(dex_router);
        self.dex_factory.set(dex_factory);
        self.launch_count.set(0);
        
        // Default: 50,000 CSPR threshold
        self.default_graduation_threshold.set(U512::from(50_000u64) * U512::from(1_000_000_000u64));
        // Default: 1% creator fee
        self.default_creator_fee_bps.set(100);
        // Default: 30 days deadline
        self.default_deadline_days.set(30);
    }

    // ============ View Functions ============

    /// Get total launch count
    pub fn launch_count(&self) -> u64 {
        self.launch_count.get_or_default()
    }

    /// Get launch token address by ID
    pub fn get_launch_token(&self, id: u64) -> Option<Address> {
        self.launch_tokens.get(&id)
    }

    /// Get launch curve address by ID
    pub fn get_launch_curve(&self, id: u64) -> Option<Address> {
        self.launch_curves.get(&id)
    }

    /// Get launch creator by ID
    pub fn get_launch_creator(&self, id: u64) -> Option<Address> {
        self.launch_creators.get(&id)
    }

    /// Get launch curve type by ID
    pub fn get_launch_curve_type(&self, id: u64) -> Option<u8> {
        self.launch_curve_types.get(&id)
    }

    /// Get launch status by ID
    pub fn get_launch_status(&self, id: u64) -> Option<u8> {
        self.launch_statuses.get(&id)
    }

    /// Get launch created_at by ID
    pub fn get_launch_created_at(&self, id: u64) -> Option<u64> {
        self.launch_created_at.get(&id)
    }

    /// Get DEX router address
    pub fn dex_router(&self) -> Address {
        self.dex_router.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get DEX factory address
    pub fn dex_factory(&self) -> Address {
        self.dex_factory.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get admin address
    pub fn admin(&self) -> Address {
        self.admin.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get default graduation threshold
    pub fn default_graduation_threshold(&self) -> U512 {
        self.default_graduation_threshold.get_or_default()
    }

    /// Get default creator fee
    pub fn default_creator_fee_bps(&self) -> u64 {
        self.default_creator_fee_bps.get_or_default()
    }

    /// Get default deadline days
    pub fn default_deadline_days(&self) -> u64 {
        self.default_deadline_days.get_or_default()
    }

    // ============ Write Functions ============

    /// Create a new token launch
    /// 
    /// This is a simplified version that stores launch metadata.
    /// In production, this would deploy actual LaunchToken and BondingCurve contracts.
    /// 
    /// # Arguments
    /// * `name` - Token name (stored off-chain or in events for now)
    /// * `symbol` - Token symbol (max 6 chars)
    /// * `curve_type` - 0=Linear, 1=Sigmoid, 2=Steep
    /// * `graduation_threshold` - Optional override for CSPR threshold
    /// * `creator_fee_bps` - Optional override for creator fee
    /// * `deadline_days` - Optional override for deadline
    pub fn create_launch(
        &mut self,
        _name: String,
        symbol: String,
        curve_type: u8,
        graduation_threshold: Option<U512>,
        creator_fee_bps: Option<u64>,
        deadline_days: Option<u64>,
    ) -> u64 {
        // Validate inputs
        if symbol.len() > 6 {
            self.env().revert(LaunchpadError::InvalidCurveType);
        }
        if curve_type > 2 {
            self.env().revert(LaunchpadError::InvalidCurveType);
        }

        let caller = self.env().caller();
        let current_count = self.launch_count.get_or_default();
        let new_id = current_count;

        // Use defaults if not provided
        let _threshold = graduation_threshold.unwrap_or_else(|| {
            self.default_graduation_threshold.get_or_default()
        });
        let _fee_bps = creator_fee_bps.unwrap_or_else(|| {
            self.default_creator_fee_bps.get_or_default()
        });
        let _days = deadline_days.unwrap_or_else(|| {
            self.default_deadline_days.get_or_default()
        });

        // Calculate deadline timestamp
        let current_time = self.env().get_block_time();

        // NOTE: In production, we would deploy actual contracts here:
        // 1. Deploy LaunchToken with (name, symbol, curve_address, caller)
        // 2. Deploy BondingCurve with (token, curve_type, threshold, deadline, caller, fee_bps, router, factory)
        // 
        // For now, we create placeholder addresses (self address as placeholder)
        // The actual deployment would use Odra's factory pattern similar to PairFactory
        
        let placeholder_token = self.env().self_address();
        let placeholder_curve = self.env().self_address();

        // Store launch data in separate mappings
        self.launch_tokens.set(&new_id, placeholder_token);
        self.launch_curves.set(&new_id, placeholder_curve);
        self.launch_creators.set(&new_id, caller);
        self.launch_curve_types.set(&new_id, curve_type);
        self.launch_statuses.set(&new_id, 0); // 0 = Active
        self.launch_created_at.set(&new_id, current_time);
        self.launch_count.set(current_count + 1);

        new_id
    }

    // ============ Admin Functions ============

    /// Update default graduation threshold
    pub fn set_default_graduation_threshold(&mut self, threshold: U512) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LaunchpadError::Unauthorized);
        if caller != admin {
            self.env().revert(LaunchpadError::Unauthorized);
        }
        self.default_graduation_threshold.set(threshold);
    }

    /// Update default creator fee
    pub fn set_default_creator_fee_bps(&mut self, fee_bps: u64) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LaunchpadError::Unauthorized);
        if caller != admin {
            self.env().revert(LaunchpadError::Unauthorized);
        }
        self.default_creator_fee_bps.set(fee_bps);
    }

    /// Update default deadline days
    pub fn set_default_deadline_days(&mut self, days: u64) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LaunchpadError::Unauthorized);
        if caller != admin {
            self.env().revert(LaunchpadError::Unauthorized);
        }
        self.default_deadline_days.set(days);
    }

    /// Transfer admin role
    pub fn transfer_admin(&mut self, new_admin: Address) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LaunchpadError::Unauthorized);
        if caller != admin {
            self.env().revert(LaunchpadError::Unauthorized);
        }
        self.admin.set(new_admin);
    }
}

/// External interface for TokenFactory
#[odra::external_contract]
pub trait TokenFactoryContract {
    fn launch_count(&self) -> u64;
    fn get_launch_token(&self, id: u64) -> Option<Address>;
    fn get_launch_curve(&self, id: u64) -> Option<Address>;
    fn get_launch_creator(&self, id: u64) -> Option<Address>;
    fn get_launch_curve_type(&self, id: u64) -> Option<u8>;
    fn get_launch_status(&self, id: u64) -> Option<u8>;
    fn get_launch_created_at(&self, id: u64) -> Option<u64>;
    fn dex_router(&self) -> Address;
    fn dex_factory(&self) -> Address;
    fn admin(&self) -> Address;
    fn default_graduation_threshold(&self) -> U512;
    fn default_creator_fee_bps(&self) -> u64;
    fn default_deadline_days(&self) -> u64;
    fn create_launch(
        &mut self,
        name: String,
        symbol: String,
        curve_type: u8,
        graduation_threshold: Option<U512>,
        creator_fee_bps: Option<u64>,
        deadline_days: Option<u64>,
    ) -> u64;
    fn set_default_graduation_threshold(&mut self, threshold: U512);
    fn set_default_creator_fee_bps(&mut self, fee_bps: u64);
    fn set_default_deadline_days(&mut self, days: u64);
    fn transfer_admin(&mut self, new_admin: Address);
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    fn setup() -> (HostEnv, TokenFactoryHostRef) {
        let env = odra_test::env();
        let dex_router = env.get_account(1);
        let dex_factory = env.get_account(2);
        
        let init_args = TokenFactoryInitArgs {
            dex_router,
            dex_factory,
        };
        let factory = TokenFactory::deploy(&env, init_args);
        (env, factory)
    }

    #[test]
    fn test_init() {
        let (env, factory) = setup();
        let admin = env.get_account(0);
        let dex_router = env.get_account(1);
        let dex_factory = env.get_account(2);
        
        assert_eq!(factory.admin(), admin);
        assert_eq!(factory.dex_router(), dex_router);
        assert_eq!(factory.dex_factory(), dex_factory);
        assert_eq!(factory.launch_count(), 0);
    }

    #[test]
    fn test_create_launch() {
        let (env, mut factory) = setup();
        let creator = env.get_account(0);
        
        env.set_caller(creator);
        let launch_id = factory.create_launch(
            String::from("Test Token"),
            String::from("TEST"),
            0, // Linear curve
            None,
            None,
            None,
        );
        
        assert_eq!(launch_id, 0);
        assert_eq!(factory.launch_count(), 1);
        
        let launch_creator = factory.get_launch_creator(0);
        assert!(launch_creator.is_some());
        assert_eq!(launch_creator.unwrap(), creator);
    }

    #[test]
    fn test_admin_functions() {
        let (env, mut factory) = setup();
        let admin = env.get_account(0);
        
        env.set_caller(admin);
        
        // Update threshold
        let new_threshold = U512::from(100_000_000_000_000u64); // 100k CSPR
        factory.set_default_graduation_threshold(new_threshold);
        assert_eq!(factory.default_graduation_threshold(), new_threshold);
        
        // Update fee
        factory.set_default_creator_fee_bps(200); // 2%
        assert_eq!(factory.default_creator_fee_bps(), 200);
        
        // Update deadline
        factory.set_default_deadline_days(60);
        assert_eq!(factory.default_deadline_days(), 60);
    }

    #[test]
    #[should_panic]
    fn test_non_admin_cannot_update() {
        let (env, mut factory) = setup();
        let non_admin = env.get_account(3);
        
        env.set_caller(non_admin);
        factory.set_default_creator_fee_bps(500); // Should panic
    }
}
