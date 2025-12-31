//! Error definitions for the DEX smart contract
use odra::prelude::*;

/// Custom errors for the DEX contract
#[odra::odra_error]
pub enum DexError {
    /// Insufficient liquidity in the pool
    InsufficientLiquidity = 1,
    
    /// Insufficient input amount for swap
    InsufficientInputAmount = 2,
    
    /// Insufficient output amount for swap
    InsufficientOutputAmount = 3,
    
    /// Invalid token pair
    InvalidPair = 4,
    
    /// Pair already exists
    PairExists = 5,
    
    /// Pair does not exist
    PairNotFound = 6,
    
    /// Zero address provided
    ZeroAddress = 7,
    
    /// Identical addresses provided
    IdenticalAddresses = 8,
    
    /// Insufficient amount
    InsufficientAmount = 9,
    
    /// Transfer failed
    TransferFailed = 10,
    
    /// Deadline expired
    DeadlineExpired = 11,
    
    /// Slippage too high
    ExcessiveSlippage = 12,
    
    /// Overflow error
    Overflow = 13,
    
    /// Underflow error
    Underflow = 14,
    
    /// Division by zero
    DivisionByZero = 15,
    
    /// Unauthorized access
    Unauthorized = 16,
    
    /// Invalid path for swap
    InvalidPath = 17,
    
    /// K value invariant violated
    KInvariantViolated = 18,
    
    /// Insufficient liquidity minted
    InsufficientLiquidityMinted = 19,
    
    /// Insufficient liquidity burned
    InsufficientLiquidityBurned = 20,
    
    /// Locked - reentrancy guard
    Locked = 21,
    
    /// Invalid fee
    InvalidFee = 22,
    
    /// Invalid configuration
    InvalidConfiguration = 23,
}

/// Custom errors for the LP Token contract
#[odra::odra_error]
pub enum TokenError {
    /// Insufficient allowance for transfer
    InsufficientAllowance = 100,
    
    /// Insufficient balance for operation
    InsufficientBalance = 101,
}