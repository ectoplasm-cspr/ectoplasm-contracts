//! Staking Manager - Core contract for liquid staking operations
//! 
//! This contract manages the staking/unstaking of CSPR and minting/burning of sCSPR tokens.
//! It interfaces with Casper's native staking system and maintains the exchange rate.
//! 
//! **CEP-4626 Compliant**: This contract implements the CEP-4626 Tokenized Vault Standard
//! for liquid staking, providing a standardized interface for CSPR staking.

use odra::prelude::*;
use odra::casper_types::{U256, U512};
use odra::ContractRef;
use super::errors::LstError;
use super::events::*;
use super::scspr_token::ScsprTokenContractRef;
use crate::cep4626::{Cep4626Vault, Deposit as Cep4626Deposit, Withdraw as Cep4626Withdraw};

/// Represents an unstaking request
#[odra::odra_type]
pub struct UnstakeRequest {
    /// Address of the user who initiated unstaking
    pub user: Address,
    /// Amount of CSPR to be withdrawn
    pub cspr_amount: U256,
    /// Timestamp when the funds become withdrawable
    pub withdrawable_at: u64,
    /// Whether the request has been processed
    pub processed: bool,
}

/// Staking Manager contract
#[odra::module]
pub struct StakingManager {
    /// Reference to the sCSPR token contract address
    scspr_token_address: Var<Address>,
    
    /// Total amount of CSPR staked (including rewards)
    total_cspr_staked: Var<U256>,
    
    /// Total amount of sCSPR minted
    total_scspr_supply: Var<U256>,
    
    /// Minimum stake amount (in CSPR)
    minimum_stake: Var<U256>,
    
    /// Unstaking period in seconds (7 eras ≈ 57,600 seconds ≈ 16 hours)
    unstaking_period: Var<u64>,
    
    /// List of approved validators for delegation
    validators: Mapping<Address, bool>,
    
    /// Validator list (for iteration) - stored as mapping
    validator_list: Mapping<u32, Address>,
    
    /// Number of validators
    validator_count: Var<u32>,
    
    /// Amount staked per validator
    validator_stakes: Mapping<Address, U256>,
    
    /// Unstake requests mapping: request_id -> UnstakeRequest
    unstake_requests: Mapping<u64, UnstakeRequest>,
    
    /// User's unstake request IDs: user -> Vec<request_id>
    user_unstake_requests: Mapping<Address, Vec<u64>>,
    
    /// Next unstake request ID
    next_unstake_request_id: Var<u64>,
    
    /// Contract admin
    admin: Var<Address>,
    
    /// Whether the contract is paused
    paused: Var<bool>,
    
    /// Exchange rate scaling factor (1e18)
    exchange_rate_scale: Var<U256>,
}

#[odra::module]
impl StakingManager {
    /// Initialize the staking manager
    pub fn init(&mut self, scspr_token_address: Address) {
        let caller = self.env().caller();
        
        // Initialize sCSPR token reference
        self.scspr_token_address.set(scspr_token_address);
        
        // Set initial values
        self.total_cspr_staked.set(U256::zero());
        self.total_scspr_supply.set(U256::zero());
        self.minimum_stake.set(U256::from(100_000_000_000u64)); // 100 CSPR minimum (9 decimals)
        self.unstaking_period.set(57_600); // ~16 hours (7 eras)
        self.next_unstake_request_id.set(0);
        self.admin.set(caller);
        self.paused.set(false);
        self.exchange_rate_scale.set(U256::from(1_000_000_000_000_000_000u128)); // 1e18
        self.validator_count.set(0);
    }

    /// Stake CSPR and receive sCSPR
    /// 
    /// # Arguments
    /// * `validator` - The validator address to delegate to
    /// * `cspr_amount` - Amount of CSPR to stake
    /// 
    /// # Returns
    /// The amount of sCSPR minted
    pub fn stake(&mut self, validator: Address, cspr_amount: U256) -> U256 {
        self.ensure_not_paused();
        
        let caller = self.env().caller();
        
        // Validate amount
        if cspr_amount == U256::zero() {
            self.env().revert(LstError::InvalidAmount);
        }
        
        let minimum = self.minimum_stake.get_or_default();
        if cspr_amount < minimum {
            self.env().revert(LstError::BelowMinimumStake);
        }
        
        // Validate validator
        if !self.validators.get(&validator).unwrap_or(false) {
            self.env().revert(LstError::InvalidValidator);
        }
        
        // Calculate sCSPR amount based on current exchange rate
        let scspr_amount = self.calculate_scspr_amount(cspr_amount);
        
        // Update total staked
        let current_total = self.total_cspr_staked.get_or_default();
        self.total_cspr_staked.set(current_total + cspr_amount);
        
        // Update total sCSPR supply
        let current_supply = self.total_scspr_supply.get_or_default();
        self.total_scspr_supply.set(current_supply + scspr_amount);
        
        // Update validator stake
        let validator_stake = self.validator_stakes.get(&validator).unwrap_or_default();
        self.validator_stakes.set(&validator, validator_stake + cspr_amount);
        
        // Mint sCSPR to the user
        let token_address = self.scspr_token_address.get_or_revert_with(LstError::StakingFailed);
        let mut token = ScsprTokenContractRef::new(self.env(), token_address);
        token.mint(caller, scspr_amount);
        
        // TODO: Actual delegation to Casper validator would happen here
        // This would use Casper's native staking system calls
        
        // Emit event
        let exchange_rate = self.get_exchange_rate();
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Staked {
            staker: caller,
            cspr_amount,
            scspr_amount,
            validator,
            exchange_rate,
            timestamp,
        });
        
        scspr_amount
    }

    /// Unstake sCSPR and initiate withdrawal
    /// 
    /// # Arguments
    /// * `scspr_amount` - Amount of sCSPR to unstake
    /// 
    /// # Returns
    /// The unstake request ID
    pub fn unstake(&mut self, scspr_amount: U256) -> u64 {
        self.ensure_not_paused();
        
        let caller = self.env().caller();
        
        // Validate amount
        if scspr_amount == U256::zero() {
            self.env().revert(LstError::InvalidAmount);
        }
        
        // Check user's sCSPR balance
        let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
        let token = ScsprTokenContractRef::new(self.env(), token_address);
        let user_balance = token.balance_of(caller);
        if user_balance < scspr_amount {
            self.env().revert(LstError::InsufficientScsprBalance);
        }
        
        // Calculate CSPR amount based on current exchange rate
        let cspr_amount = self.calculate_cspr_amount(scspr_amount);
        
        // Burn sCSPR from user
        let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
        let mut token = ScsprTokenContractRef::new(self.env(), token_address);
        token.burn(caller, scspr_amount);
        
        // Update total sCSPR supply
        let current_supply = self.total_scspr_supply.get_or_default();
        self.total_scspr_supply.set(current_supply - scspr_amount);
        
        // Create unstake request
        let request_id = self.next_unstake_request_id.get_or_default();
        let timestamp = self.env().get_block_time();
        let unstaking_period = self.unstaking_period.get_or_default();
        let withdrawable_at = timestamp + unstaking_period;
        
        let request = UnstakeRequest {
            user: caller,
            cspr_amount,
            withdrawable_at,
            processed: false,
        };
        
        self.unstake_requests.set(&request_id, request);
        
        // Add to user's request list
        let mut user_requests = self.user_unstake_requests.get(&caller).unwrap_or_default();
        user_requests.push(request_id);
        self.user_unstake_requests.set(&caller, user_requests);
        
        // Increment request ID
        self.next_unstake_request_id.set(request_id + 1);
        
        // TODO: Actual undelegation from Casper validator would happen here
        
        // Emit event
        let exchange_rate = self.get_exchange_rate();
        self.env().emit_event(Unstaked {
            unstaker: caller,
            scspr_amount,
            cspr_amount,
            request_id,
            exchange_rate,
            withdrawable_at,
        });
        
        request_id
    }

    /// Withdraw unstaked CSPR after the unstaking period
    /// 
    /// # Arguments
    /// * `request_id` - The unstake request ID
    pub fn withdraw_unstaked(&mut self, request_id: u64) {
        self.ensure_not_paused();
        
        let caller = self.env().caller();
        
        // Get unstake request
        let mut request = self.unstake_requests.get(&request_id)
            .unwrap_or_else(|| self.env().revert(LstError::InvalidUnstakeRequestId));
        
        // Verify request belongs to caller
        if request.user != caller {
            self.env().revert(LstError::Unauthorized);
        }
        
        // Check if already processed
        if request.processed {
            self.env().revert(LstError::UnstakeRequestAlreadyProcessed);
        }
        
        // Check if unstaking period has passed
        let current_time = self.env().get_block_time();
        if current_time < request.withdrawable_at {
            self.env().revert(LstError::UnstakingPeriodNotComplete);
        }
        
        // Mark as processed
        request.processed = true;
        self.unstake_requests.set(&request_id, request.clone());
        
        // Update total staked
        let current_total = self.total_cspr_staked.get_or_default();
        self.total_cspr_staked.set(current_total - request.cspr_amount);
        
        // Transfer CSPR to user
        let cspr_amount_u512 = U512::from(request.cspr_amount.as_u128());
        self.env().transfer_tokens(&caller, &cspr_amount_u512);
        
        // Emit event
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Withdrawn {
            withdrawer: caller,
            cspr_amount: request.cspr_amount,
            request_id,
            timestamp,
        });
    }

    /// Distribute staking rewards (called periodically by admin or keeper)
    /// This updates the exchange rate based on accumulated rewards
    /// 
    /// # Arguments
    /// * `rewards_amount` - Amount of CSPR rewards earned
    pub fn distribute_rewards(&mut self, rewards_amount: U256) {
        self.only_admin();
        
        if rewards_amount == U256::zero() {
            return;
        }
        
        // Update total CSPR staked (includes rewards)
        let current_total = self.total_cspr_staked.get_or_default();
        let new_total = current_total + rewards_amount;
        self.total_cspr_staked.set(new_total);
        
        // Calculate new exchange rate
        let new_rate = self.get_exchange_rate();
        let total_scspr = self.total_scspr_supply.get_or_default();
        
        // Emit event
        let timestamp = self.env().get_block_time();
        self.env().emit_event(RewardsDistributed {
            rewards_amount,
            total_cspr_staked: new_total,
            total_scspr_supply: total_scspr,
            new_exchange_rate: new_rate,
            timestamp,
        });
    }

    // View functions

    /// Get the current exchange rate (sCSPR per CSPR, scaled by 1e18)
    /// If no sCSPR exists, rate is 1:1
    pub fn get_exchange_rate(&self) -> U256 {
        let total_scspr = self.total_scspr_supply.get_or_default();
        let total_cspr = self.total_cspr_staked.get_or_default();
        
        if total_scspr == U256::zero() || total_cspr == U256::zero() {
            // Initial rate: 1 sCSPR = 1 CSPR (scaled by 1e18)
            return self.exchange_rate_scale.get_or_default();
        }
        
        // Rate = (total_scspr * 1e18) / total_cspr
        // This gives us how much sCSPR per CSPR
        let scale = self.exchange_rate_scale.get_or_default();
        (total_scspr * scale) / total_cspr
    }

    /// Get the amount of CSPR for a given amount of sCSPR
    pub fn get_cspr_by_scspr(&self, scspr_amount: U256) -> U256 {
        self.calculate_cspr_amount(scspr_amount)
    }

    /// Get the amount of sCSPR for a given amount of CSPR
    pub fn get_scspr_by_cspr(&self, cspr_amount: U256) -> U256 {
        self.calculate_scspr_amount(cspr_amount)
    }

    /// Get total CSPR staked
    pub fn get_total_cspr_staked(&self) -> U256 {
        self.total_cspr_staked.get_or_default()
    }

    /// Get total sCSPR supply
    pub fn get_total_scspr_supply(&self) -> U256 {
        self.total_scspr_supply.get_or_default()
    }

    /// Get unstake request details
    pub fn get_unstake_request(&self, request_id: u64) -> Option<UnstakeRequest> {
        self.unstake_requests.get(&request_id)
    }

    /// Get all unstake request IDs for a user
    pub fn get_user_unstake_requests(&self, user: Address) -> Vec<u64> {
        self.user_unstake_requests.get(&user).unwrap_or_default()
    }

    /// Get minimum stake amount
    pub fn get_minimum_stake(&self) -> U256 {
        self.minimum_stake.get_or_default()
    }

    /// Get unstaking period
    pub fn get_unstaking_period(&self) -> u64 {
        self.unstaking_period.get_or_default()
    }

    /// Check if a validator is approved
    pub fn is_validator_approved(&self, validator: Address) -> bool {
        self.validators.get(&validator).unwrap_or(false)
    }

    /// Get all approved validators
    pub fn get_validators(&self) -> Vec<Address> {
        let count = self.validator_count.get_or_default();
        let mut validators = Vec::new();
        for i in 0..count {
            if let Some(validator) = self.validator_list.get(&i) {
                validators.push(validator);
            }
        }
        validators
    }

    /// Get stake amount for a validator
    pub fn get_validator_stake(&self, validator: Address) -> U256 {
        self.validator_stakes.get(&validator).unwrap_or_default()
    }

    // Admin functions

    /// Add a validator to the approved list
    pub fn add_validator(&mut self, validator: Address) {
        self.only_admin();
        
        if !self.validators.get(&validator).unwrap_or(false) {
            self.validators.set(&validator, true);
            let count = self.validator_count.get_or_default();
            self.validator_list.set(&count, validator);
            self.validator_count.set(count + 1);
            
            let timestamp = self.env().get_block_time();
            let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
            self.env().emit_event(ValidatorAdded {
                validator,
                added_by: admin,
                timestamp,
            });
        }
    }

    /// Remove a validator from the approved list
    pub fn remove_validator(&mut self, validator: Address) {
        self.only_admin();
        
        if self.validators.get(&validator).unwrap_or(false) {
            self.validators.set(&validator, false);
            // Note: We don't remove from validator_list to keep indices stable
            // The validator is just marked as not approved
            
            let timestamp = self.env().get_block_time();
            let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
            self.env().emit_event(ValidatorRemoved {
                validator,
                removed_by: admin,
                timestamp,
            });
        }
    }

    /// Update minimum stake amount
    pub fn set_minimum_stake(&mut self, new_minimum: U256) {
        self.only_admin();
        let old_minimum = self.minimum_stake.get_or_default();
        self.minimum_stake.set(new_minimum);
        
        let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
        self.env().emit_event(MinimumStakeUpdated {
            old_minimum,
            new_minimum,
            updated_by: admin,
        });
    }

    /// Update unstaking period
    pub fn set_unstaking_period(&mut self, new_period: u64) {
        self.only_admin();
        let old_period = self.unstaking_period.get_or_default();
        self.unstaking_period.set(new_period);
        
        let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
        self.env().emit_event(UnstakingPeriodUpdated {
            old_period,
            new_period,
            updated_by: admin,
        });
    }

    /// Pause the contract
    pub fn pause(&mut self) {
        self.only_admin();
        self.paused.set(true);
        
        let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
        let timestamp = self.env().get_block_time();
        self.env().emit_event(ContractPaused {
            paused_by: admin,
            timestamp,
        });
    }

    /// Unpause the contract
    pub fn unpause(&mut self) {
        self.only_admin();
        self.paused.set(false);
        
        let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
        let timestamp = self.env().get_block_time();
        self.env().emit_event(ContractUnpaused {
            unpaused_by: admin,
            timestamp,
        });
    }

    /// Transfer admin rights
    pub fn transfer_admin(&mut self, new_admin: Address) {
        self.only_admin();
        self.admin.set(new_admin);
    }

    /// Get admin address
    pub fn get_admin(&self) -> Address {
        self.admin.get_or_revert_with(LstError::Unauthorized)
    }

    /// Check if contract is paused
    pub fn is_paused(&self) -> bool {
        self.paused.get_or_default()
    }

    // Internal helper functions

    fn calculate_scspr_amount(&self, cspr_amount: U256) -> U256 {
        let total_scspr = self.total_scspr_supply.get_or_default();
        let total_cspr = self.total_cspr_staked.get_or_default();
        
        if total_scspr == U256::zero() || total_cspr == U256::zero() {
            // Initial stake: 1:1 ratio
            return cspr_amount;
        }
        
        // sCSPR = (cspr_amount * total_scspr) / total_cspr
        (cspr_amount * total_scspr) / total_cspr
    }

    fn calculate_cspr_amount(&self, scspr_amount: U256) -> U256 {
        let total_scspr = self.total_scspr_supply.get_or_default();
        let total_cspr = self.total_cspr_staked.get_or_default();
        
        if total_scspr == U256::zero() {
            return U256::zero();
        }
        
        // CSPR = (scspr_amount * total_cspr) / total_scspr
        (scspr_amount * total_cspr) / total_scspr
    }

    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LstError::Unauthorized);
        if caller != admin {
            self.env().revert(LstError::Unauthorized);
        }
    }

    fn ensure_not_paused(&self) {
        if self.paused.get_or_default() {
            self.env().revert(LstError::ContractPaused);
        }
    }
}

// ============================================================================
// CEP-4626 Tokenized Vault Standard Implementation
// ============================================================================

impl Cep4626Vault for StakingManager {
    // ========================================
    // Metadata
    // ========================================
    
    fn asset(&self) -> Address {
        // For liquid staking, the asset is CSPR
        // In Casper, we return a special address representing native CSPR
        // This could be a wrapped CSPR token address or a designated constant
        // For now, we'll use the contract's own address as a placeholder
        // TODO: Replace with actual CSPR token address when available
        Address::from(self.env().self_address())
    }
    
    fn total_assets(&self) -> U256 {
        // Total CSPR managed by the vault (including staking rewards)
        self.total_cspr_staked.get_or_default()
    }
    
    // ========================================
    // Conversion Functions
    // ========================================
    
    fn convert_to_shares(&self, assets: U256) -> U256 {
        // Convert CSPR to sCSPR shares
        self.calculate_scspr_amount(assets)
    }
    
    fn convert_to_assets(&self, shares: U256) -> U256 {
        // Convert sCSPR shares to CSPR
        self.calculate_cspr_amount(shares)
    }
    
    // ========================================
    // Deposit/Withdrawal Limits
    // ========================================
    
    fn max_deposit(&self, _receiver: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        // No maximum deposit limit for liquid staking
        U256::MAX
    }
    
    fn max_mint(&self, _receiver: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        // No maximum mint limit
        U256::MAX
    }
    
    fn max_withdraw(&self, owner: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        // Maximum withdrawal is the user's sCSPR balance converted to CSPR
        let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
        let token = ScsprTokenContractRef::new(self.env(), token_address);
        let user_shares = token.balance_of(owner);
        self.convert_to_assets(user_shares)
    }
    
    fn max_redeem(&self, owner: Address) -> U256 {
        if self.paused.get_or_default() {
            return U256::zero();
        }
        // Maximum redeem is the user's sCSPR balance
        let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
        let token = ScsprTokenContractRef::new(self.env(), token_address);
        token.balance_of(owner)
    }
    
    // ========================================
    // Preview Functions
    // ========================================
    
    fn preview_deposit(&self, assets: U256) -> U256 {
        // Preview how many sCSPR shares would be minted for assets
        self.convert_to_shares(assets)
    }
    
    fn preview_mint(&self, shares: U256) -> U256 {
        // Preview how many CSPR assets are needed to mint shares
        self.convert_to_assets(shares)
    }
    
    fn preview_withdraw(&self, assets: U256) -> U256 {
        // Preview how many sCSPR shares would be burned to withdraw assets
        self.convert_to_shares(assets)
    }
    
    fn preview_redeem(&self, shares: U256) -> U256 {
        // Preview how many CSPR assets would be received for redeeming shares
        self.convert_to_assets(shares)
    }
    
    // ========================================
    // Deposit/Mint Functions
    // ========================================
    
    fn deposit(&mut self, assets: U256, receiver: Address) -> U256 {
        // CEP-4626 deposit: stake CSPR and mint sCSPR to receiver
        // Note: For liquid staking, we need a validator parameter
        // We'll use the first available validator
        let validators = self.get_validators();
        if validators.is_empty() {
            self.env().revert(LstError::InvalidValidator);
        }
        let validator = validators[0];
        
        // Perform the stake operation
        let shares = self.stake(validator, assets);
        
        // If receiver is different from caller, transfer shares
        let caller = self.env().caller();
        if receiver != caller {
            let token_address = self.scspr_token_address.get_or_revert_with(LstError::StakingFailed);
            let mut token = ScsprTokenContractRef::new(self.env(), token_address);
            token.transfer_from(caller, receiver, shares);
        }
        
        // Emit CEP-4626 Deposit event
        self.env().emit_event(Cep4626Deposit {
            sender: caller,
            owner: receiver,
            assets,
            shares,
        });
        
        shares
    }
    
    fn mint(&mut self, shares: U256, receiver: Address) -> U256 {
        // CEP-4626 mint: calculate required CSPR and stake to mint exact shares
        let assets = self.convert_to_assets(shares);
        
        // Use deposit to perform the operation
        let actual_shares = self.deposit(assets, receiver);
        
        // Verify we minted at least the requested shares
        if actual_shares < shares {
            self.env().revert(LstError::InsufficientScsprBalance);
        }
        
        assets
    }
    
    // ========================================
    // Withdraw/Redeem Functions
    // ========================================
    
    fn withdraw(&mut self, assets: U256, receiver: Address, owner: Address) -> U256 {
        // CEP-4626 withdraw: burn sCSPR shares to withdraw exact CSPR amount
        // Note: Liquid staking has an unstaking period, so this initiates withdrawal
        
        let caller = self.env().caller();
        
        // Calculate shares needed
        let shares = self.convert_to_shares(assets);
        
        // Check allowance if caller is not owner
        if caller != owner {
            let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
            let token = ScsprTokenContractRef::new(self.env(), token_address);
            let allowance = token.allowance(owner, caller);
            if allowance < shares {
                self.env().revert(LstError::Unauthorized);
            }
        }
        
        // Initiate unstaking (this burns shares and creates withdrawal request)
        let _request_id = self.unstake(shares);
        
        // Emit CEP-4626 Withdraw event
        self.env().emit_event(Cep4626Withdraw {
            sender: caller,
            receiver,
            owner,
            assets,
            shares,
        });
        
        // Note: Actual withdrawal happens later via withdraw(request_id)
        // For CEP-4626 compliance, we return shares burned
        shares
    }
    
    fn redeem(&mut self, shares: U256, receiver: Address, owner: Address) -> U256 {
        // CEP-4626 redeem: burn exact sCSPR shares to withdraw CSPR
        let caller = self.env().caller();
        
        // Check allowance if caller is not owner
        if caller != owner {
            let token_address = self.scspr_token_address.get_or_revert_with(LstError::UnstakingFailed);
            let token = ScsprTokenContractRef::new(self.env(), token_address);
            let allowance = token.allowance(owner, caller);
            if allowance < shares {
                self.env().revert(LstError::Unauthorized);
            }
        }
        
        // Calculate assets to receive
        let assets = self.convert_to_assets(shares);
        
        // Initiate unstaking (this burns shares and creates withdrawal request)
        let _request_id = self.unstake(shares);
        
        // Emit CEP-4626 Withdraw event
        self.env().emit_event(Cep4626Withdraw {
            sender: caller,
            receiver,
            owner,
            assets,
            shares,
        });
        
        // Note: Actual withdrawal happens later via withdraw_unstaked(request_id)
        // For CEP-4626 compliance, we return assets that will be received
        assets
    }
}
