//! Liquid Staking Token (LST) module
//! 
//! This module provides liquid staking functionality for Casper Network,
//! allowing users to stake CSPR and receive sCSPR tokens that represent
//! their staked position while remaining liquid and composable.

pub mod scspr_token;
pub mod staking_manager;
pub mod errors;
pub mod events;

#[cfg(test)]
// mod tests;
mod working_tests;

pub use scspr_token::ScsprToken;
pub use staking_manager::StakingManager;
pub use errors::LstError;
pub use events::*;
