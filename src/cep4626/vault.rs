//! CEP-4626 Tokenized Vault Standard Interface
//! 
//! This trait defines the standard interface for tokenized vaults on Casper.
//! All vaults MUST implement CEP-18 (via the share token) and this interface.

use odra::prelude::*;
use odra::casper_types::U256;

/// CEP-4626 Tokenized Vault Interface
/// 
/// A vault represents shares of a single underlying CEP-18 token.
/// The vault token itself is CEP-18 compatible and represents shares.
pub trait Cep4626Vault {
    // ============================================
    // Vault Metadata
    // ============================================
    
    /// Returns the address of the underlying token used for the vault
    /// 
    /// - MUST be a CEP-18 token contract
    /// - MUST NOT revert
    fn asset(&self) -> Address;
    
    /// Returns the total amount of the underlying asset managed by the vault
    /// 
    /// - SHOULD include any compounding that occurs from yield
    /// - MUST be inclusive of any fees charged against assets
    /// - MUST NOT revert
    fn total_assets(&self) -> U256;
    
    // ============================================
    // Deposit/Withdrawal Limits
    // ============================================
    
    /// Returns the maximum amount of assets that can be deposited for a receiver
    /// 
    /// - MUST return the maximum amount that `deposit` would allow
    /// - MUST factor in both global and user-specific limits
    /// - MUST return 0 if deposits are disabled
    /// - MUST return U256::MAX if there is no limit
    /// - MUST NOT revert
    fn max_deposit(&self, receiver: Address) -> U256;
    
    /// Returns the maximum amount of shares that can be minted for a receiver
    /// 
    /// - MUST return the maximum amount that `mint` would allow
    /// - MUST factor in both global and user-specific limits
    /// - MUST return 0 if minting is disabled
    /// - MUST return U256::MAX if there is no limit
    /// - MUST NOT revert
    fn max_mint(&self, receiver: Address) -> U256;
    
    /// Returns the maximum amount of assets that can be withdrawn by an owner
    /// 
    /// - MUST return the maximum amount that `withdraw` would allow
    /// - MUST factor in both global and user-specific limits
    /// - MUST return 0 if withdrawal is disabled
    /// - MUST NOT revert
    fn max_withdraw(&self, owner: Address) -> U256;
    
    /// Returns the maximum amount of shares that can be redeemed by an owner
    /// 
    /// - MUST return the maximum amount that `redeem` would allow
    /// - MUST factor in both global and user-specific limits
    /// - MUST return 0 if redemption is disabled
    /// - MUST NOT revert
    fn max_redeem(&self, owner: Address) -> U256;
    
    // ============================================
    // Conversion Functions
    // ============================================
    
    /// Converts an amount of assets to shares
    /// 
    /// - MUST NOT be inclusive of any fees
    /// - MUST NOT show variations depending on the caller
    /// - MUST NOT reflect slippage or other on-chain conditions
    /// - MUST round down towards 0
    /// - MUST NOT revert unless due to integer overflow
    fn convert_to_shares(&self, assets: U256) -> U256;
    
    /// Converts an amount of shares to assets
    /// 
    /// - MUST NOT be inclusive of any fees
    /// - MUST NOT show variations depending on the caller
    /// - MUST NOT reflect slippage or other on-chain conditions
    /// - MUST round down towards 0
    /// - MUST NOT revert unless due to integer overflow
    fn convert_to_assets(&self, shares: U256) -> U256;
    
    // ============================================
    // Preview Functions
    // ============================================
    
    /// Simulates the effects of a deposit at the current block
    /// 
    /// - MUST return as close to and no more than the exact amount of shares
    ///   that would be minted in a `deposit` call
    /// - MUST be inclusive of deposit fees
    /// - MUST NOT account for deposit limits
    /// - MUST NOT revert due to vault-specific limits
    fn preview_deposit(&self, assets: U256) -> U256;
    
    /// Simulates the effects of a mint at the current block
    /// 
    /// - MUST return as close to and no fewer than the exact amount of assets
    ///   that would be deposited in a `mint` call
    /// - MUST be inclusive of deposit fees
    /// - MUST NOT account for mint limits
    /// - MUST NOT revert due to vault-specific limits
    fn preview_mint(&self, shares: U256) -> U256;
    
    /// Simulates the effects of a withdrawal at the current block
    /// 
    /// - MUST return as close to and no fewer than the exact amount of shares
    ///   that would be burned in a `withdraw` call
    /// - MUST be inclusive of withdrawal fees
    /// - MUST NOT account for withdrawal limits
    /// - MUST NOT revert due to vault-specific limits
    fn preview_withdraw(&self, assets: U256) -> U256;
    
    /// Simulates the effects of a redemption at the current block
    /// 
    /// - MUST return as close to and no more than the exact amount of assets
    ///   that would be withdrawn in a `redeem` call
    /// - MUST be inclusive of withdrawal fees
    /// - MUST NOT account for redemption limits
    /// - MUST NOT revert due to vault-specific limits
    fn preview_redeem(&self, shares: U256) -> U256;
    
    // ============================================
    // Deposit/Mint Functions
    // ============================================
    
    /// Deposits assets and mints shares to receiver
    /// 
    /// - MUST emit the Deposit event
    /// - MUST support CEP-18 approve/transfer_from flow on the asset
    /// - MUST revert if all of assets cannot be deposited
    /// 
    /// Returns the amount of shares minted
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256;
    
    /// Mints exact shares to receiver by depositing assets
    /// 
    /// - MUST emit the Deposit event
    /// - MUST support CEP-18 approve/transfer_from flow on the asset
    /// - MUST revert if all of shares cannot be minted
    /// 
    /// Returns the amount of assets deposited
    fn mint(&mut self, shares: U256, receiver: Address) -> U256;
    
    // ============================================
    // Withdraw/Redeem Functions
    // ============================================
    
    /// Withdraws assets to receiver by burning owner's shares
    /// 
    /// - MUST emit the Withdraw event
    /// - MUST support a withdraw flow where the shares are burned from owner directly
    ///   OR where the shares are transferred to the vault before withdrawal
    /// - MUST revert if all of assets cannot be withdrawn
    /// 
    /// Returns the amount of shares burned
    fn withdraw(&mut self, assets: U256, receiver: Address, owner: Address) -> U256;
    
    /// Redeems shares from owner and sends assets to receiver
    /// 
    /// - MUST emit the Withdraw event
    /// - MUST support a redeem flow where the shares are burned from owner directly
    ///   OR where the shares are transferred to the vault before redemption
    /// - MUST revert if all of shares cannot be redeemed
    /// 
    /// Returns the amount of assets sent to receiver
    fn redeem(&mut self, shares: U256, receiver: Address, owner: Address) -> U256;
}

/// Helper functions for implementing CEP-4626 vaults
pub mod helpers {
    use super::*;
    
    /// Calculate shares from assets using the vault's exchange rate
    /// 
    /// Formula: shares = (assets * total_shares) / total_assets
    /// If total_shares == 0, returns assets (1:1 initial rate)
    pub fn calculate_shares(assets: U256, total_assets: U256, total_shares: U256) -> U256 {
        if total_shares == U256::zero() || total_assets == U256::zero() {
            // Initial deposit: 1:1 ratio
            assets
        } else {
            // shares = (assets * total_shares) / total_assets
            (assets * total_shares) / total_assets
        }
    }
    
    /// Calculate assets from shares using the vault's exchange rate
    /// 
    /// Formula: assets = (shares * total_assets) / total_shares
    /// If total_shares == 0, returns 0
    pub fn calculate_assets(shares: U256, total_assets: U256, total_shares: U256) -> U256 {
        if total_shares == U256::zero() {
            U256::zero()
        } else {
            // assets = (shares * total_assets) / total_shares
            (shares * total_assets) / total_shares
        }
    }
}
