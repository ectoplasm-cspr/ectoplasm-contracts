#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
extern crate alloc;

// Core modules
pub mod flipper;

// DEX modules
pub mod dex;
pub mod token;
pub mod tokens;
pub mod errors;
pub mod events;
pub mod math;

// CEP-4626: Tokenized Vault Standard
pub mod cep4626;

// LST (Liquid Staking Token) modules
pub mod lst;

// Lending Protocol modules
pub mod lending;

// Yield Farming modules
pub mod farming;

// Incentive System modules
pub mod incentives;
