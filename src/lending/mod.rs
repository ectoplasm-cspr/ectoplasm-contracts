//! Lending Protocol - Aave-like yield protocol for ECTO
//! 
//! This module provides a lending and borrowing protocol for ECTO stablecoin,
//! with support for multiple collateral types (sCSPR, WETH, WBTC, etc.).
//! 
//! **CEP-4626 Compliant**: The lending pool implements CEP-4626 for aECTO,
//! providing a standardized interface for interest-bearing deposits.

pub mod aecto_vault;
pub mod lending_pool;
pub mod interest_rate;
pub mod collateral_manager;
pub mod liquidation;
pub mod price_oracle;
pub mod errors;
pub mod events;

pub use aecto_vault::AectoVault;
pub use lending_pool::LendingPool;
pub use interest_rate::InterestRateStrategy;
pub use collateral_manager::CollateralManager;
pub use liquidation::LiquidationEngine;
pub use price_oracle::PriceOracle;
pub use errors::LendingError;
pub use events::*;
