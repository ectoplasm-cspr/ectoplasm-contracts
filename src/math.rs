//! Mathematical utilities for the DEX smart contract
//! Implements safe math operations and AMM formulas
use odra::prelude::*;
use odra::casper_types::U256;
use crate::errors::DexError;

/// Minimum liquidity that is locked forever to prevent division by zero
pub const MINIMUM_LIQUIDITY: u128 = 1000;

/// Fee denominator (0.3% fee = 3/1000)
pub const FEE_DENOMINATOR: u128 = 1000;

/// Fee numerator (0.3% fee)
pub const FEE_NUMERATOR: u128 = 3;

/// Safe math operations for U256
pub struct SafeMath;

impl SafeMath {
    /// Safe addition with overflow check
    pub fn add(a: U256, b: U256) -> Result<U256, DexError> {
        a.checked_add(b).ok_or(DexError::Overflow)
    }

    /// Safe subtraction with underflow check
    pub fn sub(a: U256, b: U256) -> Result<U256, DexError> {
        a.checked_sub(b).ok_or(DexError::Underflow)
    }

    /// Safe multiplication with overflow check
    pub fn mul(a: U256, b: U256) -> Result<U256, DexError> {
        a.checked_mul(b).ok_or(DexError::Overflow)
    }

    /// Safe division with zero check
    pub fn div(a: U256, b: U256) -> Result<U256, DexError> {
        if b.is_zero() {
            return Err(DexError::DivisionByZero);
        }
        Ok(a / b)
    }

    /// Calculate square root using Newton's method (Babylonian method)
    pub fn sqrt(y: U256) -> U256 {
        if y > U256::from(3) {
            let mut z = y;
            let mut x = y / 2 + 1;
            while x < z {
                z = x;
                x = (y / x + x) / 2;
            }
            z
        } else if !y.is_zero() {
            U256::one()
        } else {
            U256::zero()
        }
    }

    /// Returns the minimum of two U256 values
    pub fn min(a: U256, b: U256) -> U256 {
        if a < b { a } else { b }
    }

    /// Returns the maximum of two U256 values
    pub fn max(a: U256, b: U256) -> U256 {
        if a > b { a } else { b }
    }
}

/// AMM (Automated Market Maker) calculations
pub struct AmmMath;

impl AmmMath {
    /// Calculate the amount of output tokens for a given input amount
    /// Uses the constant product formula: x * y = k
    /// With 0.3% fee: amount_out = (amount_in * 997 * reserve_out) / (reserve_in * 1000 + amount_in * 997)
    pub fn get_amount_out(
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, DexError> {
        if amount_in.is_zero() {
            return Err(DexError::InsufficientInputAmount);
        }
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }

        let amount_in_with_fee = SafeMath::mul(
            amount_in,
            U256::from(FEE_DENOMINATOR - FEE_NUMERATOR),
        )?;
        let numerator = SafeMath::mul(amount_in_with_fee, reserve_out)?;
        let denominator = SafeMath::add(
            SafeMath::mul(reserve_in, U256::from(FEE_DENOMINATOR))?,
            amount_in_with_fee,
        )?;

        SafeMath::div(numerator, denominator)
    }

    /// Calculate the amount of input tokens required for a given output amount
    /// amount_in = (reserve_in * amount_out * 1000) / ((reserve_out - amount_out) * 997) + 1
    pub fn get_amount_in(
        amount_out: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, DexError> {
        if amount_out.is_zero() {
            return Err(DexError::InsufficientOutputAmount);
        }
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }
        if amount_out >= reserve_out {
            return Err(DexError::InsufficientLiquidity);
        }

        let numerator = SafeMath::mul(
            SafeMath::mul(reserve_in, amount_out)?,
            U256::from(FEE_DENOMINATOR),
        )?;
        let denominator = SafeMath::mul(
            SafeMath::sub(reserve_out, amount_out)?,
            U256::from(FEE_DENOMINATOR - FEE_NUMERATOR),
        )?;

        SafeMath::add(SafeMath::div(numerator, denominator)?, U256::one())
    }

    /// Calculate the optimal amount of token B given an amount of token A
    /// Used when adding liquidity to maintain the price ratio
    /// amount_b = amount_a * reserve_b / reserve_a
    pub fn quote(
        amount_a: U256,
        reserve_a: U256,
        reserve_b: U256,
    ) -> Result<U256, DexError> {
        if amount_a.is_zero() {
            return Err(DexError::InsufficientAmount);
        }
        if reserve_a.is_zero() || reserve_b.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }

        SafeMath::div(SafeMath::mul(amount_a, reserve_b)?, reserve_a)
    }

    /// Calculate the amount of liquidity tokens to mint
    /// For first deposit: liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
    /// For subsequent deposits: liquidity = min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
    pub fn calculate_liquidity(
        amount0: U256,
        amount1: U256,
        reserve0: U256,
        reserve1: U256,
        total_supply: U256,
    ) -> Result<U256, DexError> {
        if total_supply.is_zero() {
            // First liquidity provision
            let liquidity = SafeMath::sqrt(SafeMath::mul(amount0, amount1)?);
            let min_liquidity = U256::from(MINIMUM_LIQUIDITY);
            
            if liquidity <= min_liquidity {
                return Err(DexError::InsufficientLiquidityMinted);
            }
            
            SafeMath::sub(liquidity, min_liquidity)
        } else {
            // Subsequent liquidity provision
            let liquidity0 = SafeMath::div(
                SafeMath::mul(amount0, total_supply)?,
                reserve0,
            )?;
            let liquidity1 = SafeMath::div(
                SafeMath::mul(amount1, total_supply)?,
                reserve1,
            )?;
            
            Ok(SafeMath::min(liquidity0, liquidity1))
        }
    }

    /// Calculate the amounts of tokens to return when burning liquidity
    /// amount0 = liquidity * reserve0 / totalSupply
    /// amount1 = liquidity * reserve1 / totalSupply
    pub fn calculate_burn_amounts(
        liquidity: U256,
        reserve0: U256,
        reserve1: U256,
        total_supply: U256,
    ) -> Result<(U256, U256), DexError> {
        if total_supply.is_zero() {
            return Err(DexError::InsufficientLiquidity);
        }

        let amount0 = SafeMath::div(SafeMath::mul(liquidity, reserve0)?, total_supply)?;
        let amount1 = SafeMath::div(SafeMath::mul(liquidity, reserve1)?, total_supply)?;

        if amount0.is_zero() || amount1.is_zero() {
            return Err(DexError::InsufficientLiquidityBurned);
        }

        Ok((amount0, amount1))
    }

    /// Verify the K invariant after a swap
    /// k_new >= k_old (accounting for fees)
    pub fn verify_k_invariant(
        reserve0_old: U256,
        reserve1_old: U256,
        reserve0_new: U256,
        reserve1_new: U256,
    ) -> Result<bool, DexError> {
        let k_old = SafeMath::mul(reserve0_old, reserve1_old)?;
        let k_new = SafeMath::mul(reserve0_new, reserve1_new)?;
        
        Ok(k_new >= k_old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(SafeMath::sqrt(U256::from(0)), U256::from(0));
        assert_eq!(SafeMath::sqrt(U256::from(1)), U256::from(1));
        assert_eq!(SafeMath::sqrt(U256::from(4)), U256::from(2));
        assert_eq!(SafeMath::sqrt(U256::from(9)), U256::from(3));
        assert_eq!(SafeMath::sqrt(U256::from(16)), U256::from(4));
        assert_eq!(SafeMath::sqrt(U256::from(100)), U256::from(10));
    }

    #[test]
    fn test_get_amount_out() {
        let amount_in = U256::from(1000);
        let reserve_in = U256::from(10000);
        let reserve_out = U256::from(10000);

        let amount_out = AmmMath::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
        // With 0.3% fee, output should be slightly less than 1000
        assert!(amount_out < U256::from(1000));
        assert!(amount_out > U256::from(900));
    }

    #[test]
    fn test_get_amount_in() {
        let amount_out = U256::from(900);
        let reserve_in = U256::from(10000);
        let reserve_out = U256::from(10000);

        let amount_in = AmmMath::get_amount_in(amount_out, reserve_in, reserve_out).unwrap();
        // Input should be more than output due to fees
        assert!(amount_in > amount_out);
    }

    #[test]
    fn test_quote() {
        let amount_a = U256::from(1000);
        let reserve_a = U256::from(10000);
        let reserve_b = U256::from(20000);

        let amount_b = AmmMath::quote(amount_a, reserve_a, reserve_b).unwrap();
        assert_eq!(amount_b, U256::from(2000));
    }

    #[test]
    fn test_calculate_liquidity_first_deposit() {
        let amount0 = U256::from(10000);
        let amount1 = U256::from(10000);
        let reserve0 = U256::zero();
        let reserve1 = U256::zero();
        let total_supply = U256::zero();

        let liquidity = AmmMath::calculate_liquidity(
            amount0, amount1, reserve0, reserve1, total_supply
        ).unwrap();
        
        // sqrt(10000 * 10000) - 1000 = 10000 - 1000 = 9000
        assert_eq!(liquidity, U256::from(9000));
    }
}