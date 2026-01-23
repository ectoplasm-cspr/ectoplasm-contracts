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

// Launchpad modules
pub mod launchpad;
