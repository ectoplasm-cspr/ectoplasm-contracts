//! Factory contract for the DEX
//!
//! The Factory is responsible for:
//! - Creating new trading pairs
//! - Managing pair registry
//! - Setting protocol fees
use odra::prelude::*;
use odra::ContractRef;
use crate::errors::DexError;
use crate::events::PairCreated;
use super::pair::PairFactoryContractRef;

/// Factory contract for creating and managing pairs
#[odra::module]
pub struct Factory {
    /// Fee recipient address
    fee_to: Var<Option<Address>>,
    /// Fee setter address (admin)
    fee_to_setter: Var<Address>,
    /// Address of the Pair Factory contract
    pair_factory: Var<Address>,
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
    /// Initialize the factory with the fee setter address and pair factory address
    pub fn init(&mut self, fee_to_setter: Address, pair_factory: Address) {
        self.fee_to_setter.set(fee_to_setter);
        self.pair_factory.set(pair_factory);
        self.fee_to.set(None);
        self.all_pairs_length.set(0);
    }

    /// Get the fee recipient address
    pub fn fee_to(&self) -> Option<Address> {
        self.fee_to.get_or_default()
    }

    /// Get the fee setter address
    pub fn fee_to_setter(&self) -> Address {
        self.fee_to_setter.get_or_revert_with(DexError::Unauthorized)
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
    ) -> Address {
        // Validate tokens
        if token_a == token_b {
            self.env().revert(DexError::IdenticalAddresses);
        }

        // Sort tokens
        let (token0, token1) = self.sort_tokens(token_a, token_b);

        // Check if pair already exists
        if self.pairs.get(&(token0, token1)).is_some() {
            self.env().revert(DexError::PairExists);
        }

        // Create the new Pair contract using the factory
        let pair_factory_addr = self.pair_factory.get_or_revert_with(DexError::ZeroAddress);
        let mut pair_factory = PairFactoryContractRef::new(self.env(), pair_factory_addr);
        
        let (pair_address, _) = pair_factory.new_contract(
            String::from("Pair"),
            token0,
            token1,
            self.env().self_address()
        );

        // Store the pair
        self.pairs.set(&(token0, token1), pair_address);
        
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

        pair_address
    }

    /// Set the fee recipient address
    /// Only callable by fee_to_setter
    pub fn set_fee_to(&mut self, fee_to: Address) {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            self.env().revert(DexError::Unauthorized);
        }
        self.fee_to.set(Some(fee_to));
    }

    /// Remove the fee recipient (disable fees)
    /// Only callable by fee_to_setter
    pub fn remove_fee_to(&mut self) {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            self.env().revert(DexError::Unauthorized);
        }
        self.fee_to.set(None);
    }

    /// Set a new fee setter address
    /// Only callable by current fee_to_setter
    pub fn set_fee_to_setter(&mut self, new_fee_to_setter: Address) {
        let caller = self.env().caller();
        if caller != self.fee_to_setter() {
            self.env().revert(DexError::Unauthorized);
        }
        self.fee_to_setter.set(new_fee_to_setter);
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
}



/// External interface for the Factory contract
#[odra::external_contract]
pub trait FactoryContract {
    fn fee_to(&self) -> Option<Address>;
    fn fee_to_setter(&self) -> Address;
    fn get_pair(&self, token_a: Address, token_b: Address) -> Option<Address>;
    fn all_pairs_at(&self, index: u32) -> Option<Address>;
    fn all_pairs_length(&self) -> u32;
    fn create_pair(&mut self, token_a: Address, token_b: Address) -> Address;
    fn set_fee_to(&mut self, fee_to: Address);
    fn set_fee_to_setter(&mut self, new_fee_to_setter: Address);
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    fn setup() -> (HostEnv, FactoryHostRef) {
        let env = odra_test::env();
        let admin = env.get_account(0);
        
        let pair_factory = super::super::pair::PairFactory::deploy(&env, odra::host::NoArgs);
        
        let init_args = FactoryInitArgs {
            fee_to_setter: admin,
            pair_factory: pair_factory.address().clone(),
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
    #[ignore = "Factory pattern not supported in Odra MockVM"]
    fn test_create_pair() {
        let (env, mut factory) = setup();
        let token_a = env.get_account(1);
        let token_b = env.get_account(2);

        let _pair = factory.create_pair(token_a, token_b);
        
        assert_eq!(factory.all_pairs_length(), 1);
        assert!(factory.pair_exists(token_a, token_b));
        assert!(factory.pair_exists(token_b, token_a)); // Should work both ways
    }

    #[test]
    fn test_set_fee_to() {
        let (env, mut factory) = setup();
        let admin = env.get_account(0);
        let fee_recipient = env.get_account(1);

        env.set_caller(admin);
        factory.set_fee_to(fee_recipient);
        assert_eq!(factory.fee_to(), Some(fee_recipient));
    }
}