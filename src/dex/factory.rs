//! Factory contract for the DEX
//! 
//! The Factory is responsible for:
//! - Creating new trading pairs
//! - Managing pair registry
//! - Setting protocol fees
use odra::prelude::*;
use odra::casper_types::U256;
use odra::{Address, Mapping, Var};
use crate::errors::DexError;
use crate::events::PairCreated;

/// Factory contract for creating and managing pairs
#[odra::module]
pub struct Factory {
    /// Fee recipient address
    fee_to: Var<Option<Address>>,
    /// Fee setter address (admin)
    fee_to_setter: Var<Address>,
    /// Mapping from token pair to pair address
    /// Key is (token0, token1) where token0 < token1
    pairs: Mapping<(Address, Address), Address>,
    /// List of all pairs (stored as index -> address)
    all_pairs: Mapping<u32, Address>,
    /// Total number of pairs
    all_pairs_length: Var<u32>,
}

#[odra::module]
impl Factory {
    /// Initialize the factory with the fee setter address
    pub fn init(&mut self, fee_to_setter: Address) {
        self.fee_to_setter.set(fee_to_setter);
        self.fee_to.set(None);
        self.all_pairs_length.set(0);
    }

    /// Get the fee recipient address
    pub fn fee_to(&self) -> Option<Address> {
        self.fee_to.get_or_default()
    }

    /// Get the fee setter address
    pub fn fee_to_setter(&self) -> Address {
        self.fee_to_setter.get_or_revert()
    }

    /// Get the pair address for two tokens
    pub fn get_pair(&self, token_a: Address, token_b: Address) -> Option<Address> {
        let (token0, token1) = self.sort_tokens(token_a, token_b);
        self.pairs.get(&(token0, token1))
    }

    /// Get pair by index
    pub fn all_pairs_at(&self, index: u32) -> Option<Address> {
        self.all_pairs.get(&index)
    }

    /// Get total number of pairs
    pub fn all_pairs_length(&self) -> u32 {
        self.all_pairs_length.get_or_default()
    }

    /// Create a new pair for two tokens
    /// Returns the address of the created pair
    pub fn create_pair(
        &mut self,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, DexError> {
        // Validate tokens
        if token_a == token_b {
            return Err(DexError::IdenticalAddresses);
        }

        // Sort tokens
        let (token0, token1) = self.sort_tokens(token_a, token_b);

        // Check if pair already exists
        if self.pairs.get(&(token0, token1)).is_some() {
            return Err(DexError::PairExists);
        }

        // In a real implementation, we would deploy a new Pair contract here
        // For now, we'll create a deterministic address based on the tokens
        // This is a placeholder - in production, you'd use Odra's contract deployment
        let pair_address = self.compute_pair_address(token0, token1);

        // Store the pair
        self.pairs.set(&(token0, token1), pair_address);
        
        // Also store reverse mapping for convenience
        // (Not strictly necessary but useful for lookups)
        
        // Add to all pairs list
        let pair_index = self.all_pairs_length.get_or_default();
        self.all_pairs.set(&pair_index, pair_address);
        self.all_pairs_length.set(pair_index + 1);

        // Emit event
        self.env().emit_event(PairCreated {
            token0,
            token1,
            pair: pair_address,
            pair_count: pair_index + 1,
        });

        Ok(pair_address)
    }

    /// Set the fee recipient address
    /// Only callable by fee_to_setter
    pub fn set_fee_to(&mut self, fee_to: Address) -> Result<(), DexError> {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            return Err(DexError::Unauthorized);
        }
        self.fee_to.set(Some(fee_to));
        Ok(())
    }

    /// Remove the fee recipient (disable fees)
    /// Only callable by fee_to_setter
    pub fn remove_fee_to(&mut self) -> Result<(), DexError> {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            return Err(DexError::Unauthorized);
        }
        self.fee_to.set(None);
        Ok(())
    }

    /// Set a new fee setter address
    /// Only callable by current fee_to_setter
    pub fn set_fee_to_setter(&mut self, new_fee_to_setter: Address) -> Result<(), DexError> {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            return Err(DexError::Unauthorized);
        }
        self.fee_to_setter.set(new_fee_to_setter);
        Ok(())
    }

    /// Check if a pair exists
    pub fn pair_exists(&self, token_a: Address, token_b: Address) -> bool {
        self.get_pair(token_a, token_b).is_some()
    }

    // ============ Internal Functions ============

    /// Sort two token addresses (smaller address first)
    fn sort_tokens(&self, token_a: Address, token_b: Address) -> (Address, Address) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }

    /// Compute a deterministic pair address
    /// In production, this would be the actual deployed contract address
    fn compute_pair_address(&self, token0: Address, token1: Address) -> Address {
        // This is a simplified version - in production you'd use CREATE2-style
        // deterministic deployment or store the actual deployed address
        // For now, we use the factory address as a placeholder
        // The actual pair deployment would happen in create_pair
        self.env().self_address()
    }
}

/// External interface for the Factory contract
#[odra::external_contract]
pub trait FactoryContract {
    fn fee_to(&self) -> Option<Address>;
    fn fee_to_setter(&self) -> Address;
    fn get_pair(&self, token_a: Address, token_b: Address) -> Option<Address>;
    fn all_pairs_at(&self, index: u32) -> Option<Address>;
    fn all_pairs_length(&self) -> u32;
    fn create_pair(&mut self, token_a: Address, token_b: Address) -> Result<Address, DexError>;
    fn set_fee_to(&mut self, fee_to: Address) -> Result<(), DexError>;
    fn set_fee_to_setter(&mut self, new_fee_to_setter: Address) -> Result<(), DexError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    fn setup() -> (HostEnv, FactoryHostRef) {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let init_args = FactoryInitArgs {
            fee_to_setter: admin,
        };
        let factory = Factory::deploy(&env, init_args);
        (env, factory)
    }

    #[test]
    fn test_factory_init() {
        let (env, factory) = setup();
        let admin = env.get_account(0);
        
        assert_eq!(factory.fee_to_setter(), admin);
        assert_eq!(factory.fee_to(), None);
        assert_eq!(factory.all_pairs_length(), 0);
    }

    #[test]
    fn test_create_pair() {
        let (env, mut factory) = setup();
        let token_a = env.get_account(1);
        let token_b = env.get_account(2);

        let result = factory.create_pair(token_a, token_b);
        assert!(result.is_ok());
        
        assert_eq!(factory.all_pairs_length(), 1);
        assert!(factory.pair_exists(token_a, token_b));
        assert!(factory.pair_exists(token_b, token_a)); // Should work both ways
    }

    #[test]
    fn test_create_pair_identical_addresses() {
        let (env, mut factory) = setup();
        let token = env.get_account(1);

        let result = factory.create_pair(token, token);
        assert_eq!(result, Err(DexError::IdenticalAddresses));
    }

    #[test]
    fn test_create_pair_already_exists() {
        let (env, mut factory) = setup();
        let token_a = env.get_account(1);
        let token_b = env.get_account(2);

        factory.create_pair(token_a, token_b).unwrap();
        let result = factory.create_pair(token_a, token_b);
        assert_eq!(result, Err(DexError::PairExists));
    }

    #[test]
    fn test_set_fee_to() {
        let (env, mut factory) = setup();
        let admin = env.get_account(0);
        let fee_recipient = env.get_account(1);

        env.set_caller(admin);
        let result = factory.set_fee_to(fee_recipient);
        assert!(result.is_ok());
        assert_eq!(factory.fee_to(), Some(fee_recipient));
    }

    #[test]
    fn test_set_fee_to_unauthorized() {
        let (env, mut factory) = setup();
        let non_admin = env.get_account(1);
        let fee_recipient = env.get_account(2);

        env.set_caller(non_admin);
        let result = factory.set_fee_to(fee_recipient);
        assert_eq!(result, Err(DexError::Unauthorized));
    }
}