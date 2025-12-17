//! Router contract for the DEX
//! 
//! The Router is the main user-facing contract that provides:
//! - Adding/removing liquidity
//! - Swapping tokens (exact input and exact output)
//! - Multi-hop swaps through multiple pairs
//! - Deadline protection
use odra::prelude::*;
use odra::casper_types::U256;
use odra::{Address, Var};
use crate::errors::DexError;
use crate::math::{AmmMath, SafeMath};
use crate::token::Cep18TokenContractRef;

/// External interface for Pair contract
#[odra::external_contract]
pub trait PairContract {
    fn token0(&self) -> Address;
    fn token1(&self) -> Address;
    fn get_reserves(&self) -> (U256, U256, u64);
    fn mint(&mut self, to: Address) -> Result<U256, DexError>;
    fn burn(&mut self, to: Address) -> Result<(U256, U256), DexError>;
    fn swap(&mut self, amount0_out: U256, amount1_out: U256, to: Address) -> Result<(), DexError>;
    fn transfer_from(&mut self, from: Address, to: Address, amount: U256) -> bool;
}

/// External interface for Factory contract
#[odra::external_contract]
pub trait FactoryContractRef {
    fn get_pair(&self, token_a: Address, token_b: Address) -> Option<Address>;
    fn create_pair(&mut self, token_a: Address, token_b: Address) -> Result<Address, DexError>;
}

/// Router contract for user interactions
#[odra::module]
pub struct Router {
    /// Factory contract address
    factory: Var<Address>,
    /// WCSPR (Wrapped CSPR) token address for native token swaps
    wcspr: Var<Address>,
}

#[odra::module]
impl Router {
    /// Initialize the router with factory and WCSPR addresses
    pub fn init(&mut self, factory: Address, wcspr: Address) {
        self.factory.set(factory);
        self.wcspr.set(wcspr);
    }

    /// Get the factory address
    pub fn factory(&self) -> Address {
        self.factory.get_or_revert()
    }

    /// Get the WCSPR address
    pub fn wcspr(&self) -> Address {
        self.wcspr.get_or_revert()
    }

    // ============ Liquidity Functions ============

    /// Add liquidity to a pair
    /// Returns (amount_a, amount_b, liquidity)
    pub fn add_liquidity(
        &mut self,
        token_a: Address,
        token_b: Address,
        amount_a_desired: U256,
        amount_b_desired: U256,
        amount_a_min: U256,
        amount_b_min: U256,
        to: Address,
        deadline: u64,
    ) -> Result<(U256, U256, U256), DexError> {
        self.ensure_deadline(deadline)?;

        // Calculate optimal amounts
        let (amount_a, amount_b) = self.calculate_liquidity_amounts(
            token_a,
            token_b,
            amount_a_desired,
            amount_b_desired,
            amount_a_min,
            amount_b_min,
        )?;

        // Get or create pair
        let pair = self.get_or_create_pair(token_a, token_b)?;

        // Transfer tokens to pair
        self.safe_transfer_from(token_a, self.env().caller(), pair, amount_a)?;
        self.safe_transfer_from(token_b, self.env().caller(), pair, amount_b)?;

        // Mint LP tokens
        let mut pair_ref = PairContractContractRef::new(self.env(), pair);
        let liquidity = pair_ref.mint(to)?;

        Ok((amount_a, amount_b, liquidity))
    }

    /// Remove liquidity from a pair
    /// Returns (amount_a, amount_b)
    pub fn remove_liquidity(
        &mut self,
        token_a: Address,
        token_b: Address,
        liquidity: U256,
        amount_a_min: U256,
        amount_b_min: U256,
        to: Address,
        deadline: u64,
    ) -> Result<(U256, U256), DexError> {
        self.ensure_deadline(deadline)?;

        let pair = self.get_pair(token_a, token_b)?;

        // Transfer LP tokens to pair
        let mut pair_ref = PairContractContractRef::new(self.env(), pair);
        pair_ref.transfer_from(self.env().caller(), pair, liquidity);

        // Burn LP tokens and get underlying tokens
        let (amount0, amount1) = pair_ref.burn(to)?;

        // Sort tokens to match pair order
        let (token0, _) = self.sort_tokens(token_a, token_b);
        let (amount_a, amount_b) = if token_a == token0 {
            (amount0, amount1)
        } else {
            (amount1, amount0)
        };

        // Check minimum amounts
        if amount_a < amount_a_min {
            return Err(DexError::InsufficientAmount);
        }
        if amount_b < amount_b_min {
            return Err(DexError::InsufficientAmount);
        }

        Ok((amount_a, amount_b))
    }

    // ============ Swap Functions ============

    /// Swap exact input amount for output tokens
    /// path is an array of token addresses representing the swap route
    pub fn swap_exact_tokens_for_tokens(
        &mut self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Result<Vec<U256>, DexError> {
        self.ensure_deadline(deadline)?;

        let amounts = self.get_amounts_out(amount_in, &path)?;
        
        if amounts[amounts.len() - 1] < amount_out_min {
            return Err(DexError::InsufficientOutputAmount);
        }

        // Transfer input tokens to first pair
        let pair = self.get_pair(path[0], path[1])?;
        self.safe_transfer_from(path[0], self.env().caller(), pair, amounts[0])?;

        // Execute swaps
        self.execute_swap(&amounts, &path, to)?;

        Ok(amounts)
    }

    /// Swap tokens for exact output amount
    pub fn swap_tokens_for_exact_tokens(
        &mut self,
        amount_out: U256,
        amount_in_max: U256,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Result<Vec<U256>, DexError> {
        self.ensure_deadline(deadline)?;

        let amounts = self.get_amounts_in(amount_out, &path)?;
        
        if amounts[0] > amount_in_max {
            return Err(DexError::ExcessiveSlippage);
        }

        // Transfer input tokens to first pair
        let pair = self.get_pair(path[0], path[1])?;
        self.safe_transfer_from(path[0], self.env().caller(), pair, amounts[0])?;

        // Execute swaps
        self.execute_swap(&amounts, &path, to)?;

        Ok(amounts)
    }

    // ============ Quote Functions ============

    /// Get the output amount for a given input amount
    pub fn get_amount_out(
        &self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, DexError> {
        AmmMath::get_amount_out(amount_in, reserve_in, reserve_out)
    }

    /// Get the input amount required for a given output amount
    pub fn get_amount_in(
        &self,
        amount_out: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, DexError> {
        AmmMath::get_amount_in(amount_out, reserve_in, reserve_out)
    }

    /// Get output amounts for a swap path
    pub fn get_amounts_out(
        &self,
        amount_in: U256,
        path: &[Address],
    ) -> Result<Vec<U256>, DexError> {
        if path.len() < 2 {
            return Err(DexError::InvalidPath);
        }

        let mut amounts = Vec::with_capacity(path.len());
        amounts.push(amount_in);

        for i in 0..path.len() - 1 {
            let (reserve_in, reserve_out) = self.get_reserves(path[i], path[i + 1])?;
            let amount_out = AmmMath::get_amount_out(amounts[i], reserve_in, reserve_out)?;
            amounts.push(amount_out);
        }

        Ok(amounts)
    }

    /// Get input amounts for a swap path
    pub fn get_amounts_in(
        &self,
        amount_out: U256,
        path: &[Address],
    ) -> Result<Vec<U256>, DexError> {
        if path.len() < 2 {
            return Err(DexError::InvalidPath);
        }

        let mut amounts = vec![U256::zero(); path.len()];
        amounts[path.len() - 1] = amount_out;

        for i in (0..path.len() - 1).rev() {
            let (reserve_in, reserve_out) = self.get_reserves(path[i], path[i + 1])?;
            let amount_in = AmmMath::get_amount_in(amounts[i + 1], reserve_in, reserve_out)?;
            amounts[i] = amount_in;
        }

        Ok(amounts)
    }

    /// Quote the amount of token B for a given amount of token A
    pub fn quote(
        &self,
        amount_a: U256,
        reserve_a: U256,
        reserve_b: U256,
    ) -> Result<U256, DexError> {
        AmmMath::quote(amount_a, reserve_a, reserve_b)
    }

    /// Get reserves for a token pair
    pub fn get_reserves(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> Result<(U256, U256), DexError> {
        let (token0, _) = self.sort_tokens(token_a, token_b);
        let pair = self.get_pair(token_a, token_b)?;
        
        let pair_ref = PairContractContractRef::new(self.env(), pair);
        let (reserve0, reserve1, _) = pair_ref.get_reserves();

        if token_a == token0 {
            Ok((reserve0, reserve1))
        } else {
            Ok((reserve1, reserve0))
        }
    }

    // ============ Internal Functions ============

    /// Ensure the deadline has not passed
    fn ensure_deadline(&self, deadline: u64) -> Result<(), DexError> {
        if self.env().get_block_time() > deadline {
            return Err(DexError::DeadlineExpired);
        }
        Ok(())
    }

    /// Sort two token addresses
    fn sort_tokens(&self, token_a: Address, token_b: Address) -> (Address, Address) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }

    /// Get pair address for two tokens
    fn get_pair(&self, token_a: Address, token_b: Address) -> Result<Address, DexError> {
        let factory_ref = FactoryContractRefContractRef::new(self.env(), self.factory());
        factory_ref
            .get_pair(token_a, token_b)
            .ok_or(DexError::PairNotFound)
    }

    /// Get or create pair for two tokens
    fn get_or_create_pair(
        &mut self,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, DexError> {
        let factory = self.factory();
        let factory_ref = FactoryContractRefContractRef::new(self.env(), factory);
        
        match factory_ref.get_pair(token_a, token_b) {
            Some(pair) => Ok(pair),
            None => {
                let mut factory_ref_mut = FactoryContractRefContractRef::new(self.env(), factory);
                factory_ref_mut.create_pair(token_a, token_b)
            }
        }
    }

    /// Calculate optimal liquidity amounts
    fn calculate_liquidity_amounts(
        &self,
        token_a: Address,
        token_b: Address,
        amount_a_desired: U256,
        amount_b_desired: U256,
        amount_a_min: U256,
        amount_b_min: U256,
    ) -> Result<(U256, U256), DexError> {
        // Try to get existing reserves
        let reserves = self.get_reserves(token_a, token_b);
        
        match reserves {
            Ok((reserve_a, reserve_b)) if !reserve_a.is_zero() && !reserve_b.is_zero() => {
                // Calculate optimal amount B
                let amount_b_optimal = AmmMath::quote(amount_a_desired, reserve_a, reserve_b)?;
                
                if amount_b_optimal <= amount_b_desired {
                    if amount_b_optimal < amount_b_min {
                        return Err(DexError::InsufficientAmount);
                    }
                    Ok((amount_a_desired, amount_b_optimal))
                } else {
                    // Calculate optimal amount A
                    let amount_a_optimal = AmmMath::quote(amount_b_desired, reserve_b, reserve_a)?;
                    
                    if amount_a_optimal > amount_a_desired {
                        return Err(DexError::InsufficientAmount);
                    }
                    if amount_a_optimal < amount_a_min {
                        return Err(DexError::InsufficientAmount);
                    }
                    Ok((amount_a_optimal, amount_b_desired))
                }
            }
            _ => {
                // First liquidity provision - use desired amounts
                Ok((amount_a_desired, amount_b_desired))
            }
        }
    }

    /// Execute a multi-hop swap
    fn execute_swap(
        &self,
        amounts: &[U256],
        path: &[Address],
        to: Address,
    ) -> Result<(), DexError> {
        for i in 0..path.len() - 1 {
            let (input, output) = (path[i], path[i + 1]);
            let (token0, _) = self.sort_tokens(input, output);
            let amount_out = amounts[i + 1];

            let (amount0_out, amount1_out) = if input == token0 {
                (U256::zero(), amount_out)
            } else {
                (amount_out, U256::zero())
            };

            // Determine recipient
            let recipient = if i < path.len() - 2 {
                self.get_pair(output, path[i + 2])?
            } else {
                to
            };

            let pair = self.get_pair(input, output)?;
            let mut pair_ref = PairContractContractRef::new(self.env(), pair);
            pair_ref.swap(amount0_out, amount1_out, recipient)?;
        }

        Ok(())
    }

    /// Safe transfer tokens from one address to another
    fn safe_transfer_from(
        &self,
        token: Address,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<(), DexError> {
        let mut token_ref = Cep18TokenContractRef::new(self.env(), token);
        let success = token_ref.transfer_from(from, to, amount);
        if !success {
            return Err(DexError::TransferFailed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv};

    #[test]
    fn test_router_init() {
        let env = odra_test::env();
        let factory = env.get_account(1);
        let wcspr = env.get_account(2);

        let init_args = RouterInitArgs {
            factory,
            wcspr,
        };
        let router = Router::deploy(&env, init_args);

        assert_eq!(router.factory(), factory);
        assert_eq!(router.wcspr(), wcspr);
    }

    #[test]
    fn test_sort_tokens() {
        let env = odra_test::env();
        let factory = env.get_account(1);
        let wcspr = env.get_account(2);

        let init_args = RouterInitArgs {
            factory,
            wcspr,
        };
        let router = Router::deploy(&env, init_args);

        let token_a = env.get_account(3);
        let token_b = env.get_account(4);

        // Test that tokens are sorted correctly
        let (t0, t1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        // The sort_tokens function is private, but we can verify behavior
        // through other public functions that use it
    }
}