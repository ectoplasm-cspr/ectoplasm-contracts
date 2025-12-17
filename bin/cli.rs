//! CLI tool for deploying and interacting with DEX smart contracts.

use my_project::flipper::Flipper;
use my_project::dex::factory::Factory;
use my_project::dex::router::Router;
use my_project::token::LpToken;
use odra::host::{HostEnv, NoArgs};
use odra::schema::casper_contract_schema::NamedCLType;
use odra_cli::{
    deploy::DeployScript,
    scenario::{Args, Error, Scenario, ScenarioMetadata},
    CommandArg, ContractProvider, DeployedContractsContainer, DeployerExt,
    OdraCli, 
};

/// Deploys the `Flipper` and adds it to the container.
pub struct FlipperDeployScript;

impl DeployScript for FlipperDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer
    ) -> Result<(), odra_cli::deploy::Error> {
        let _flipper = Flipper::load_or_deploy(
            &env,
            NoArgs,
            container,
            350_000_000_000 // Adjust gas limit as needed
        )?;

        Ok(())
    }
}

/// Deploys the DEX Factory contract.
pub struct FactoryDeployScript;

impl DeployScript for FactoryDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer
    ) -> Result<(), odra_cli::deploy::Error> {
        use my_project::dex::factory::FactoryInitArgs;
        
        let caller = env.caller();
        let _factory = Factory::load_or_deploy(
            &env,
            FactoryInitArgs {
                fee_to_setter: caller,
            },
            container,
            500_000_000_000 // Gas limit for factory deployment
        )?;

        Ok(())
    }
}

/// Deploys the DEX Router contract.
/// Requires Factory to be deployed first.
pub struct RouterDeployScript;

impl DeployScript for RouterDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer
    ) -> Result<(), odra_cli::deploy::Error> {
        use my_project::dex::router::RouterInitArgs;
        use my_project::token::LpTokenInitArgs;
        
        // Get factory address from container
        let factory = container.contract_ref::<Factory>(env)?;
        let factory_address = factory.address().clone();
        
        // Deploy WCSPR token if not exists
        let wcspr = LpToken::load_or_deploy(
            &env,
            LpTokenInitArgs {
                name: String::from("Wrapped CSPR"),
                symbol: String::from("WCSPR"),
            },
            container,
            300_000_000_000
        )?;
        
        let _router = Router::load_or_deploy(
            &env,
            RouterInitArgs {
                factory: factory_address,
                wcspr: wcspr.address().clone(),
            },
            container,
            500_000_000_000 // Gas limit for router deployment
        )?;

        Ok(())
    }
}

/// Deploys the complete DEX (Factory + Router).
pub struct DexDeployScript;

impl DeployScript for DexDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer
    ) -> Result<(), odra_cli::deploy::Error> {
        // Deploy Factory first
        FactoryDeployScript.deploy(env, container)?;
        
        // Then deploy Router
        RouterDeployScript.deploy(env, container)?;
        
        Ok(())
    }
}

/// Scenario that flips the state of the deployed `Flipper` contract a specified number of times.
pub struct FlippingScenario;

impl Scenario for FlippingScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![CommandArg::new(
            "number",
            "The number of times to flip the state",
            NamedCLType::U64,
        )]
    }

    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        args: Args
    ) -> Result<(), Error> {
        let mut contract = container.contract_ref::<Flipper>(env)?;
        let n = args.get_single::<u64>("number")?;

        env.set_gas(50_000_000);
        for _ in 0..n {
            contract.try_flip()?;
        }

        Ok(())
    }
}

impl ScenarioMetadata for FlippingScenario {
    const NAME: &'static str = "flip";
    const DESCRIPTION: &'static str =
        "Flips the state of the deployed flipper contract a specified number of times";
}

/// Scenario to create a new trading pair.
pub struct CreatePairScenario;

impl Scenario for CreatePairScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new(
                "token_a",
                "Address of the first token",
                NamedCLType::Key,
            ),
            CommandArg::new(
                "token_b",
                "Address of the second token",
                NamedCLType::Key,
            ),
        ]
    }

    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        args: Args
    ) -> Result<(), Error> {
        let mut factory = container.contract_ref::<Factory>(env)?;
        let token_a = args.get_single::<odra::Address>("token_a")?;
        let token_b = args.get_single::<odra::Address>("token_b")?;

        env.set_gas(300_000_000_000);
        factory.try_create_pair(token_a, token_b)?;
        
        println!("Pair created successfully!");
        Ok(())
    }
}

impl ScenarioMetadata for CreatePairScenario {
    const NAME: &'static str = "create-pair";
    const DESCRIPTION: &'static str = "Creates a new trading pair for two tokens";
}

/// Main function to run the CLI tool.
pub fn main() {
    OdraCli::new()
        .about("CLI tool for Casper DEX smart contracts")
        // Deploy scripts
        .deploy(FlipperDeployScript)
        .deploy(FactoryDeployScript)
        .deploy(RouterDeployScript)
        .deploy(DexDeployScript)
        // Contract references
        .contract::<Flipper>()
        .contract::<Factory>()
        .contract::<Router>()
        .contract::<LpToken>()
        // Scenarios
        .scenario(FlippingScenario)
        .scenario(CreatePairScenario)
        .build()
        .run();
}
