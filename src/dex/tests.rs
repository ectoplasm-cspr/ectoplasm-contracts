//! Integration tests for the DEX smart contracts
//! 
//! These tests verify the complete DEX functionality including:
//! - Pair creation
//! - Liquidity provision
//! - Token swaps
//! - Fee collection

#[cfg(test)]
mod integration_tests {
    use odra::prelude::*;
    use odra::casper_types::U256;
    use odra::host::{Deployer, HostEnv};
    
    use crate::dex::factory::{Factory, FactoryInitArgs, FactoryHostRef};
    use crate::dex::router::{Router, RouterInitArgs, RouterHostRef};
    use crate::dex::pair::{Pair, PairInitArgs, PairHostRef};
    use crate::token::{LpToken, LpTokenInitArgs, LpTokenHostRef};
    use crate::errors::DexError;

    /// Helper struct to set up test environment
    struct TestEnv {
        env: HostEnv,
        factory: FactoryHostRef,
        router: RouterHostRef,
        token_a: LpTokenHostRef,
        token_b: LpTokenHostRef,
        wcspr: LpTokenHostRef,
    }

    impl TestEnv {
        fn new() -> Self {
            let env = odra_test::env();
            let admin = env.get_account(0);

            // Deploy mock tokens
            let token_a = LpToken::deploy(&env, LpTokenInitArgs {
                name: String::from("Token A"),
                symbol: String::from("TKA"),
            });

            let token_b = LpToken::deploy(&env, LpTokenInitArgs {
                name: String::from("Token B"),
                symbol: String::from("TKB"),
            });

            let wcspr = LpToken::deploy(&env, LpTokenInitArgs {
                name: String::from("Wrapped CSPR"),
                symbol: String::from("WCSPR"),
            });

            // Deploy factory
            let factory = Factory::deploy(&env, FactoryInitArgs {
                fee_to_setter: admin,
            });

            // Deploy router
            let router = Router::deploy(&env, RouterInitArgs {
                factory: factory.address().clone(),
                wcspr: wcspr.address().clone(),
            });

            TestEnv {
                env,
                factory,
                router,
                token_a,
                token_b,
                wcspr,
            }
        }

        fn mint_tokens(&mut self, user: odra::Address, amount: U256) {
            self.token_a.mint(user, amount);
            self.token_b.mint(user, amount);
        }
    }

    #[test]
    fn test_factory_deployment() {
        let test_env = TestEnv::new();
        let admin = test_env.env.get_account(0);

        assert_eq!(test_env.factory.fee_to_setter(), admin);
        assert_eq!(test_env.factory.all_pairs_length(), 0);
    }

    #[test]
    fn test_create_pair() {
        let mut test_env = TestEnv::new();
        
        let token_a_addr = test_env.token_a.address().clone();
        let token_b_addr = test_env.token_b.address().clone();

        let result = test_env.factory.create_pair(token_a_addr, token_b_addr);
        assert!(result.is_ok());

        assert_eq!(test_env.factory.all_pairs_length(), 1);
        assert!(test_env.factory.pair_exists(token_a_addr, token_b_addr));
    }

    #[test]
    fn test_create_duplicate_pair_fails() {
        let mut test_env = TestEnv::new();
        
        let token_a_addr = test_env.token_a.address().clone();
        let token_b_addr = test_env.token_b.address().clone();

        test_env.factory.create_pair(token_a_addr, token_b_addr).unwrap();
        
        let result = test_env.factory.create_pair(token_a_addr, token_b_addr);
        assert_eq!(result, Err(DexError::PairExists));
    }

    #[test]
    fn test_router_deployment() {
        let test_env = TestEnv::new();

        assert_eq!(test_env.router.factory(), test_env.factory.address().clone());
        assert_eq!(test_env.router.wcspr(), test_env.wcspr.address().clone());
    }

    #[test]
    fn test_amm_math_get_amount_out() {
        use crate::math::AmmMath;

        // Test with equal reserves
        let amount_in = U256::from(1000);
        let reserve_in = U256::from(10000);
        let reserve_out = U256::from(10000);

        let amount_out = AmmMath::get_amount_out(amount_in, reserve_in, reserve_out).unwrap();
        
        // With 0.3% fee, output should be less than input
        assert!(amount_out < amount_in);
        // But should be close (around 906 for these values)
        assert!(amount_out > U256::from(900));
    }

    #[test]
    fn test_amm_math_get_amount_in() {
        use crate::math::AmmMath;

        let amount_out = U256::from(900);
        let reserve_in = U256::from(10000);
        let reserve_out = U256::from(10000);

        let amount_in = AmmMath::get_amount_in(amount_out, reserve_in, reserve_out).unwrap();
        
        // Input should be more than output due to fees
        assert!(amount_in > amount_out);
    }

    #[test]
    fn test_amm_math_quote() {
        use crate::math::AmmMath;

        let amount_a = U256::from(1000);
        let reserve_a = U256::from(10000);
        let reserve_b = U256::from(20000);

        let amount_b = AmmMath::quote(amount_a, reserve_a, reserve_b).unwrap();
        
        // Should maintain the ratio: 1000 * 20000 / 10000 = 2000
        assert_eq!(amount_b, U256::from(2000));
    }

    #[test]
    fn test_amm_math_sqrt() {
        use crate::math::SafeMath;

        assert_eq!(SafeMath::sqrt(U256::from(0)), U256::from(0));
        assert_eq!(SafeMath::sqrt(U256::from(1)), U256::from(1));
        assert_eq!(SafeMath::sqrt(U256::from(4)), U256::from(2));
        assert_eq!(SafeMath::sqrt(U256::from(9)), U256::from(3));
        assert_eq!(SafeMath::sqrt(U256::from(100)), U256::from(10));
        assert_eq!(SafeMath::sqrt(U256::from(10000)), U256::from(100));
    }

    #[test]
    fn test_lp_token_mint_and_burn() {
        let env = odra_test::env();
        let user = env.get_account(1);

        let mut token = LpToken::deploy(&env, LpTokenInitArgs {
            name: String::from("LP Token"),
            symbol: String::from("LP"),
        });

        let amount = U256::from(1000);

        // Mint tokens
        token.mint(user, amount);
        assert_eq!(token.balance_of(user), amount);
        assert_eq!(token.total_supply(), amount);

        // Burn tokens
        token.burn(user, amount);
        assert_eq!(token.balance_of(user), U256::zero());
        assert_eq!(token.total_supply(), U256::zero());
    }

    #[test]
    fn test_lp_token_transfer() {
        let env = odra_test::env();
        let user1 = env.get_account(0);
        let user2 = env.get_account(1);

        let mut token = LpToken::deploy(&env, LpTokenInitArgs {
            name: String::from("LP Token"),
            symbol: String::from("LP"),
        });

        let amount = U256::from(1000);
        token.mint(user1, amount);

        env.set_caller(user1);
        token.transfer(user2, U256::from(500));

        assert_eq!(token.balance_of(user1), U256::from(500));
        assert_eq!(token.balance_of(user2), U256::from(500));
    }

    #[test]
    fn test_lp_token_approve_and_transfer_from() {
        let env = odra_test::env();
        let owner = env.get_account(0);
        let spender = env.get_account(1);
        let recipient = env.get_account(2);

        let mut token = LpToken::deploy(&env, LpTokenInitArgs {
            name: String::from("LP Token"),
            symbol: String::from("LP"),
        });

        let amount = U256::from(1000);
        token.mint(owner, amount);

        // Approve spender
        env.set_caller(owner);
        token.approve(spender, U256::from(500));
        assert_eq!(token.allowance(owner, spender), U256::from(500));

        // Transfer from owner to recipient via spender
        env.set_caller(spender);
        token.transfer_from(owner, recipient, U256::from(300));

        assert_eq!(token.balance_of(owner), U256::from(700));
        assert_eq!(token.balance_of(recipient), U256::from(300));
        assert_eq!(token.allowance(owner, spender), U256::from(200));
    }

    #[test]
    fn test_factory_set_fee_to() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let fee_recipient = env.get_account(1);

        let mut factory = Factory::deploy(&env, FactoryInitArgs {
            fee_to_setter: admin,
        });

        env.set_caller(admin);
        factory.set_fee_to(fee_recipient).unwrap();

        assert_eq!(factory.fee_to(), Some(fee_recipient));
    }

    #[test]
    fn test_factory_set_fee_to_unauthorized() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let non_admin = env.get_account(1);
        let fee_recipient = env.get_account(2);

        let mut factory = Factory::deploy(&env, FactoryInitArgs {
            fee_to_setter: admin,
        });

        env.set_caller(non_admin);
        let result = factory.set_fee_to(fee_recipient);

        assert_eq!(result, Err(DexError::Unauthorized));
    }

    #[test]
    fn test_pair_initialization() {
        let env = odra_test::env();
        let token0 = env.get_account(1);
        let token1 = env.get_account(2);
        let factory = env.get_account(0);

        let pair = Pair::deploy(&env, PairInitArgs {
            token0,
            token1,
            factory,
        });

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
        assert_eq!(pair.total_supply(), U256::zero());
    }

    #[test]
    fn test_calculate_liquidity_first_deposit() {
        use crate::math::AmmMath;

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

    #[test]
    fn test_calculate_liquidity_subsequent_deposit() {
        use crate::math::AmmMath;

        let amount0 = U256::from(1000);
        let amount1 = U256::from(1000);
        let reserve0 = U256::from(10000);
        let reserve1 = U256::from(10000);
        let total_supply = U256::from(9000);

        let liquidity = AmmMath::calculate_liquidity(
            amount0, amount1, reserve0, reserve1, total_supply
        ).unwrap();

        // min(1000 * 9000 / 10000, 1000 * 9000 / 10000) = 900
        assert_eq!(liquidity, U256::from(900));
    }

    #[test]
    fn test_calculate_burn_amounts() {
        use crate::math::AmmMath;

        let liquidity = U256::from(900);
        let reserve0 = U256::from(10000);
        let reserve1 = U256::from(10000);
        let total_supply = U256::from(9000);

        let (amount0, amount1) = AmmMath::calculate_burn_amounts(
            liquidity, reserve0, reserve1, total_supply
        ).unwrap();

        // 900 * 10000 / 9000 = 1000
        assert_eq!(amount0, U256::from(1000));
        assert_eq!(amount1, U256::from(1000));
    }

    #[test]
    fn test_k_invariant_verification() {
        use crate::math::AmmMath;

        let reserve0_old = U256::from(10000);
        let reserve1_old = U256::from(10000);
        let reserve0_new = U256::from(10997);
        let reserve1_new = U256::from(9094);

        // After a swap, k should be maintained or increased
        let result = AmmMath::verify_k_invariant(
            reserve0_old, reserve1_old, reserve0_new, reserve1_new
        ).unwrap();

        assert!(result);
    }
}