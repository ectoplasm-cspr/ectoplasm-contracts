//! LaunchToken - CEP-18 compatible token with restricted minting
//! 
//! This token is used for launchpad tokens. Only the associated
//! BondingCurve contract can mint/burn tokens.

use odra::prelude::*;
use odra::casper_types::U256;
use crate::events::{Transfer, Approval};
use super::errors::LaunchpadError;

/// LaunchToken module implementing CEP-18 with restricted minting
#[odra::module]
pub struct LaunchToken {
    /// Token name
    name: Var<String>,
    /// Token symbol
    symbol: Var<String>,
    /// Token decimals
    decimals: Var<u8>,
    /// Total supply of tokens
    total_supply: Var<U256>,
    /// Balance mapping: owner -> balance
    balances: Mapping<Address, U256>,
    /// Allowance mapping: owner -> spender -> amount
    allowances: Mapping<(Address, Address), U256>,
    /// Minter address (the BondingCurve contract)
    minter: Var<Address>,
    /// Creator address
    creator: Var<Address>,
}

#[odra::module]
impl LaunchToken {
    /// Initialize the LaunchToken
    pub fn init(&mut self, name: String, symbol: String, minter: Address, creator: Address) {
        self.name.set(name);
        self.symbol.set(symbol);
        self.decimals.set(18);
        self.total_supply.set(U256::zero());
        self.minter.set(minter);
        self.creator.set(creator);
    }

    // ============ View Functions ============

    /// Get the token name
    pub fn name(&self) -> String {
        self.name.get_or_default()
    }

    /// Get the token symbol
    pub fn symbol(&self) -> String {
        self.symbol.get_or_default()
    }

    /// Get the token decimals
    pub fn decimals(&self) -> u8 {
        self.decimals.get_or_default()
    }

    /// Get the total supply
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get_or_default()
    }

    /// Get the balance of an address
    pub fn balance_of(&self, owner: Address) -> U256 {
        self.balances.get(&owner).unwrap_or_default()
    }

    /// Get the allowance for a spender
    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.get(&(owner, spender)).unwrap_or_default()
    }

    /// Get the minter address
    pub fn minter(&self) -> Address {
        self.minter.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get the creator address
    pub fn creator(&self) -> Address {
        self.creator.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    // ============ Write Functions ============

    /// Transfer tokens to another address
    pub fn transfer(&mut self, to: Address, amount: U256) -> bool {
        let caller = self.env().caller();
        self.transfer_internal(caller, to, amount);
        true
    }

    /// Approve a spender to spend tokens
    pub fn approve(&mut self, spender: Address, amount: U256) -> bool {
        let caller = self.env().caller();
        self.approve_internal(caller, spender, amount);
        true
    }

    /// Transfer tokens from one address to another (requires approval)
    pub fn transfer_from(&mut self, from: Address, to: Address, amount: U256) -> bool {
        let caller = self.env().caller();
        let current_allowance = self.allowance(from, caller);
        
        if current_allowance < amount {
            self.env().revert(LaunchpadError::InsufficientBalance);
        }
        
        self.approve_internal(from, caller, current_allowance - amount);
        self.transfer_internal(from, to, amount);
        true
    }

    /// Mint new tokens - ONLY callable by the minter (BondingCurve)
    pub fn mint(&mut self, to: Address, amount: U256) {
        let caller = self.env().caller();
        let minter = self.minter.get_or_revert_with(LaunchpadError::Unauthorized);
        
        if caller != minter {
            self.env().revert(LaunchpadError::Unauthorized);
        }

        let current_supply = self.total_supply();
        let new_supply = current_supply + amount;
        self.total_supply.set(new_supply);

        let current_balance = self.balance_of(to);
        self.balances.set(&to, current_balance + amount);

        self.env().emit_event(Transfer {
            from: Address::from(self.env().self_address()),
            to,
            value: amount,
        });
    }

    /// Burn tokens - ONLY callable by the minter (BondingCurve)
    pub fn burn(&mut self, from: Address, amount: U256) {
        let caller = self.env().caller();
        let minter = self.minter.get_or_revert_with(LaunchpadError::Unauthorized);
        
        if caller != minter {
            self.env().revert(LaunchpadError::Unauthorized);
        }

        let current_balance = self.balance_of(from);
        if current_balance < amount {
            self.env().revert(LaunchpadError::InsufficientBalance);
        }

        self.balances.set(&from, current_balance - amount);

        let current_supply = self.total_supply();
        self.total_supply.set(current_supply - amount);

        self.env().emit_event(Transfer {
            from,
            to: Address::from(self.env().self_address()),
            value: amount,
        });
    }

    // ============ Internal Functions ============

    /// Internal transfer function
    fn transfer_internal(&mut self, from: Address, to: Address, amount: U256) {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            self.env().revert(LaunchpadError::InsufficientBalance);
        }

        self.balances.set(&from, from_balance - amount);
        let to_balance = self.balance_of(to);
        self.balances.set(&to, to_balance + amount);

        self.env().emit_event(Transfer {
            from,
            to,
            value: amount,
        });
    }

    /// Internal approve function
    fn approve_internal(&mut self, owner: Address, spender: Address, amount: U256) {
        self.allowances.set(&(owner, spender), amount);

        self.env().emit_event(Approval {
            owner,
            spender,
            value: amount,
        });
    }
}

/// External interface for LaunchToken
#[odra::external_contract]
pub trait LaunchTokenContract {
    fn name(&self) -> String;
    fn symbol(&self) -> String;
    fn decimals(&self) -> u8;
    fn total_supply(&self) -> U256;
    fn balance_of(&self, owner: Address) -> U256;
    fn allowance(&self, owner: Address, spender: Address) -> U256;
    fn transfer(&mut self, to: Address, amount: U256) -> bool;
    fn approve(&mut self, spender: Address, amount: U256) -> bool;
    fn transfer_from(&mut self, from: Address, to: Address, amount: U256) -> bool;
    fn mint(&mut self, to: Address, amount: U256);
    fn burn(&mut self, from: Address, amount: U256);
    fn minter(&self) -> Address;
    fn creator(&self) -> Address;
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    fn setup() -> (HostEnv, LaunchTokenHostRef) {
        let env = odra_test::env();
        let minter = env.get_account(1);
        let creator = env.get_account(2);
        
        let init_args = LaunchTokenInitArgs {
            name: String::from("Test Launch Token"),
            symbol: String::from("TLT"),
            minter,
            creator,
        };
        let token = LaunchToken::deploy(&env, init_args);
        (env, token)
    }

    #[test]
    fn test_init() {
        let (env, token) = setup();
        let minter = env.get_account(1);
        let creator = env.get_account(2);
        
        assert_eq!(token.name(), "Test Launch Token");
        assert_eq!(token.symbol(), "TLT");
        assert_eq!(token.decimals(), 18);
        assert_eq!(token.total_supply(), U256::zero());
        assert_eq!(token.minter(), minter);
        assert_eq!(token.creator(), creator);
    }

    #[test]
    fn test_mint_by_minter() {
        let (env, mut token) = setup();
        let minter = env.get_account(1);
        let user = env.get_account(3);
        let amount = U256::from(1000);

        env.set_caller(minter);
        token.mint(user, amount);
        
        assert_eq!(token.balance_of(user), amount);
        assert_eq!(token.total_supply(), amount);
    }

    #[test]
    #[should_panic]
    fn test_mint_by_non_minter_fails() {
        let (env, mut token) = setup();
        let non_minter = env.get_account(3);
        let user = env.get_account(4);
        let amount = U256::from(1000);

        env.set_caller(non_minter);
        token.mint(user, amount); // Should panic
    }

    #[test]
    fn test_burn_by_minter() {
        let (env, mut token) = setup();
        let minter = env.get_account(1);
        let user = env.get_account(3);
        let amount = U256::from(1000);

        env.set_caller(minter);
        token.mint(user, amount);
        token.burn(user, amount);
        
        assert_eq!(token.balance_of(user), U256::zero());
        assert_eq!(token.total_supply(), U256::zero());
    }

    #[test]
    fn test_transfer() {
        let (env, mut token) = setup();
        let minter = env.get_account(1);
        let user1 = env.get_account(3);
        let user2 = env.get_account(4);
        let amount = U256::from(1000);

        env.set_caller(minter);
        token.mint(user1, amount);
        
        env.set_caller(user1);
        token.transfer(user2, U256::from(500));
        
        assert_eq!(token.balance_of(user1), U256::from(500));
        assert_eq!(token.balance_of(user2), U256::from(500));
    }
}
