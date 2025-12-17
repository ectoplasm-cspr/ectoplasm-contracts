//! Event definitions for the DEX smart contract
use odra::prelude::*;
use odra::casper_types::U256;
use odra::Address;

/// Event emitted when a new pair is created
#[odra::event]
pub struct PairCreated {
    /// First token address
    pub token0: Address,
    /// Second token address
    pub token1: Address,
    /// Address of the created pair
    pub pair: Address,
    /// Total number of pairs
    pub pair_count: u32,
}

/// Event emitted when liquidity is added to a pool
#[odra::event]
pub struct LiquidityAdded {
    /// Address of the liquidity provider
    pub provider: Address,
    /// Address of the pair
    pub pair: Address,
    /// Amount of token0 added
    pub amount0: U256,
    /// Amount of token1 added
    pub amount1: U256,
    /// LP tokens minted
    pub liquidity: U256,
}

/// Event emitted when liquidity is removed from a pool
#[odra::event]
pub struct LiquidityRemoved {
    /// Address of the liquidity provider
    pub provider: Address,
    /// Address of the pair
    pub pair: Address,
    /// Amount of token0 removed
    pub amount0: U256,
    /// Amount of token1 removed
    pub amount1: U256,
    /// LP tokens burned
    pub liquidity: U256,
}

/// Event emitted when a swap occurs
#[odra::event]
pub struct Swap {
    /// Address of the sender
    pub sender: Address,
    /// Address of the pair
    pub pair: Address,
    /// Amount of token0 in
    pub amount0_in: U256,
    /// Amount of token1 in
    pub amount1_in: U256,
    /// Amount of token0 out
    pub amount0_out: U256,
    /// Amount of token1 out
    pub amount1_out: U256,
    /// Address receiving the output
    pub to: Address,
}

/// Event emitted when reserves are synced
#[odra::event]
pub struct Sync {
    /// Address of the pair
    pub pair: Address,
    /// Reserve of token0
    pub reserve0: U256,
    /// Reserve of token1
    pub reserve1: U256,
}

/// Event emitted when LP tokens are transferred
#[odra::event]
pub struct Transfer {
    /// From address
    pub from: Address,
    /// To address
    pub to: Address,
    /// Amount transferred
    pub value: U256,
}

/// Event emitted when approval is granted
#[odra::event]
pub struct Approval {
    /// Owner address
    pub owner: Address,
    /// Spender address
    pub spender: Address,
    /// Amount approved
    pub value: U256,
}

/// Event emitted when fee is collected
#[odra::event]
pub struct FeeCollected {
    /// Address of the pair
    pub pair: Address,
    /// Fee recipient
    pub recipient: Address,
    /// Amount collected
    pub amount: U256,
}