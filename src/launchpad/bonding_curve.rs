//! BondingCurve - Manages token pricing and buy/sell operations
//!
//! The bonding curve determines the price of tokens based on supply.
//! Supports multiple curve types: Linear, Sigmoid, and Steep.

use odra::prelude::*;
use odra::casper_types::{U256, U512};
use odra::ContractRef;
use super::errors::LaunchpadError;
use super::launch_token::LaunchTokenContractRef;

/// Curve type enum (stored as u8)
pub const CURVE_LINEAR: u8 = 0;
pub const CURVE_SIGMOID: u8 = 1;
pub const CURVE_STEEP: u8 = 2;

/// Launch status enum (stored as u8)
pub const STATUS_ACTIVE: u8 = 0;
pub const STATUS_GRADUATED: u8 = 1;
pub const STATUS_REFUNDING: u8 = 2;

/// BondingCurve contract managing buy/sell operations
#[odra::module]
pub struct BondingCurve {
    /// Associated LaunchToken address
    token: Var<Address>,
    /// Curve type (0=Linear, 1=Sigmoid, 2=Steep)
    curve_type: Var<u8>,
    /// Total CSPR raised
    cspr_raised: Var<U512>,
    /// Total tokens sold
    tokens_sold: Var<U256>,
    /// CSPR threshold to graduate to DEX
    graduation_threshold: Var<U512>,
    /// Current status (0=Active, 1=Graduated, 2=Refunding)
    status: Var<u8>,
    /// Deadline timestamp for refund eligibility
    deadline: Var<u64>,
    /// Creator address
    creator: Var<Address>,
    /// Creator fee in basis points (100 = 1%)
    creator_fee_bps: Var<u64>,
    /// DEX Router address for graduation
    dex_router: Var<Address>,
    /// DEX Factory address for pair creation
    dex_factory: Var<Address>,
    /// User contributions for refund tracking: user -> CSPR amount
    contributions: Mapping<Address, U512>,
    /// Total token supply cap (1 billion tokens with 18 decimals)
    total_supply_cap: Var<U256>,
    /// Base price for curve calculations (in motes per token unit)
    base_price: Var<U512>,
}

#[odra::module]
impl BondingCurve {
    /// Initialize the bonding curve
    pub fn init(
        &mut self,
        token: Address,
        curve_type: u8,
        graduation_threshold: U512,
        deadline: u64,
        creator: Address,
        creator_fee_bps: u64,
        dex_router: Address,
        dex_factory: Address,
    ) {
        if curve_type > CURVE_STEEP {
            self.env().revert(LaunchpadError::InvalidCurveType);
        }

        self.token.set(token);
        self.curve_type.set(curve_type);
        self.graduation_threshold.set(graduation_threshold);
        self.deadline.set(deadline);
        self.creator.set(creator);
        self.creator_fee_bps.set(creator_fee_bps);
        self.dex_router.set(dex_router);
        self.dex_factory.set(dex_factory);
        self.status.set(STATUS_ACTIVE);
        self.cspr_raised.set(U512::zero());
        self.tokens_sold.set(U256::zero());
        
        // Set supply cap: 1 billion tokens with 18 decimals
        let supply_cap = U256::from(1_000_000_000u64) * U256::from(10u64).pow(U256::from(18));
        self.total_supply_cap.set(supply_cap);
        
        // Base price: 0.0001 CSPR per token (100_000 motes = 0.0001 CSPR)
        self.base_price.set(U512::from(100_000u64));
    }

    // ============ View Functions ============

    /// Get the associated token address
    pub fn token(&self) -> Address {
        self.token.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get the curve type
    pub fn curve_type(&self) -> u8 {
        self.curve_type.get_or_default()
    }

    /// Get total CSPR raised
    pub fn cspr_raised(&self) -> U512 {
        self.cspr_raised.get_or_default()
    }

    /// Get total tokens sold
    pub fn tokens_sold(&self) -> U256 {
        self.tokens_sold.get_or_default()
    }

    /// Get graduation threshold
    pub fn graduation_threshold(&self) -> U512 {
        self.graduation_threshold.get_or_default()
    }

    /// Get current status
    pub fn status(&self) -> u8 {
        self.status.get_or_default()
    }

    /// Get deadline
    pub fn deadline(&self) -> u64 {
        self.deadline.get_or_default()
    }

    /// Get creator address
    pub fn creator(&self) -> Address {
        self.creator.get_or_revert_with(LaunchpadError::Unauthorized)
    }

    /// Get user contribution
    pub fn contribution_of(&self, user: Address) -> U512 {
        self.contributions.get(&user).unwrap_or_default()
    }

    /// Get current token price in motes (per 1 token with 18 decimals)
    pub fn get_current_price(&self) -> U512 {
        let tokens_sold = self.tokens_sold.get_or_default();
        self.calculate_price(tokens_sold)
    }

    /// Get quote for buying tokens with given CSPR amount
    pub fn get_quote_buy(&self, cspr_amount: U512) -> U256 {
        if cspr_amount == U512::zero() {
            return U256::zero();
        }
        
        // Simplified: tokens = cspr_amount / current_price
        // In practice, we'd integrate over the curve for accurate amounts
        let current_price = self.get_current_price();
        if current_price == U512::zero() {
            return U256::zero();
        }
        
        // Convert: cspr_amount (U512) / price (U512) = tokens (need to handle decimals)
        // cspr_amount is in motes, price is motes per token
        // tokens = cspr_amount / price * 10^18 (to get full precision tokens)
        let one_token = U512::from(10u64).pow(U512::from(18));
        let tokens_u512 = (cspr_amount * one_token) / current_price;
        
        // Convert to U256 (safe since tokens won't exceed supply cap)
        U256::from(tokens_u512.as_u128())
    }

    /// Get quote for selling tokens
    pub fn get_quote_sell(&self, token_amount: U256) -> U512 {
        if token_amount == U256::zero() {
            return U512::zero();
        }
        
        let current_price = self.get_current_price();
        let one_token = U512::from(10u64).pow(U512::from(18));
        
        // cspr_out = token_amount * price / 10^18
        let token_amount_u512 = U512::from(token_amount.as_u128());
        (token_amount_u512 * current_price) / one_token
    }

    /// Get progress towards graduation (0-100)
    pub fn get_progress(&self) -> u8 {
        let raised = self.cspr_raised.get_or_default();
        let threshold = self.graduation_threshold.get_or_default();
        
        if threshold == U512::zero() {
            return 100;
        }
        
        let progress = (raised * U512::from(100u64)) / threshold;
        if progress > U512::from(100u64) {
            100
        } else {
            progress.as_u64() as u8
        }
    }

    // ============ Write Functions ============

    /// Buy tokens with attached CSPR value
    /// Note: Caller must attach CSPR value to this call
    pub fn buy(&mut self, min_tokens_out: U256) {
        // Check status
        let status = self.status.get_or_default();
        if status != STATUS_ACTIVE {
            self.env().revert(LaunchpadError::NotActive);
        }

        // Get attached value
        let cspr_amount = self.env().attached_value();
        if cspr_amount == U512::zero() {
            self.env().revert(LaunchpadError::InsufficientPayment);
        }

        // Calculate tokens to mint
        let tokens_out = self.get_quote_buy(cspr_amount);
        if tokens_out < min_tokens_out {
            self.env().revert(LaunchpadError::SlippageExceeded);
        }

        // Check supply cap
        let current_sold = self.tokens_sold.get_or_default();
        let new_sold = current_sold + tokens_out;
        let supply_cap = self.total_supply_cap.get_or_default();
        if new_sold > supply_cap {
            self.env().revert(LaunchpadError::InsufficientBalance);
        }

        // Deduct creator fee
        let creator_fee_bps = self.creator_fee_bps.get_or_default();
        let creator_fee = (cspr_amount * U512::from(creator_fee_bps)) / U512::from(10_000u64);
        let net_amount = cspr_amount - creator_fee;

        // Update state
        let current_raised = self.cspr_raised.get_or_default();
        self.cspr_raised.set(current_raised + net_amount);
        self.tokens_sold.set(new_sold);

        // Track user contribution for potential refund
        let caller = self.env().caller();
        let current_contribution = self.contributions.get(&caller).unwrap_or_default();
        self.contributions.set(&caller, current_contribution + net_amount);

        // Mint tokens to buyer
        let token_addr = self.token.get_or_revert_with(LaunchpadError::Unauthorized);
        let mut token = LaunchTokenContractRef::new(self.env(), token_addr);
        token.mint(caller, tokens_out);

        // Transfer creator fee
        if creator_fee > U512::zero() {
            let creator = self.creator.get_or_revert_with(LaunchpadError::Unauthorized);
            self.env().transfer_tokens(&creator, &creator_fee);
        }

        // Check if graduation threshold met
        let new_raised = self.cspr_raised.get_or_default();
        let threshold = self.graduation_threshold.get_or_default();
        if new_raised >= threshold {
            self.trigger_graduation();
        }
    }

    /// Sell tokens back to the curve
    pub fn sell(&mut self, amount: U256, min_cspr_out: U512) {
        // Check status
        let status = self.status.get_or_default();
        if status != STATUS_ACTIVE {
            self.env().revert(LaunchpadError::NotActive);
        }

        if amount == U256::zero() {
            self.env().revert(LaunchpadError::ZeroAmount);
        }

        // Calculate CSPR to return
        let cspr_out = self.get_quote_sell(amount);
        if cspr_out < min_cspr_out {
            self.env().revert(LaunchpadError::SlippageExceeded);
        }

        // Verify we have enough CSPR in the curve
        let current_raised = self.cspr_raised.get_or_default();
        if cspr_out > current_raised {
            self.env().revert(LaunchpadError::InsufficientBalance);
        }

        // Burn tokens from seller
        let caller = self.env().caller();
        let token_addr = self.token.get_or_revert_with(LaunchpadError::Unauthorized);
        let mut token = LaunchTokenContractRef::new(self.env(), token_addr);
        token.burn(caller, amount);

        // Update state
        let current_sold = self.tokens_sold.get_or_default();
        self.tokens_sold.set(current_sold - amount);
        self.cspr_raised.set(current_raised - cspr_out);

        // Update contribution tracking
        let current_contribution = self.contributions.get(&caller).unwrap_or_default();
        if cspr_out <= current_contribution {
            self.contributions.set(&caller, current_contribution - cspr_out);
        } else {
            self.contributions.set(&caller, U512::zero());
        }

        // Transfer CSPR to seller
        self.env().transfer_tokens(&caller, &cspr_out);
    }

    /// Graduate the launch to DEX (creates liquidity pair)
    pub fn graduate(&mut self) {
        let status = self.status.get_or_default();
        if status != STATUS_ACTIVE {
            self.env().revert(LaunchpadError::AlreadyGraduated);
        }

        let raised = self.cspr_raised.get_or_default();
        let threshold = self.graduation_threshold.get_or_default();
        if raised < threshold {
            self.env().revert(LaunchpadError::ThresholdNotMet);
        }

        self.trigger_graduation();
    }

    /// Claim refund if launch failed (deadline passed without graduation)
    pub fn claim_refund(&mut self) {
        let status = self.status.get_or_default();
        
        // Check if refunding is allowed
        if status == STATUS_GRADUATED {
            self.env().revert(LaunchpadError::AlreadyGraduated);
        }

        // If still active, check deadline
        if status == STATUS_ACTIVE {
            let deadline = self.deadline.get_or_default();
            let current_time = self.env().get_block_time();
            if current_time < deadline {
                self.env().revert(LaunchpadError::DeadlineNotPassed);
            }
            // Set status to refunding
            self.status.set(STATUS_REFUNDING);
        }

        // Process refund
        let caller = self.env().caller();
        let contribution = self.contributions.get(&caller).unwrap_or_default();
        if contribution == U512::zero() {
            self.env().revert(LaunchpadError::NoRefundAvailable);
        }

        // Clear contribution
        self.contributions.set(&caller, U512::zero());

        // Transfer refund
        self.env().transfer_tokens(&caller, &contribution);
    }

    // ============ Internal Functions ============

    /// Calculate price based on tokens sold (bonding curve formula)
    fn calculate_price(&self, tokens_sold: U256) -> U512 {
        let base_price = self.base_price.get_or_default();
        let curve_type = self.curve_type.get_or_default();
        let supply_cap = self.total_supply_cap.get_or_default();
        
        if supply_cap == U256::zero() {
            return base_price;
        }

        // Progress ratio (0 to 1, scaled by 10000 for precision)
        let progress = if supply_cap > U256::zero() {
            (tokens_sold * U256::from(10_000u64)) / supply_cap
        } else {
            U256::zero()
        };

        match curve_type {
            CURVE_LINEAR => {
                // Linear: price = base * (1 + progress * 10)
                // At 0%: price = base
                // At 100%: price = base * 11
                let multiplier = U512::from(10_000u64) + U512::from(progress.as_u128()) * U512::from(10u64);
                (base_price * multiplier) / U512::from(10_000u64)
            }
            CURVE_SIGMOID => {
                // Sigmoid approximation: steeper in the middle
                // Simplified: use quadratic for now
                let progress_u512 = U512::from(progress.as_u128());
                let multiplier = U512::from(10_000u64) + (progress_u512 * progress_u512) / U512::from(100u64);
                (base_price * multiplier) / U512::from(10_000u64)
            }
            CURVE_STEEP => {
                // Exponential-like: grows faster at higher supply
                // price = base * (1 + progress^2 * 50)
                let progress_u512 = U512::from(progress.as_u128());
                let multiplier = U512::from(10_000u64) + (progress_u512 * progress_u512 * U512::from(50u64)) / U512::from(10_000u64);
                (base_price * multiplier) / U512::from(10_000u64)
            }
            _ => base_price,
        }
    }

    /// Internal function to handle graduation
    fn trigger_graduation(&mut self) {
        // Mark as graduated
        self.status.set(STATUS_GRADUATED);

        // Note: In production, this would call the DEX Router to:
        // 1. Create a new pair via Factory
        // 2. Add initial liquidity with raised CSPR and remaining tokens
        // For now, we just update the status
        
        // TODO: Implement DEX integration
        // let router = self.dex_router.get_or_revert_with(LaunchpadError::DexIntegrationFailed);
        // let factory = self.dex_factory.get_or_revert_with(LaunchpadError::DexIntegrationFailed);
        // ... call router.add_liquidity(...)
    }
}

/// External interface for BondingCurve
#[odra::external_contract]
pub trait BondingCurveContract {
    fn token(&self) -> Address;
    fn curve_type(&self) -> u8;
    fn cspr_raised(&self) -> U512;
    fn tokens_sold(&self) -> U256;
    fn graduation_threshold(&self) -> U512;
    fn status(&self) -> u8;
    fn deadline(&self) -> u64;
    fn creator(&self) -> Address;
    fn contribution_of(&self, user: Address) -> U512;
    fn get_current_price(&self) -> U512;
    fn get_quote_buy(&self, cspr_amount: U512) -> U256;
    fn get_quote_sell(&self, token_amount: U256) -> U512;
    fn get_progress(&self) -> u8;
    fn buy(&mut self, min_tokens_out: U256);
    fn sell(&mut self, amount: U256, min_cspr_out: U512);
    fn graduate(&mut self);
    fn claim_refund(&mut self);
}
