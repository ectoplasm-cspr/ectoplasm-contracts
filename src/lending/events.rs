//! Events for the Lending Protocol

use odra::prelude::*;
use odra::casper_types::U256;

// ============================================================================
// Deposit/Withdrawal Events
// ============================================================================

/// Event emitted when ECTO is deposited into the lending pool
#[odra::event]
pub struct Deposited {
    /// Address that deposited
    pub user: Address,
    /// Amount of ECTO deposited
    pub amount: U256,
    /// Amount of aECTO minted
    pub shares: U256,
    /// Timestamp of deposit
    pub timestamp: u64,
}

/// Event emitted when ECTO is withdrawn from the lending pool
#[odra::event]
pub struct Withdrawn {
    /// Address that withdrew
    pub user: Address,
    /// Amount of ECTO withdrawn
    pub amount: U256,
    /// Amount of aECTO burned
    pub shares: U256,
    /// Timestamp of withdrawal
    pub timestamp: u64,
}

// ============================================================================
// Borrowing Events
// ============================================================================

/// Event emitted when ECTO is borrowed
#[odra::event]
pub struct Borrowed {
    /// Address that borrowed
    pub borrower: Address,
    /// Amount of ECTO borrowed
    pub amount: U256,
    /// Collateral type used
    pub collateral_asset: Address,
    /// Interest rate at time of borrow
    pub borrow_rate: U256,
    /// Timestamp of borrow
    pub timestamp: u64,
}

/// Event emitted when borrowed ECTO is repaid
#[odra::event]
pub struct Repaid {
    /// Address that repaid
    pub borrower: Address,
    /// Amount of ECTO repaid
    pub amount: U256,
    /// Interest paid
    pub interest: U256,
    /// Timestamp of repayment
    pub timestamp: u64,
}

// ============================================================================
// Collateral Events
// ============================================================================

/// Event emitted when collateral is deposited
#[odra::event]
pub struct CollateralDeposited {
    /// Address that deposited collateral
    pub user: Address,
    /// Collateral asset address
    pub asset: Address,
    /// Amount of collateral deposited
    pub amount: U256,
    /// Timestamp of deposit
    pub timestamp: u64,
}

/// Event emitted when collateral is withdrawn
#[odra::event]
pub struct CollateralWithdrawn {
    /// Address that withdrew collateral
    pub user: Address,
    /// Collateral asset address
    pub asset: Address,
    /// Amount of collateral withdrawn
    pub amount: U256,
    /// Timestamp of withdrawal
    pub timestamp: u64,
}

// ============================================================================
// Liquidation Events
// ============================================================================

/// Event emitted when a position is liquidated
#[odra::event]
pub struct Liquidated {
    /// Address of the borrower being liquidated
    pub borrower: Address,
    /// Address of the liquidator
    pub liquidator: Address,
    /// Collateral asset being liquidated
    pub collateral_asset: Address,
    /// Amount of debt repaid
    pub debt_covered: U256,
    /// Amount of collateral seized
    pub collateral_seized: U256,
    /// Liquidation bonus paid to liquidator
    pub liquidation_bonus: U256,
    /// Timestamp of liquidation
    pub timestamp: u64,
}

// ============================================================================
// Interest Rate Events
// ============================================================================

/// Event emitted when interest rates are updated
#[odra::event]
pub struct InterestRatesUpdated {
    /// New borrow rate
    pub borrow_rate: U256,
    /// New supply rate (deposit APY)
    pub supply_rate: U256,
    /// Current utilization rate
    pub utilization_rate: U256,
    /// Timestamp of update
    pub timestamp: u64,
}

/// Event emitted when interest is accrued
#[odra::event]
pub struct InterestAccrued {
    /// Total interest accrued
    pub interest_amount: U256,
    /// New total borrows
    pub total_borrows: U256,
    /// Timestamp of accrual
    pub timestamp: u64,
}

// ============================================================================
// Configuration Events
// ============================================================================

/// Event emitted when a new collateral type is added
#[odra::event]
pub struct CollateralAdded {
    /// Collateral asset address
    pub asset: Address,
    /// Loan-to-value ratio (scaled by 1e18)
    pub ltv: U256,
    /// Liquidation threshold (scaled by 1e18)
    pub liquidation_threshold: U256,
    /// Liquidation bonus (scaled by 1e18)
    pub liquidation_bonus: U256,
    /// Added by
    pub added_by: Address,
}

/// Event emitted when collateral parameters are updated
#[odra::event]
pub struct CollateralUpdated {
    /// Collateral asset address
    pub asset: Address,
    /// New LTV
    pub ltv: U256,
    /// New liquidation threshold
    pub liquidation_threshold: U256,
    /// New liquidation bonus
    pub liquidation_bonus: U256,
    /// Updated by
    pub updated_by: Address,
}

/// Event emitted when interest rate parameters are updated
#[odra::event]
pub struct InterestRateParamsUpdated {
    /// Base rate (scaled by 1e18)
    pub base_rate: U256,
    /// Optimal utilization rate (scaled by 1e18)
    pub optimal_utilization: U256,
    /// Slope 1 (rate increase before optimal)
    pub slope1: U256,
    /// Slope 2 (rate increase after optimal)
    pub slope2: U256,
    /// Updated by
    pub updated_by: Address,
}

// ============================================================================
// Admin Events
// ============================================================================

/// Event emitted when contract is paused
#[odra::event]
pub struct ContractPaused {
    /// Address that paused
    pub paused_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when contract is unpaused
#[odra::event]
pub struct ContractUnpaused {
    /// Address that unpaused
    pub unpaused_by: Address,
    /// Timestamp
    pub timestamp: u64,
}

/// Event emitted when reserve factor is updated
#[odra::event]
pub struct ReserveFactorUpdated {
    /// Old reserve factor
    pub old_factor: U256,
    /// New reserve factor
    pub new_factor: U256,
    /// Updated by
    pub updated_by: Address,
}
