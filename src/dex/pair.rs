//! Liquidity Pair contract for the DEX
//! 
//! Each Pair holds reserves of two tokens and allows:
//! - Adding liquidity (minting LP tokens)
//! - Removing liquidity (burning LP tokens)
//! - Swapping tokens
use odra::prelude::*;
use odra::casper_types::U256;
use odra::{Address, Mapping, SubModule, Var};
use crate::errors::DexError;
use crate::events::{LiquidityAdded, LiquidityRemoved, Swap, Sync};
use crate::math::{AmmMath, SafeMath, MINIMUM_LIQUIDITY};
use crate::token::{LpToken, Cep18TokenContractRef};

/// Liquidity Pair contract
#[odra::module]
pub struct Pair {
    /// LP token for this pair
    lp_token: SubModule<LpToken>,
    /// Address of token0
    token0: Var<Address>,
    /// Address of token1
    token1: Var<Address>,
    /// Reserve of token0
    reserve0: Var<U256>,
    /// Reserve of token1
    reserve1: Var<U256>,
    /// Block timestamp of last update
    block_timestamp_last: Var<u64>,
    /// Cumulative price of token0
    price0_cumulative_last: Var<U256>,
    /// Cumulative price of token1
    price1_cumulative_last: Var<U256>,
    /// K value from last liquidity event (for fee calculation)
    k_last: Var<U256>,
    /// Factory address
    factory: Var<Address>,
    /// Reentrancy lock
    locked: Var<bool>,
}

#[odra::module]
impl Pair {
    /// Initialize the pair with two token addresses
    pub fn init(
        &mut self,
        token0: Address,
        token1: Address,
        factory: Address,
    ) {
        // Ensure tokens are ordered
        let (t0, t1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };

        self.token0.set(t0);
        self.token1.set(t1);
        self.factory.set(factory);
        self.reserve0.set(U256::zero());
        self.reserve1.set(U256::zero());
        self.locked.set(false);

        // Initialize LP token
        let name = String::from("DEX LP Token");
        let symbol = String::from("DEX-LP");
        self.lp_token.init(name, symbol);
    }

    /// Get token0 address
    pub fn token0(&self) -> Address {
        self.token0.get_or_revert()
    }

    /// Get token1 address
    pub fn token1(&self) -> Address {
        self.token1.get_or_revert()
    }

    /// Get current reserves
    pub fn get_reserves(&self) -> (U256, U256, u64) {
        (
            self.reserve0.get_or_default(),
            self.reserve1.get_or_default(),
            self.block_timestamp_last.get_or_default(),
        )
    }

    /// Get LP token total supply
    pub fn total_supply(&self) -> U256 {
        self.lp_token.total_supply()
    }

    /// Get LP token balance of an address
    pub fn balance_of(&self, owner: Address) -> U256 {
        self.lp_token.balance_of(owner)
    }

    /// Transfer LP tokens
    pub fn transfer(&mut self, to: Address, amount: U256) -> bool {
        self.lp_token.transfer(to, amount)
    }

    /// Approve LP token spending
    pub fn approve(&mut self, spender: Address, amount: U256) -> bool {
        self.lp_token.approve(spender, amount)
    }

    /// Transfer LP tokens from another address
    pub fn transfer_from(&mut self, from: Address, to: Address, amount: U256) -> bool {
        self.lp_token.transfer_from(from, to, amount)
    }

    /// Mint LP tokens when liquidity is added
    /// Returns the amount of LP tokens minted
    pub fn mint(&mut self, to: Address) -> Result<U256, DexError> {
        self.lock()?;

        let (reserve0, reserve1, _) = self.get_reserves();
        
        // Get current balances
        let balance0 = self.get_token_balance(self.token0());
        let balance1 = self.get_token_balance(self.token1());

        // Calculate amounts deposited
        let amount0 = SafeMath::sub(balance0, reserve0)?;
        let amount1 = SafeMath::sub(balance1, reserve1)?;

        let total_supply = self.total_supply();
        let liquidity: U256;

        if total_supply.is_zero() {
            // First liquidity provision
            liquidity = AmmMath::calculate_liquidity(
                amount0, amount1, reserve0, reserve1, total_supply
            )?;
            
            // Permanently lock MINIMUM_LIQUIDITY tokens
            self.lp_token.mint(
                Address::from(self.env().self_address()),
                U256::from(MINIMUM_LIQUIDITY),
            );
        } else {
            liquidity = AmmMath::calculate_liquidity(
                amount0, amount1, reserve0, reserve1, total_supply
            )?;
        }

        if liquidity.is_zero() {
            return Err(DexError::InsufficientLiquidityMinted);
        }

        self.lp_token.mint(to, liquidity);

        // Update reserves
        self.update_reserves(balance0, balance1)?;

        // Update k_last for fee calculation
        let (new_reserve0, new_reserve1, _) = self.get_reserves();
        self.k_last.set(SafeMath::mul(new_reserve0, new_reserve1)?);

        self.env().emit_event(LiquidityAdded {
            provider: to,
            pair: self.env().self_address(),
            amount0,
            amount1,
            liquidity,
        });

        self.unlock();
        Ok(liquidity)
    }

    /// Burn LP tokens when liquidity is removed
    /// Returns the amounts of token0 and token1 returned
    pub fn burn(&mut self, to: Address) -> Result<(U256, U256), DexError> {
        self.lock()?;

        let (reserve0, reserve1, _) = self.get_reserves();
        let token0 = self.token0();
        let token1 = self.token1();

        // Get current balances
        let balance0 = self.get_token_balance(token0);
        let balance1 = self.get_token_balance(token1);

        // Get LP tokens sent to this contract
        let liquidity = self.lp_token.balance_of(self.env().self_address());
        let total_supply = self.total_supply();

        // Calculate amounts to return
        let (amount0, amount1) = AmmMath::calculate_burn_amounts(
            liquidity, reserve0, reserve1, total_supply
        )?;

        // Burn LP tokens
        self.lp_token.burn(self.env().self_address(), liquidity);

        // Transfer tokens to user
        self.safe_transfer(token0, to, amount0)?;
        self.safe_transfer(token1, to, amount1)?;

        // Update reserves
        let new_balance0 = SafeMath::sub(balance0, amount0)?;
        let new_balance1 = SafeMath::sub(balance1, amount1)?;
        self.update_reserves(new_balance0, new_balance1)?;

        self.env().emit_event(LiquidityRemoved {
            provider: to,
            pair: self.env().self_address(),
            amount0,
            amount1,
            liquidity,
        });

        self.unlock();
        Ok((amount0, amount1))
    }

    /// Swap tokens
    /// amount0_out and amount1_out are the amounts to send out
    /// One of them should be zero
    pub fn swap(
        &mut self,
        amount0_out: U256,
        amount1_out: U256,
        to: Address,
    ) -> Result<(), DexError> {
        self.lock()?;

        if amount0_out.is_zero() && amount1_out.is_zero() {
            return Err(DexError::InsufficientOutputAmount);
        }

        let (reserve0, reserve1, _) = self.get_reserves();

        if amount0_out >= reserve0 || amount1_out >= reserve1 {
            return Err(DexError::InsufficientLiquidity);
        }

        let token0 = self.token0();
        let token1 = self.token1();

        // Ensure recipient is not one of the tokens
        if to == token0 || to == token1 {
            return Err(DexError::InvalidPair);
        }

        // Transfer tokens out
        if !amount0_out.is_zero() {
            self.safe_transfer(token0, to, amount0_out)?;
        }
        if !amount1_out.is_zero() {
            self.safe_transfer(token1, to, amount1_out)?;
        }

        // Get new balances
        let balance0 = self.get_token_balance(token0);
        let balance1 = self.get_token_balance(token1);

        // Calculate amounts in
        let amount0_in = if balance0 > SafeMath::sub(reserve0, amount0_out)? {
            SafeMath::sub(balance0, SafeMath::sub(reserve0, amount0_out)?)?
        } else {
            U256::zero()
        };
        let amount1_in = if balance1 > SafeMath::sub(reserve1, amount1_out)? {
            SafeMath::sub(balance1, SafeMath::sub(reserve1, amount1_out)?)?
        } else {
            U256::zero()
        };

        if amount0_in.is_zero() && amount1_in.is_zero() {
            return Err(DexError::InsufficientInputAmount);
        }

        // Verify K invariant (with fee adjustment)
        // balance0_adjusted = balance0 * 1000 - amount0_in * 3
        // balance1_adjusted = balance1 * 1000 - amount1_in * 3
        // balance0_adjusted * balance1_adjusted >= reserve0 * reserve1 * 1000^2
        let balance0_adjusted = SafeMath::sub(
            SafeMath::mul(balance0, U256::from(1000))?,
            SafeMath::mul(amount0_in, U256::from(3))?,
        )?;
        let balance1_adjusted = SafeMath::sub(
            SafeMath::mul(balance1, U256::from(1000))?,
            SafeMath::mul(amount1_in, U256::from(3))?,
        )?;

        let k_new = SafeMath::mul(balance0_adjusted, balance1_adjusted)?;
        let k_old = SafeMath::mul(
            SafeMath::mul(reserve0, reserve1)?,
            U256::from(1000000),
        )?;

        if k_new < k_old {
            return Err(DexError::KInvariantViolated);
        }

        // Update reserves
        self.update_reserves(balance0, balance1)?;

        self.env().emit_event(Swap {
            sender: self.env().caller(),
            pair: self.env().self_address(),
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            to,
        });

        self.unlock();
        Ok(())
    }

    /// Force reserves to match balances (for recovery)
    pub fn skim(&mut self, to: Address) -> Result<(), DexError> {
        let token0 = self.token0();
        let token1 = self.token1();
        let (reserve0, reserve1, _) = self.get_reserves();

        let balance0 = self.get_token_balance(token0);
        let balance1 = self.get_token_balance(token1);

        if balance0 > reserve0 {
            self.safe_transfer(token0, to, SafeMath::sub(balance0, reserve0)?)?;
        }
        if balance1 > reserve1 {
            self.safe_transfer(token1, to, SafeMath::sub(balance1, reserve1)?)?;
        }

        Ok(())
    }

    /// Force balances to match reserves (for recovery)
    pub fn sync(&mut self) -> Result<(), DexError> {
        let token0 = self.token0();
        let token1 = self.token1();

        let balance0 = self.get_token_balance(token0);
        let balance1 = self.get_token_balance(token1);

        self.update_reserves(balance0, balance1)
    }

    /// Get the price of token0 in terms of token1
    pub fn get_price0(&self) -> Result<U256, DexError> {
        let (reserve0, reserve1, _) = self.get_reserves();
        if reserve0.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }
        // Price with 18 decimal precision
        SafeMath::div(
            SafeMath::mul(reserve1, U256::from(10u128.pow(18)))?,
            reserve0,
        )
    }

    /// Get the price of token1 in terms of token0
    pub fn get_price1(&self) -> Result<U256, DexError> {
        let (reserve0, reserve1, _) = self.get_reserves();
        if reserve1.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }
        // Price with 18 decimal precision
        SafeMath::div(
            SafeMath::mul(reserve0, U256::from(10u128.pow(18)))?,
            reserve1,
        )
    }

    // ============ Internal Functions ============

    /// Update reserves and emit Sync event
    fn update_reserves(&mut self, balance0: U256, balance1: U256) -> Result<(), DexError> {
        self.reserve0.set(balance0);
        self.reserve1.set(balance1);
        self.block_timestamp_last.set(self.env().get_block_time());

        self.env().emit_event(Sync {
            pair: self.env().self_address(),
            reserve0: balance0,
            reserve1: balance1,
        });

        Ok(())
    }

    /// Get token balance of this contract
    fn get_token_balance(&self, token: Address) -> U256 {
        let token_ref = Cep18TokenContractRef::new(self.env(), token);
        token_ref.balance_of(self.env().self_address())
    }

    /// Safe transfer tokens
    fn safe_transfer(&self, token: Address, to: Address, amount: U256) -> Result<(), DexError> {
        let mut token_ref = Cep18TokenContractRef::new(self.env(), token);
        let success = token_ref.transfer(to, amount);
        if !success {
            return Err(DexError::TransferFailed);
        }
        Ok(())
    }

    /// Reentrancy lock
    fn lock(&mut self) -> Result<(), DexError> {
        if self.locked.get_or_default() {
            return Err(DexError::Locked);
        }
        self.locked.set(true);
        Ok(())
    }

    /// Reentrancy unlock
    fn unlock(&mut self) {
        self.locked.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    #[test]
    fn test_pair_init() {
        let env = odra_test::env();
        let token0 = env.get_account(1);
        let token1 = env.get_account(2);
        let factory = env.get_account(0);

        let init_args = PairInitArgs {
            token0,
            token1,
            factory,
        };
        let pair = Pair::deploy(&env, init_args);

        // Tokens should be sorted
        let (t0, t1) = if token0 < token1 {
            (token0, token1)
        } else {
            (token1, token0)
        };

        assert_eq!(pair.token0(), t0);
        assert_eq!(pair.token1(), t1);
        
        let (reserve0, reserve1, _) = pair.get_reserves();
        assert_eq!(reserve0, U256::zero());
        assert_eq!(reserve1, U256::zero());
    }
}