//! Events for CEP-4626 Tokenized Vaults

use odra::prelude::*;
use odra::casper_types::U256;

/// Event emitted when assets are deposited into the vault
#[odra::event]
pub struct Deposit {
    /// Address that called the deposit function
    pub sender: Address,
    /// Address that received the shares
    pub owner: Address,
    /// Amount of assets deposited
    pub assets: U256,
    /// Amount of shares minted
    pub shares: U256,
}

/// Event emitted when shares are redeemed from the vault
#[odra::event]
pub struct Withdraw {
    /// Address that called the withdraw function
    pub sender: Address,
    /// Address that received the assets
    pub receiver: Address,
    /// Address that owned the shares
    pub owner: Address,
    /// Amount of assets withdrawn
    pub assets: U256,
    /// Amount of shares burned
    pub shares: U256,
}
