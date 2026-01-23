//! Launchpad-specific error types

use odra::prelude::*;

/// Errors that can occur in the Launchpad contracts
#[odra::odra_error]
pub enum LaunchpadError {
    /// Caller is not authorized for this operation
    Unauthorized = 30_000,
    
    /// Launch has already graduated to DEX
    AlreadyGraduated = 30_001,
    
    /// Launch is in refunding state
    IsRefunding = 30_002,
    
    /// Launch is not active
    NotActive = 30_003,
    
    /// Slippage tolerance exceeded
    SlippageExceeded = 30_004,
    
    /// Insufficient CSPR sent
    InsufficientPayment = 30_005,
    
    /// Insufficient token balance
    InsufficientBalance = 30_006,
    
    /// Graduation threshold not met
    ThresholdNotMet = 30_007,
    
    /// Deadline not yet passed for refunds
    DeadlineNotPassed = 30_008,
    
    /// No refund available for caller
    NoRefundAvailable = 30_009,
    
    /// Invalid curve type
    InvalidCurveType = 30_010,
    
    /// Zero amount not allowed
    ZeroAmount = 30_011,
    
    /// Token transfer failed
    TransferFailed = 30_012,
    
    /// Launch not found
    LaunchNotFound = 30_013,
    
    /// DEX integration failed
    DexIntegrationFailed = 30_014,
}
