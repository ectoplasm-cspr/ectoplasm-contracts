//! CEP-18 compatible token implementation for LP tokens
//! This module provides the LP (Liquidity Provider) token functionality
use odra::prelude::*;
use odra::casper_types::U256;
use odra::{Address, Mapping, Var};
use crate::events::{Transfer, Approval};

/// LP Token module implementing CEP-18 standard
#[odra::module]
pub struct LpToken {
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
}

#[odra::module]
impl LpToken {
    /// Initialize the LP token with name and symbol
    pub fn init(&mut self, name: String, symbol: String) {
        self.name.set(name);
        self.symbol.set(symbol);
        self.decimals.set(18);
        self.total_supply.set(U256::zero());
    }

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
            self.env().revert(odra::ExecutionError::User(1)); // InsufficientAllowance
        }
        
        self.approve_internal(from, caller, current_allowance - amount);
        self.transfer_internal(from, to, amount);
        true
    }

    /// Mint new tokens (internal function)
    pub fn mint(&mut self, to: Address, amount: U256) {
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

    /// Burn tokens (internal function)
    pub fn burn(&mut self, from: Address, amount: U256) {
        let current_balance = self.balance_of(from);
        if current_balance < amount {
            self.env().revert(odra::ExecutionError::User(2)); // InsufficientBalance
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

    /// Internal transfer function
    fn transfer_internal(&mut self, from: Address, to: Address, amount: U256) {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            self.env().revert(odra::ExecutionError::User(2)); // InsufficientBalance
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

/// External token interface for interacting with CEP-18 tokens
#[odra::external_contract]
pub trait Cep18Token {
    /// Get the balance of an address
    fn balance_of(&self, owner: Address) -> U256;
    
    /// Transfer tokens
    fn transfer(&mut self, to: Address, amount: U256) -> bool;
    
    /// Transfer tokens from another address
    fn transfer_from(&mut self, from: Address, to: Address, amount: U256) -> bool;
    
    /// Approve a spender
    fn approve(&mut self, spender: Address, amount: U256) -> bool;
    
    /// Get allowance
    fn allowance(&self, owner: Address, spender: Address) -> U256;
    
    /// Get total supply
    fn total_supply(&self) -> U256;
    
    /// Get token name
    fn name(&self) -> String;
    
    /// Get token symbol
    fn symbol(&self) -> String;
    
    /// Get token decimals
    fn decimals(&self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    fn setup() -> (HostEnv, LpTokenHostRef) {
        let env = odra_test::env();
        let init_args = LpTokenInitArgs {
            name: String::from("LP Token"),
            symbol: String::from("LP"),
        };
        let token = LpToken::deploy(&env, init_args);
        (env, token)
    }

    #[test]
    fn test_init() {
        let (_, token) = setup();
        assert_eq!(token.name(), "LP Token");
        assert_eq!(token.symbol(), "LP");
        assert_eq!(token.decimals(), 18);
        assert_eq!(token.total_supply(), U256::zero());
    }

    #[test]
    fn test_mint_and_burn() {
        let (env, mut token) = setup();
        let user = env.get_account(1);
        let amount = U256::from(1000);

        token.mint(user, amount);
        assert_eq!(token.balance_of(user), amount);
        assert_eq!(token.total_supply(), amount);

        token.burn(user, amount);
        assert_eq!(token.balance_of(user), U256::zero());
        assert_eq!(token.total_supply(), U256::zero());
    }

    #[test]
    fn test_transfer() {
        let (env, mut token) = setup();
        let user1 = env.get_account(0);
        let user2 = env.get_account(1);
        let amount = U256::from(1000);

        token.mint(user1, amount);
        
        env.set_caller(user1);
        token.transfer(user2, U256::from(500));
        
        assert_eq!(token.balance_of(user1), U256::from(500));
        assert_eq!(token.balance_of(user2), U256::from(500));
    }
}