//! CEP-4626: Tokenized Vault Standard for Casper
//! 
//! This is a Casper adaptation of ERC-4626, providing a standard API for
//! tokenized vaults representing shares of a single underlying CEP-18 token.
//! 
//! Use cases:
//! - Liquid staking (sCSPR vault for CSPR)
//! - Lending pools (aECTO vault for ECTO)
//! - Yield aggregators
//! - Interest-bearing tokens

pub mod vault;
pub mod events;

pub use vault::Cep4626Vault;
pub use events::*;
