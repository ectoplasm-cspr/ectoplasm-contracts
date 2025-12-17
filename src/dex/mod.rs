//! DEX module containing all DEX-related contracts
//!
//! This module implements a Uniswap V2-style AMM DEX with:
//! - Pair: Individual liquidity pools for token pairs
//! - Factory: Creates and manages pairs
//! - Router: User-facing contract for swaps and liquidity management

pub mod pair;
pub mod factory;
pub mod router;

#[cfg(test)]
pub mod tests;

pub use pair::Pair;
pub use factory::Factory;
pub use router::Router;