//! Error types for the Lending Protocol

use odra::prelude::*;

/// Errors that can occur in the lending protocol
#[odra::odra_error]
pub enum LendingError {
    // Deposit/Withdrawal Errors
    /// Insufficient balance for operation
    InsufficientBalance = 1,
    /// Amount is below minimum deposit
    BelowMinimumDeposit = 2,
    /// Amount exceeds maximum deposit
    ExceedsMaximumDeposit = 3,
    /// Insufficient liquidity for withdrawal
    InsufficientLiquidity = 4,
    
    // Borrowing Errors
    /// Insufficient collateral to borrow
    InsufficientCollateral = 5,
    /// Amount is below minimum borrow
    BelowMinimumBorrow = 6,
    /// Amount exceeds maximum borrow
    ExceedsMaximumBorrow = 7,
    /// Borrow would exceed collateral limit
    ExceedsBorrowLimit = 8,
    /// User has no active borrow
    NoBorrowPosition = 9,
    
    // Collateral Errors
    /// Collateral type not supported
    UnsupportedCollateral = 10,
    /// Insufficient collateral deposited
    InsufficientCollateralDeposit = 11,
    /// Cannot withdraw collateral (would be undercollateralized)
    CannotWithdrawCollateral = 12,
    /// Collateral is disabled
    CollateralDisabled = 13,
    
    // Health Factor Errors
    /// Health factor below liquidation threshold
    HealthFactorBelowThreshold = 14,
    /// Position is healthy, cannot liquidate
    PositionHealthy = 15,
    /// Health factor too low to borrow more
    HealthFactorTooLow = 16,
    
    // Liquidation Errors
    /// Liquidation amount exceeds debt
    ExceedsDebtAmount = 17,
    /// Liquidation bonus calculation failed
    LiquidationBonusFailed = 18,
    /// Insufficient collateral to cover liquidation
    InsufficientCollateralForLiquidation = 19,
    
    // Interest Rate Errors
    /// Invalid interest rate parameters
    InvalidInterestRateParams = 20,
    /// Utilization rate calculation failed
    UtilizationCalculationFailed = 21,
    
    // Price Oracle Errors
    /// Price feed not available
    PriceFeedNotAvailable = 22,
    /// Price is stale or invalid
    InvalidPrice = 23,
    /// Price oracle not initialized
    OracleNotInitialized = 24,
    
    // Access Control Errors
    /// Caller is not authorized
    Unauthorized = 25,
    /// Contract is paused
    ContractPaused = 26,
    /// Operation not allowed
    OperationNotAllowed = 27,
    
    // Configuration Errors
    /// Invalid configuration parameter
    InvalidConfiguration = 28,
    /// Reserve not initialized
    ReserveNotInitialized = 29,
    /// Reserve already initialized
    ReserveAlreadyInitialized = 30,
    
    // General Errors
    /// Zero amount not allowed
    ZeroAmount = 31,
    /// Invalid address provided
    InvalidAddress = 32,
    /// Math overflow occurred
    MathOverflow = 33,
    /// Math underflow occurred
    MathUnderflow = 34,
    /// Division by zero
    DivisionByZero = 35,
}
