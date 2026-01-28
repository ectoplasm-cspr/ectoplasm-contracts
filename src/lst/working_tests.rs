//! Working tests for LST contracts
//! Based on the DEX test pattern

#[cfg(test)]
mod working_tests {
    use odra::prelude::*;
    use odra::prelude::Addressable;
    use odra::casper_types::U256;
    use odra::host::{Deployer, HostEnv};
    
    use crate::lst::{ScsprToken, StakingManager};
    use crate::lst::scspr_token::ScsprTokenInitArgs;
    use crate::lst::staking_manager::StakingManagerInitArgs;

    fn cspr(amount: u64) -> U256 {
        U256::from(amount) * U256::from(1_000_000_000u64)
    }

    #[test]
    fn test_basic_staking_flow() {
        let env = odra_test::env();
        let validator = env.get_account(1);
        let user = env.get_account(2);

        // Deploy sCSPR token with a temporary staking manager
        let temp_manager = env.get_account(8);
        let mut scspr_token = ScsprToken::deploy(&env, ScsprTokenInitArgs {
            staking_manager: temp_manager,
        });

        // Deploy staking manager with the token address
        let mut staking_manager = StakingManager::deploy(&env, StakingManagerInitArgs {
            scspr_token_address: scspr_token.address(),
            
        });

        // Update token with correct staking manager
        scspr_token.set_staking_manager(staking_manager.address());

        // User stakes
        env.set_caller(user);
        let stake_amount = cspr(1000);
        let scspr_minted = staking_manager.stake(stake_amount);

        // Verify
        assert!(scspr_minted > U256::zero());
        assert_eq!(staking_manager.get_total_cspr_staked(), stake_amount);
        println!("✓ Basic staking flow test passed!");
        println!("  Staked: {} CSPR", stake_amount / cspr(1));
        println!("  Minted: {} sCSPR (in smallest units)", scspr_minted);
    }

    #[test]
    fn test_rewards_and_exchange_rate() {
        let env = odra_test::env();
        let validator = env.get_account(1);
        let user = env.get_account(2);
        let admin = env.get_account(0);

        // Setup contracts
        let temp_manager = env.get_account(8);
        let mut scspr_token = ScsprToken::deploy(&env, ScsprTokenInitArgs {
            staking_manager: temp_manager,
        });
        let mut staking_manager = StakingManager::deploy(&env, StakingManagerInitArgs {
            scspr_token_address: scspr_token.address(),
            
        });
        scspr_token.set_staking_manager(staking_manager.address());

        // User stakes 1000 CSPR
        env.set_caller(user);
        let stake_amount = cspr(1000);
        let scspr_minted = staking_manager.stake(stake_amount);

        // Admin distributes 100 CSPR rewards (10%)
        env.set_caller(admin);
        let rewards = cspr(100);
        staking_manager.distribute_rewards(rewards);

        // Check totals
        assert_eq!(staking_manager.get_total_cspr_staked(), stake_amount + rewards);
        assert_eq!(staking_manager.get_total_scspr_supply(), scspr_minted);

        // Exchange rate should be 1.1 CSPR per sCSPR
        let rate = staking_manager.get_exchange_rate();
        println!("✓ Rewards test passed!");
        println!("  Initial stake: {} CSPR", stake_amount / cspr(1));
        println!("  Rewards: {} CSPR", rewards / cspr(1));
        println!("  Exchange rate: {}", rate);
    }

    #[test]
    fn test_unstaking() {
        let env = odra_test::env();
        let validator = env.get_account(1);
        let user = env.get_account(2);

        // Setup contracts
        let temp_manager = env.get_account(8);
        let mut scspr_token = ScsprToken::deploy(&env, ScsprTokenInitArgs {
            staking_manager: temp_manager,
        });
        let mut staking_manager = StakingManager::deploy(&env, StakingManagerInitArgs {
            scspr_token_address: scspr_token.address(),
            
        });
        scspr_token.set_staking_manager(staking_manager.address());

        // User stakes
        env.set_caller(user);
        let stake_amount = cspr(1000);
        let scspr_minted = staking_manager.stake(stake_amount);

        // User unstakes half
        let unstake_amount = scspr_minted / U256::from(2u64);
        let request_id = staking_manager.unstake(unstake_amount);

        // Check request created
        let request = staking_manager.get_unstake_request(request_id);
        assert!(request.is_some());

        // Check sCSPR burned
        let remaining_scspr = scspr_token.balance_of(user);
        assert_eq!(remaining_scspr, scspr_minted - unstake_amount);

        println!("✓ Unstaking test passed!");
        println!("  Unstaked: {} sCSPR", unstake_amount);
        println!("  Request ID: {}", request_id);
    }

    #[test]
    fn test_multiple_users() {
        let env = odra_test::env();
        let validator = env.get_account(1);
        let user1 = env.get_account(2);
        let user2 = env.get_account(3);

        // Setup contracts
        let temp_manager = env.get_account(8);
        let mut scspr_token = ScsprToken::deploy(&env, ScsprTokenInitArgs {
            staking_manager: temp_manager,
        });
        let mut staking_manager = StakingManager::deploy(&env, StakingManagerInitArgs {
            scspr_token_address: scspr_token.address(),
            
        });
        scspr_token.set_staking_manager(staking_manager.address());

        // User 1 stakes 1000 CSPR
        env.set_caller(user1);
        let stake1 = cspr(1000);
        let scspr1 = staking_manager.stake(stake1);

        // User 2 stakes 500 CSPR
        env.set_caller(user2);
        let stake2 = cspr(500);
        let scspr2 = staking_manager.stake(stake2);

        // Check balances
        assert_eq!(scspr_token.balance_of(user1), scspr1);
        assert_eq!(scspr_token.balance_of(user2), scspr2);

        // Check total
        assert_eq!(staking_manager.get_total_cspr_staked(), stake1 + stake2);

        println!("✓ Multiple users test passed!");
        println!("  User 1 staked: {} CSPR, got {} sCSPR", stake1 / cspr(1), scspr1);
        println!("  User 2 staked: {} CSPR, got {} sCSPR", stake2 / cspr(1), scspr2);
    }
}
