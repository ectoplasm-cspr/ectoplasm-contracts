//! Error definitions for the DEX smart contract
use odra::prelude::*;
use odra::OdraError;

/// Custom errors for the DEX contract
#[derive(OdraError, Debug, PartialEq, Eq)]
pub enum DexError {
    /// Insufficient liquidity in the pool
    #[odra_error(code = 1)]
    InsufficientLiquidity,
    
    /// Insufficient input amount for swap
    #[odra_error(code = 2)]
    InsufficientInputAmount,
    
    /// Insufficient output amount for swap
    #[odra_error(code = 3)]
    InsufficientOutputAmount,
    
    /// Invalid token pair
    #[odra_error(code = 4)]
    InvalidPair,
    
    /// Pair already exists
    #[odra_error(code = 5)]
    PairExists,
    
    /// Pair does not exist
    #[odra_error(code = 6)]
    PairNotFound,
    
    /// Zero address provided
    #[odra_error(code = 7)]
    ZeroAddress,
    
    /// Identical addresses provided
    #[odra_error(code = 8)]
    IdenticalAddresses,
    
    /// Insufficient amount
    #[odra_error(code = 9)]
    InsufficientAmount,
    
    /// Transfer failed
    #[odra_error(code = 10)]
    TransferFailed,
    
    /// Deadline expired
    #[odra_error(code = 11)]
    DeadlineExpired,
    
    /// Slippage too high
    #[odra_error(code = 12)]
    ExcessiveSlippage,
    
    /// Overflow error
    #[odra_error(code = 13)]
    Overflow,
    
    /// Underflow error
    #[odra_error(code = 14)]
    Underflow,
    
    /// Division by zero
    #[odra_error(code = 15)]
    DivisionByZero,
    
    /// Unauthorized access
    #[odra_error(code = 16)]
    Unauthorized,
    
    /// Invalid path for swap
    #[odra_error(code = 17)]
    InvalidPath,
    
    /// K value invariant violated
    #[odra_error(code = 18)]
    KInvariantViolated,
    
    /// Insufficient liquidity minted
    #[odra_error(code = 19)]
    InsufficientLiquidityMinted,
    
    /// Insufficient liquidity burned
    #[odra_error(code = 20)]
    InsufficientLiquidityBurned,
    
    /// Locked - reentrancy guard
    #[odra_error(code = 21)]
    Locked,
    
    /// Invalid fee
    #[odra_error(code = 22)]
    InvalidFee,
}