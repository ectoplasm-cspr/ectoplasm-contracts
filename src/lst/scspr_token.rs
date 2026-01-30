//! sCSPR Token - Liquid Staking Token for Casper Network
//! 
//! This token represents staked CSPR that continues to earn rewards
//! while remaining liquid and composable in DeFi applications.

use odra::prelude::*;
use odra::casper_types::U256;
use crate::events::{Transfer, Approval};
use crate::errors::TokenError;

/// sCSPR Token - Staked CSPR liquid token
/// This token is minted when users stake CSPR and burned when they unstake.
/// The exchange rate between sCSPR and CSPR increases over time as rewards accumulate.
#[odra::module]
pub struct ScsprToken {
    /// Token name
    name: Var<String>,
    /// Token symbol
    symbol: Var<String>,
    /// Token decimals (18 to match CSPR)
    decimals: Var<u8>,
    /// Total supply of sCSPR tokens
    total_supply: Var<U256>,
    /// Balance mapping: owner -> balance
    balances: Mapping<Address, U256>,
    /// Allowance mapping: owner -> spender -> amount
    allowances: Mapping<(Address, Address), U256>,
    /// Staking manager contract address (only this contract can mint/burn)
    staking_manager: Var<Address>,
    /// Contract admin
    admin: Var<Address>,
}

#[odra::module]
impl ScsprToken {
    /// Initialize the sCSPR token
    pub fn init(&mut self, staking_manager: Address) {
        let caller = self.env().caller();
        self.name.set(String::from("Staked CSPR"));
        self.symbol.set(String::from("sCSPR"));
        self.decimals.set(18);
        self.total_supply.set(U256::zero());
        self.staking_manager.set(staking_manager);
        self.admin.set(caller);
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
            self.env().revert(TokenError::InsufficientAllowance);
        }
        
        self.approve_internal(from, caller, current_allowance - amount);
        self.transfer_internal(from, to, amount);
        true
    }

    /// Mint new sCSPR tokens (only callable by staking manager)
    pub fn mint(&mut self, to: Address, amount: U256) {
        self.only_staking_manager();
        
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

    /// Burn sCSPR tokens (only callable by staking manager)
    pub fn burn(&mut self, from: Address, amount: U256) {
        self.only_staking_manager();
        
        let current_balance = self.balance_of(from);
        if current_balance < amount {
            self.env().revert(TokenError::InsufficientBalance);
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

    /// Get the staking manager address
    pub fn get_staking_manager(&self) -> Address {
        self.staking_manager.get_or_revert_with(TokenError::InsufficientAllowance)
    }

    /// Update the staking manager address (admin only)
    pub fn set_staking_manager(&mut self, new_manager: Address) {
        self.only_admin();
        self.staking_manager.set(new_manager);
    }

    /// Get the admin address
    pub fn get_admin(&self) -> Address {
        self.admin.get_or_revert_with(TokenError::InsufficientAllowance)
    }

    /// Transfer admin rights (admin only)
    pub fn transfer_admin(&mut self, new_admin: Address) {
        self.only_admin();
        self.admin.set(new_admin);
    }

    // Internal functions

    fn transfer_internal(&mut self, from: Address, to: Address, amount: U256) {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            self.env().revert(TokenError::InsufficientBalance);
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

    fn approve_internal(&mut self, owner: Address, spender: Address, amount: U256) {
        self.allowances.set(&(owner, spender), amount);
        self.env().emit_event(Approval {
            owner,
            spender,
            value: amount,
        });
    }

    fn only_staking_manager(&self) {
        let caller = self.env().caller();
        let manager = self.staking_manager.get_or_revert_with(TokenError::InsufficientAllowance);
        if caller != manager {
            self.env().revert(TokenError::InsufficientAllowance); // Using existing error for unauthorized
        }
    }

    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(TokenError::InsufficientAllowance);
        if caller != admin {
            self.env().revert(TokenError::InsufficientAllowance); // Using existing error for unauthorized
        }
    }
}
