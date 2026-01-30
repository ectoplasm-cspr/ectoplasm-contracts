//! aECTO Vault - CEP-4626 compliant interest-bearing ECTO token
//! 
//! Users deposit ECTO and receive aECTO shares that increase in value
//! as interest accrues from borrowers.

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use super::errors::LendingError;
use crate::cep4626::{Cep4626Vault, Deposit as Cep4626Deposit, Withdraw as Cep4626Withdraw};
use crate::token::Cep18TokenContractRef;

/// aECTO Vault - Interest-bearing ECTO token
#[odra::module]
pub struct AectoVault {
    /// Name of the token
    name: Var<String>,
    /// Symbol of the token
    symbol: Var<String>,
    /// Decimals
    decimals: Var<u8>,
    /// Total supply of aECTO shares
    total_supply: Var<U256>,
    /// User balances
    balances: Mapping<Address, U256>,
    /// Allowances
    allowances: Mapping<(Address, Address), U256>,
    
    /// Underlying ECTO token address
    ecto_token: Var<Address>,
    /// Total ECTO deposited (including accrued interest)
    total_assets: Var<U256>,
    
    /// Lending pool address (can deposit/withdraw)
    lending_pool: Var<Address>,
    /// Admin address
    admin: Var<Address>,
    /// Paused state
    paused: Var<bool>,
}

#[odra::module]
impl AectoVault {
    /// Initialize the aECTO vault
    pub fn init(&mut self, ecto_token_address: Address, lending_pool_address: Address) {
        let caller = self.env().caller();
        
        self.name.set(String::from("Aave ECTO"));
        self.symbol.set(String::from("aECTO"));
        self.decimals.set(18);
        self.total_supply.set(U256::zero());
        
        self.ecto_token.set(ecto_token_address);
        self.lending_pool.set(lending_pool_address);
        self.total_assets.set(U256::zero());
        
        self.admin.set(caller);
        self.paused.set(false);
    }
    
    // ========================================
    // CEP-18 Token Functions
    // ========================================
    
    pub fn name(&self) -> String {
        self.name.get_or_default()
    }
    
    pub fn symbol(&self) -> String {
        self.symbol.get_or_default()
    }
    
    pub fn decimals(&self) -> u8 {
        self.decimals.get_or_default()
    }
    
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get_or_default()
    }
    
    pub fn balance_of(&self, owner: Address) -> U256 {
        self.balances.get(&owner).unwrap_or(U256::zero())
    }
    
    pub fn transfer(&mut self, recipient: Address, amount: U256) {
        let sender = self.env().caller();
        self.transfer_internal(sender, recipient, amount);
    }
    
    pub fn approve(&mut self, spender: Address, amount: U256) {
        let owner = self.env().caller();
        self.allowances.set(&(owner, spender), amount);
    }
    
    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.get(&(owner, spender)).unwrap_or(U256::zero())
    }
    
    pub fn transfer_from(&mut self, owner: Address, recipient: Address, amount: U256) {
        let spender = self.env().caller();
        let current_allowance = self.allowance(owner, spender);
        
        if current_allowance < amount {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        self.allowances.set(&(owner, spender), current_allowance - amount);
        self.transfer_internal(owner, recipient, amount);
    }
    
    fn transfer_internal(&mut self, from: Address, to: Address, amount: U256) {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        self.balances.set(&from, from_balance - amount);
        let to_balance = self.balance_of(to);
        self.balances.set(&to, to_balance + amount);
    }
    
    // ========================================
    // Vault Management (Lending Pool Only)
    // ========================================
    
    /// Mint aECTO shares (lending pool only)
    pub fn mint(&mut self, to: Address, amount: U256) {
        self.only_lending_pool();
        
        let current_supply = self.total_supply.get_or_default();
        self.total_supply.set(current_supply + amount);
        
        let balance = self.balance_of(to);
        self.balances.set(&to, balance + amount);
    }
    
    /// Burn aECTO shares (lending pool only)
    pub fn burn(&mut self, from: Address, amount: U256) {
        self.only_lending_pool();
        
        let balance = self.balance_of(from);
        if balance < amount {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        self.balances.set(&from, balance - amount);
        
        let current_supply = self.total_supply.get_or_default();
        self.total_supply.set(current_supply - amount);
    }
    
    /// Update total assets (lending pool only)
    pub fn update_total_assets(&mut self, new_total: U256) {
        self.only_lending_pool();
        self.total_assets.set(new_total);
    }
    
    /// Get total assets
    pub fn get_total_assets(&self) -> U256 {
        self.total_assets.get_or_default()
    }
    
    /// Convert assets to shares (public wrapper for CEP-4626)
    pub fn convert_to_shares(&self, assets: U256) -> U256 {
        let total_supply = self.total_supply();
        let total_assets = self.total_assets.get_or_default();
        
        if total_supply == U256::zero() || total_assets == U256::zero() {
            return assets; // 1:1 initial rate
        }
        
        // shares = (assets * total_supply) / total_assets
        (assets * total_supply) / total_assets
    }
    
    /// Convert shares to assets (public wrapper for CEP-4626)
    pub fn convert_to_assets(&self, shares: U256) -> U256 {
        let total_supply = self.total_supply();
        
        if total_supply == U256::zero() {
            return U256::zero();
        }
        
        let total_assets = self.total_assets.get_or_default();
        // assets = (shares * total_assets) / total_supply
        (shares * total_assets) / total_supply
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    pub fn pause(&mut self) {
        self.only_admin();
        self.paused.set(true);
    }
    
    pub fn unpause(&mut self) {
        self.only_admin();
        self.paused.set(false);
    }
    
    pub fn is_paused(&self) -> bool {
        self.paused.get_or_default()
    }
    
    fn only_lending_pool(&self) {
        let caller = self.env().caller();
        let pool = self.lending_pool.get_or_revert_with(LendingError::Unauthorized);
        if caller != pool {
            self.env().revert(LendingError::Unauthorized);
        }
    }
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        if caller != admin {
            self.env().revert(LendingError::Unauthorized);
        }
    }
}

// ============================================================================
// CEP-4626 Implementation
// ============================================================================

impl Cep4626Vault for AectoVault {
    fn asset(&self) -> Address {
        self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration)
    }
    
    fn total_assets(&self) -> U256 {
        self.total_assets.get_or_default()
    }
    
    fn convert_to_shares(&self, assets: U256) -> U256 {
        let total_supply = self.total_supply();
        let total_assets = self.total_assets();
        
        if total_supply == U256::zero() || total_assets == U256::zero() {
            return assets; // 1:1 initial rate
        }
        
        // shares = (assets * total_supply) / total_assets
        (assets * total_supply) / total_assets
    }
    
    fn convert_to_assets(&self, shares: U256) -> U256 {
        let total_supply = self.total_supply();
        
        if total_supply == U256::zero() {
            return U256::zero();
        }
        
        let total_assets = self.total_assets();
        // assets = (shares * total_assets) / total_supply
        (shares * total_assets) / total_supply
    }
    
    fn max_deposit(&self, _receiver: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        U256::MAX
    }
    
    fn max_mint(&self, _receiver: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        U256::MAX
    }
    
    fn max_withdraw(&self, owner: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        let shares = self.balance_of(owner);
        self.convert_to_assets(shares)
    }
    
    fn max_redeem(&self, owner: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        self.balance_of(owner)
    }
    
    fn preview_deposit(&self, assets: U256) -> U256 {
        self.convert_to_shares(assets)
    }
    
    fn preview_mint(&self, shares: U256) -> U256 {
        self.convert_to_assets(shares)
    }
    
    fn preview_withdraw(&self, assets: U256) -> U256 {
        self.convert_to_shares(assets)
    }
    
    fn preview_redeem(&self, shares: U256) -> U256 {
        self.convert_to_assets(shares)
    }
    
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256 {
        if self.paused.get_or_default() {
            self.env().revert(LendingError::ContractPaused);
        }
        
        let caller = self.env().caller();
        let shares = self.convert_to_shares(assets);
        
        // Transfer ECTO from user to vault
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer_from(caller, Address::from(self.env().self_address()), assets);
        
        // Update total assets
        let current_total = self.total_assets.get_or_default();
        self.total_assets.set(current_total + assets);
        
        // Mint shares
        let current_supply = self.total_supply.get_or_default();
        self.total_supply.set(current_supply + shares);
        
        let balance = self.balance_of(receiver);
        self.balances.set(&receiver, balance + shares);
        
        // Emit CEP-4626 event
        self.env().emit_event(Cep4626Deposit {
            sender: caller,
            owner: receiver,
            assets,
            shares,
        });
        
        shares
    }
    
    fn mint(&mut self, shares: U256, receiver: Address) -> U256 {
        let assets = self.convert_to_assets(shares);
        let actual_shares = self.deposit(assets, receiver);
        
        if actual_shares < shares {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        assets
    }
    
    fn withdraw(&mut self, assets: U256, receiver: Address, owner: Address) -> U256 {
        if self.paused.get_or_default() {
            self.env().revert(LendingError::ContractPaused);
        }
        
        let caller = self.env().caller();
        let shares = self.convert_to_shares(assets);
        
        // Check allowance if caller is not owner
        if caller != owner {
            let allowance = self.allowance(owner, caller);
            if allowance < shares {
                self.env().revert(LendingError::Unauthorized);
            }
            self.allowances.set(&(owner, caller), allowance - shares);
        }
        
        // Check owner has sufficient shares
        let owner_balance = self.balance_of(owner);
        if owner_balance < shares {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        // Burn shares
        self.balances.set(&owner, owner_balance - shares);
        let current_supply = self.total_supply.get_or_default();
        self.total_supply.set(current_supply - shares);
        
        // Update total assets
        let current_total = self.total_assets.get_or_default();
        self.total_assets.set(current_total - assets);
        
        // Transfer ECTO to receiver
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer(receiver, assets);
        
        // Emit CEP-4626 event
        self.env().emit_event(Cep4626Withdraw {
            sender: caller,
            receiver,
            owner,
            assets,
            shares,
        });
        
        shares
    }
    
    fn redeem(&mut self, shares: U256, receiver: Address, owner: Address) -> U256 {
        let assets = self.convert_to_assets(shares);
        let actual_shares = self.withdraw(assets, receiver, owner);
        
        if actual_shares > shares {
            self.env().revert(LendingError::InsufficientBalance);
        }
        
        assets
    }
}
