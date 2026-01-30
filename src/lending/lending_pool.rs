//! Lending Pool - Main contract for deposits, borrows, and repayments
//! 
//! Core lending protocol contract that coordinates:
//! - ECTO deposits and withdrawals (via aECTO vault)
//! - Borrowing against collateral
//! - Repayments
//! - Liquidations
//! - Interest accrual

use odra::prelude::*;
use odra::casper_types::U256;
use odra::ContractRef;
use super::errors::LendingError;
use super::events::*;
use super::aecto_vault::AectoVaultContractRef;
use super::collateral_manager::CollateralManagerContractRef;
use super::interest_rate::InterestRateStrategyContractRef;
use super::liquidation::LiquidationEngineContractRef;
use super::price_oracle::PriceOracleContractRef;
use crate::token::Cep18TokenContractRef;

/// User's borrow position
#[odra::odra_type]
pub struct BorrowPosition {
    /// User address
    pub user: Address,
    /// Principal borrowed
    pub principal: U256,
    /// Interest accrued
    pub interest_accrued: U256,
    /// Timestamp of last update
    pub last_update: u64,
}

/// Lending Pool contract
#[odra::module]
pub struct LendingPool {
    /// aECTO vault address
    aecto_vault: Var<Address>,
    /// Collateral manager address
    collateral_manager: Var<Address>,
    /// Interest rate strategy address
    interest_rate_strategy: Var<Address>,
    /// Liquidation engine address
    liquidation_engine: Var<Address>,
    /// Price oracle address
    price_oracle: Var<Address>,
    /// ECTO token address
    ecto_token: Var<Address>,
    /// User borrow positions
    borrow_positions: Mapping<Address, BorrowPosition>,
    /// Total borrows (principal + interest)
    total_borrows: Var<U256>,
    /// Total liquidity available
    total_liquidity: Var<U256>,
    /// Current borrow rate (annual, scaled by 1e18)
    borrow_rate: Var<U256>,
    /// Current supply rate (annual, scaled by 1e18)
    supply_rate: Var<U256>,
    /// Reserve factor (percentage of interest going to reserves, scaled by 1e18)
    reserve_factor: Var<U256>,
    /// Total reserves accumulated
    total_reserves: Var<U256>,
    /// Admin address
    admin: Var<Address>,
    /// Paused state
    paused: Var<bool>,
}

#[odra::module]
impl LendingPool {
    /// Initialize the lending pool
    pub fn init(
        &mut self,
        aecto_vault_address: Address,
        collateral_manager_address: Address,
        interest_rate_strategy_address: Address,
        liquidation_engine_address: Address,
        price_oracle_address: Address,
        ecto_token_address: Address,
    ) {
        let caller = self.env().caller();
        
        self.aecto_vault.set(aecto_vault_address);
        self.collateral_manager.set(collateral_manager_address);
        self.interest_rate_strategy.set(interest_rate_strategy_address);
        self.liquidation_engine.set(liquidation_engine_address);
        self.price_oracle.set(price_oracle_address);
        self.ecto_token.set(ecto_token_address);
        
        self.total_borrows.set(U256::zero());
        self.total_liquidity.set(U256::zero());
        self.borrow_rate.set(U256::zero());
        self.supply_rate.set(U256::zero());
        
        // Default 10% reserve factor
        self.reserve_factor.set(U256::from(100_000_000_000_000_000u128)); // 0.1 * 1e18
        self.total_reserves.set(U256::zero());
        
        self.admin.set(caller);
        self.paused.set(false);
    }
    
    // ========================================
    // Deposit/Withdrawal (via aECTO vault)
    // ========================================
    
    /// Deposit ECTO and receive aECTO
    /// Note: Users should call aECTO vault directly for CEP-4626 interface
    pub fn deposit(&mut self, amount: U256) -> U256 {
        self.ensure_not_paused();
        self.accrue_interest();
        
        let caller = self.env().caller();
        
        // Transfer ECTO from user to pool
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer_from(caller, Address::from(self.env().self_address()), amount);
        
        // Update liquidity
        let current_liquidity = self.total_liquidity.get_or_default();
        self.total_liquidity.set(current_liquidity + amount);
        
        // Mint aECTO via vault
        let vault_address = self.aecto_vault.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut vault = AectoVaultContractRef::new(self.env(), vault_address);
        
        // Calculate shares
        let shares = vault.convert_to_shares(amount);
        vault.mint(caller, shares);
        
        // Update total assets in vault
        let new_total_assets = current_liquidity + amount + self.total_borrows.get_or_default();
        vault.update_total_assets(new_total_assets);
        
        // Update interest rates
        self.update_interest_rates();
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Deposited {
            user: caller,
            amount,
            shares,
            timestamp,
        });
        
        shares
    }
    
    /// Withdraw ECTO by burning aECTO
    pub fn withdraw(&mut self, amount: U256) -> U256 {
        self.ensure_not_paused();
        self.accrue_interest();
        
        let caller = self.env().caller();
        
        // Check liquidity
        let current_liquidity = self.total_liquidity.get_or_default();
        if amount > current_liquidity {
            self.env().revert(LendingError::InsufficientLiquidity);
        }
        
        // Calculate shares to burn
        let vault_address = self.aecto_vault.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut vault = AectoVaultContractRef::new(self.env(), vault_address);
        let shares = vault.convert_to_shares(amount);
        
        // Burn aECTO
        vault.burn(caller, shares);
        
        // Update liquidity
        self.total_liquidity.set(current_liquidity - amount);
        
        // Update total assets in vault
        let new_total_assets = current_liquidity - amount + self.total_borrows.get_or_default();
        vault.update_total_assets(new_total_assets);
        
        // Transfer ECTO to user
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer(caller, amount);
        
        // Update interest rates
        self.update_interest_rates();
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Withdrawn {
            user: caller,
            amount,
            shares,
            timestamp,
        });
        
        shares
    }
    
    // ========================================
    // Borrowing
    // ========================================
    
    /// Borrow ECTO against collateral
    pub fn borrow(&mut self, amount: U256, collateral_asset: Address) {
        self.ensure_not_paused();
        self.accrue_interest();
        
        let caller = self.env().caller();
        
        if amount == U256::zero() {
            self.env().revert(LendingError::ZeroAmount);
        }
        
        // Check liquidity
        let current_liquidity = self.total_liquidity.get_or_default();
        if amount > current_liquidity {
            self.env().revert(LendingError::InsufficientLiquidity);
        }
        
        // Get collateral manager
        let collateral_mgr_address = self.collateral_manager.get_or_revert_with(LendingError::InvalidConfiguration);
        let collateral_mgr = CollateralManagerContractRef::new(self.env(), collateral_mgr_address);
        
        // Check user has collateral
        let user_collateral = collateral_mgr.get_user_collateral(caller, collateral_asset);
        if user_collateral == U256::zero() {
            self.env().revert(LendingError::InsufficientCollateral);
        }
        
        // Get current debt
        let position = self.borrow_positions.get(&caller);
        let (current_debt, new_principal) = if let Some(pos) = position {
            let debt = pos.principal + pos.interest_accrued;
            let principal = pos.principal + pos.interest_accrued + amount;
            (debt, principal)
        } else {
            (U256::zero(), amount)
        };
        
        let new_debt = current_debt + amount;
        
        // Check borrow limit
        let max_borrow = collateral_mgr.get_max_borrow_amount(caller);
        if new_debt > max_borrow {
            self.env().revert(LendingError::ExceedsBorrowLimit);
        }
        
        // Check health factor
        let health_factor = collateral_mgr.calculate_health_factor(caller, new_debt);
        let scale = U256::from(1_000_000_000_000_000_000u128); // 1e18
        if health_factor < scale {
            self.env().revert(LendingError::HealthFactorTooLow);
        }
        
        // Update borrow position
        let new_position = BorrowPosition {
            user: caller,
            principal: new_principal,
            interest_accrued: U256::zero(),
            last_update: self.env().get_block_time(),
        };
        self.borrow_positions.set(&caller, new_position);
        
        // Update totals
        let total_borrows = self.total_borrows.get_or_default();
        self.total_borrows.set(total_borrows + amount);
        self.total_liquidity.set(current_liquidity - amount);
        
        // Transfer ECTO to borrower
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer(caller, amount);
        
        // Update interest rates
        self.update_interest_rates();
        
        let timestamp = self.env().get_block_time();
        let borrow_rate = self.borrow_rate.get_or_default();
        self.env().emit_event(Borrowed {
            borrower: caller,
            amount,
            collateral_asset,
            borrow_rate,
            timestamp,
        });
    }
    
    /// Repay borrowed ECTO
    pub fn repay(&mut self, amount: U256) {
        self.ensure_not_paused();
        self.accrue_interest();
        
        let caller = self.env().caller();
        
        if amount == U256::zero() {
            self.env().revert(LendingError::ZeroAmount);
        }
        
        // Get borrow position
        let position = self.borrow_positions.get(&caller)
            .unwrap_or_revert_with(&self.env(), LendingError::NoBorrowPosition);
        
        let total_debt = position.principal + position.interest_accrued;
        
        // Calculate actual repayment amount
        let repay_amount = if amount > total_debt {
            total_debt
        } else {
            amount
        };
        
        // Transfer ECTO from user to pool
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer_from(caller, Address::from(self.env().self_address()), repay_amount);
        
        // Calculate interest paid
        let interest_paid = if repay_amount >= position.interest_accrued {
            position.interest_accrued
        } else {
            repay_amount
        };
        
        let principal_paid = repay_amount - interest_paid;
        
        // Update position
        let new_debt = total_debt - repay_amount;
        if new_debt == U256::zero() {
            // Fully repaid, remove position
            self.borrow_positions.set(&caller, BorrowPosition {
                user: caller,
                principal: U256::zero(),
                interest_accrued: U256::zero(),
                last_update: self.env().get_block_time(),
            });
        } else {
            self.borrow_positions.set(&caller, BorrowPosition {
                user: caller,
                principal: position.principal - principal_paid,
                interest_accrued: position.interest_accrued - interest_paid,
                last_update: self.env().get_block_time(),
            });
        }
        
        // Update totals
        let total_borrows = self.total_borrows.get_or_default();
        self.total_borrows.set(total_borrows - repay_amount);
        
        let current_liquidity = self.total_liquidity.get_or_default();
        self.total_liquidity.set(current_liquidity + repay_amount);
        
        // Allocate interest to reserves
        let reserve_factor = self.reserve_factor.get_or_default();
        let scale = U256::from(1_000_000_000_000_000_000u128); // 1e18
        let reserves_added = (interest_paid * reserve_factor) / scale;
        let total_reserves = self.total_reserves.get_or_default();
        self.total_reserves.set(total_reserves + reserves_added);
        
        // Update interest rates
        self.update_interest_rates();
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(Repaid {
            borrower: caller,
            amount: repay_amount,
            interest: interest_paid,
            timestamp,
        });
    }
    
    // ========================================
    // Liquidation
    // ========================================
    
    /// Liquidate an undercollateralized position
    pub fn liquidate(
        &mut self,
        borrower: Address,
        debt_to_cover: U256,
        collateral_asset: Address,
    ) {
        self.ensure_not_paused();
        self.accrue_interest();
        
        let liquidator = self.env().caller();
        
        // Get borrower's position
        let position = self.borrow_positions.get(&borrower)
            .unwrap_or_revert_with(&self.env(), LendingError::NoBorrowPosition);
        
        let total_debt = position.principal + position.interest_accrued;
        
        if total_debt == U256::zero() {
            self.env().revert(LendingError::NoBorrowPosition);
        }
        
        // Check if position can be liquidated
        let collateral_mgr_address = self.collateral_manager.get_or_revert_with(LendingError::InvalidConfiguration);
        let collateral_mgr = CollateralManagerContractRef::new(self.env(), collateral_mgr_address);
        
        if !collateral_mgr.can_liquidate(borrower, total_debt) {
            self.env().revert(LendingError::PositionHealthy);
        }
        
        // Get collateral config
        let collateral_config = collateral_mgr.get_collateral_config(collateral_asset);
        
        // Get liquidation engine
        let liquidation_engine_address = self.liquidation_engine.get_or_revert_with(LendingError::InvalidConfiguration);
        let liquidation_engine = LiquidationEngineContractRef::new(self.env(), liquidation_engine_address);
        
        // Calculate liquidation amounts
        let borrower_collateral = collateral_mgr.get_user_collateral(borrower, collateral_asset);
        let oracle_address = self.price_oracle.get_or_revert_with(LendingError::InvalidConfiguration);
        let oracle = PriceOracleContractRef::new(self.env(), oracle_address);
        let collateral_value = oracle.get_asset_value(collateral_asset, borrower_collateral);
        
        let (actual_debt_covered, collateral_to_seize) = liquidation_engine.calculate_liquidation_amounts(
            debt_to_cover,
            total_debt,
            collateral_value,
            collateral_config.liquidation_bonus,
        );
        
        // Transfer debt payment from liquidator
        let ecto_address = self.ecto_token.get_or_revert_with(LendingError::InvalidConfiguration);
        let mut ecto_token = Cep18TokenContractRef::new(self.env(), ecto_address);
        ecto_token.transfer_from(liquidator, Address::from(self.env().self_address()), actual_debt_covered);
        
        // Update borrower's debt
        let new_debt = total_debt - actual_debt_covered;
        if new_debt == U256::zero() {
            self.borrow_positions.set(&borrower, BorrowPosition {
                user: borrower,
                principal: U256::zero(),
                interest_accrued: U256::zero(),
                last_update: self.env().get_block_time(),
            });
        } else {
            // Reduce principal proportionally
            let principal_covered = (position.principal * actual_debt_covered) / total_debt;
            let interest_covered = actual_debt_covered - principal_covered;
            
            self.borrow_positions.set(&borrower, BorrowPosition {
                user: borrower,
                principal: position.principal - principal_covered,
                interest_accrued: position.interest_accrued - interest_covered,
                last_update: self.env().get_block_time(),
            });
        }
        
        // Transfer collateral from borrower to liquidator
        // This is done through collateral manager
        let collateral_amount_in_tokens = oracle.get_asset_amount(collateral_asset, collateral_to_seize);
        
        // Note: In a full implementation, we'd need to handle the collateral transfer
        // For now, we emit the event with the amounts
        
        // Update totals
        let total_borrows = self.total_borrows.get_or_default();
        self.total_borrows.set(total_borrows - actual_debt_covered);
        
        let current_liquidity = self.total_liquidity.get_or_default();
        self.total_liquidity.set(current_liquidity + actual_debt_covered);
        
        // Update interest rates
        self.update_interest_rates();
        
        let timestamp = self.env().get_block_time();
        let liquidation_bonus = collateral_to_seize - actual_debt_covered;
        self.env().emit_event(Liquidated {
            borrower,
            liquidator,
            collateral_asset,
            debt_covered: actual_debt_covered,
            collateral_seized: collateral_amount_in_tokens,
            liquidation_bonus,
            timestamp,
        });
    }
    
    // ========================================
    // Interest Accrual
    // ========================================
    
    /// Accrue interest on all borrows
    fn accrue_interest(&mut self) {
        // In a full implementation, this would update all positions
        // For simplicity, we update on a per-user basis when they interact
        let timestamp = self.env().get_block_time();
        self.env().emit_event(InterestAccrued {
            interest_amount: U256::zero(), // Calculated per user
            total_borrows: self.total_borrows.get_or_default(),
            timestamp,
        });
    }
    
    /// Update interest rates based on utilization
    fn update_interest_rates(&mut self) {
        let total_borrows = self.total_borrows.get_or_default();
        let total_liquidity = self.total_liquidity.get_or_default();
        
        let strategy_address = self.interest_rate_strategy.get_or_revert_with(LendingError::InvalidConfiguration);
        let strategy = InterestRateStrategyContractRef::new(self.env(), strategy_address);
        
        let borrow_rate = strategy.calculate_borrow_rate(total_borrows, total_liquidity);
        let reserve_factor = self.reserve_factor.get_or_default();
        let supply_rate = strategy.calculate_supply_rate(borrow_rate, total_borrows, total_liquidity, reserve_factor);
        
        self.borrow_rate.set(borrow_rate);
        self.supply_rate.set(supply_rate);
        
        let utilization_rate = strategy.calculate_utilization_rate(total_borrows, total_liquidity);
        
        let timestamp = self.env().get_block_time();
        self.env().emit_event(InterestRatesUpdated {
            borrow_rate,
            supply_rate,
            utilization_rate,
            timestamp,
        });
    }
    
    // ========================================
    // View Functions
    // ========================================
    
    pub fn get_borrow_position(&self, user: Address) -> Option<BorrowPosition> {
        self.borrow_positions.get(&user)
    }
    
    pub fn get_total_borrows(&self) -> U256 {
        self.total_borrows.get_or_default()
    }
    
    pub fn get_total_liquidity(&self) -> U256 {
        self.total_liquidity.get_or_default()
    }
    
    pub fn get_borrow_rate(&self) -> U256 {
        self.borrow_rate.get_or_default()
    }
    
    pub fn get_supply_rate(&self) -> U256 {
        self.supply_rate.get_or_default()
    }
    
    pub fn get_utilization_rate(&self) -> U256 {
        let total_borrows = self.total_borrows.get_or_default();
        let total_liquidity = self.total_liquidity.get_or_default();
        
        if total_borrows == U256::zero() {
            return U256::zero();
        }
        
        let total = total_borrows + total_liquidity;
        if total == U256::zero() {
            return U256::zero();
        }
        
        let scale = U256::from(1_000_000_000_000_000_000u128); // 1e18
        (total_borrows * scale) / total
    }
    
    // ========================================
    // Admin Functions
    // ========================================
    
    pub fn pause(&mut self) {
        self.only_admin();
        self.paused.set(true);
        
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        let timestamp = self.env().get_block_time();
        self.env().emit_event(ContractPaused {
            paused_by: admin,
            timestamp,
        });
    }
    
    pub fn unpause(&mut self) {
        self.only_admin();
        self.paused.set(false);
        
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        let timestamp = self.env().get_block_time();
        self.env().emit_event(ContractUnpaused {
            unpaused_by: admin,
            timestamp,
        });
    }
    
    pub fn set_reserve_factor(&mut self, new_factor: U256) {
        self.only_admin();
        
        let scale = U256::from(1_000_000_000_000_000_000u128); // 1e18
        if new_factor > scale {
            self.env().revert(LendingError::InvalidConfiguration);
        }
        
        let old_factor = self.reserve_factor.get_or_default();
        self.reserve_factor.set(new_factor);
        
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        self.env().emit_event(ReserveFactorUpdated {
            old_factor,
            new_factor,
            updated_by: admin,
        });
    }
    
    fn only_admin(&self) {
        let caller = self.env().caller();
        let admin = self.admin.get_or_revert_with(LendingError::Unauthorized);
        if caller != admin {
            self.env().revert(LendingError::Unauthorized);
        }
    }
    
    fn ensure_not_paused(&self) {
        if self.paused.get_or_default() {
            self.env().revert(LendingError::ContractPaused);
        }
    }
}
