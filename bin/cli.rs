
use ectoplasm_contracts::dex::factory::{Factory, FactoryInitArgs};
use ectoplasm_contracts::dex::pair::PairFactory;
use ectoplasm_contracts::dex::router::{Router, RouterInitArgs};
use ectoplasm_contracts::token::LpToken;
use ectoplasm_contracts::tokens::{EctoToken, UsdcToken, WethToken, WbtcToken};
use odra::prelude::{Address, Addressable};
use odra::host::{HostEnv, Deployer};
use odra::host::NoArgs;
use odra::schema::casper_contract_schema::NamedCLType;
use odra_cli::{
    deploy::DeployScript,
    scenario::{Args, Error, Scenario, ScenarioMetadata},
    CommandArg, ContractProvider, DeployedContractsContainer, DeployerExt,
    OdraCli,
};
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Deploys the complete DEX environment, matching scripts/deploy-new.sh
pub struct DeployNewScript;

impl DeployScript for DeployNewScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer
    ) -> Result<(), odra_cli::deploy::Error> {
        let caller = env.caller();

        // 1. Deploy Tokens
        println!("==> Deploying Tokens");
        
        // WCSPR (LpToken)
        use ectoplasm_contracts::token::LpTokenInitArgs;
        let wcspr = LpToken::load_or_deploy(
            &env,
            LpTokenInitArgs {
                name: String::from("Wrapped CSPR"),
                symbol: String::from("WCSPR"),
            },
            container,
            600_000_000_000
        )?;
        println!("WCSPR deployed at: {:?}", wcspr.address());

        // ECTO
        let _ecto = EctoToken::load_or_deploy(
            &env,
            NoArgs,
            container,
            600_000_000_000
        )?;
        println!("ECTO deployed at: {:?}", _ecto.address());

        // USDC
        let _usdc = UsdcToken::load_or_deploy(
            &env,
            NoArgs,
            container,
            600_000_000_000
        )?;
        println!("USDC deployed at: {:?}", _usdc.address());

        // WETH
        let _weth = WethToken::load_or_deploy(
            &env,
            NoArgs,
            container,
            600_000_000_000
        )?;
        println!("WETH deployed at: {:?}", _weth.address());

        // WBTC
        let _wbtc = WbtcToken::load_or_deploy(
            &env,
            NoArgs,
            container,
            600_000_000_000
        )?;
        println!("WBTC deployed at: {:?}", _wbtc.address());

        // 2. Deploy PairFactory
        println!("==> Deploying PairFactory");
        let pair_factory = PairFactory::load_or_deploy(
            &env,
            NoArgs,
            container,
            750_000_000_000 // High gas for factory deployment
        )?;
        println!("PairFactory deployed at: {:?}", pair_factory.address());

        // 3. Deploy Factory
        println!("==> Deploying Factory");
        let factory = Factory::load_or_deploy(
            &env,
            FactoryInitArgs {
                fee_to_setter: caller,
                pair_factory: pair_factory.address().clone(),
            },
            container,
            500_000_000_000
        )?;
        println!("Factory deployed at: {:?}", factory.address());

        // 4. Deploy Router
        println!("==> Deploying Router");
        let _router = Router::load_or_deploy(
            &env,
            RouterInitArgs {
                factory: factory.address().clone(),
                wcspr: wcspr.address().clone(),
            },
            container,
            600_000_000_000
        )?;
        println!("Router deployed at: {:?}", _router.address());

        generate_env_file(container);

        Ok(())
    }
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
        let token_a = args.get_single::<Address>("token_a")?;
        let token_b = args.get_single::<Address>("token_b")?;

        env.set_gas(900_000_000_000); // 900 CSPR for creating pair
        factory.try_create_pair(token_a, token_b)?;
        
        println!("Pair created successfully!");
        Ok(())
    }
}

impl ScenarioMetadata for CreatePairScenario {
    const NAME: &'static str = "create-pair";
    const DESCRIPTION: &'static str = "Creates a new trading pair for two tokens";
}

fn generate_env_file(container: &DeployedContractsContainer) {
    println!("==> Generating scripts/deploy-new.out.env");
    let node_address = std::env::var("ODRA_CASPER_LIVENET_NODE_ADDRESS")
        .or_else(|_| std::env::var("NODE_ADDRESS"))
        .expect("NODE_ADDRESS not set");
    let chain_name = std::env::var("ODRA_CASPER_LIVENET_CHAIN_NAME")
        .or_else(|_| std::env::var("CHAIN_NAME"))
        .unwrap_or_else(|_| "casper-test".to_string());
    let deployer = std::env::var("DEPLOYER_ACCOUNT_HASH")
        .unwrap_or_else(|_| "UNKNOWN".to_string());

    let mut file = File::create("scripts/deploy-new.out.env").expect("Unable to create file");

    writeln!(file, "NODE_ADDRESS={}", node_address).unwrap();
    writeln!(file, "CHAIN_NAME={}", chain_name).unwrap();
    writeln!(file, "DEPLOYER_ACCOUNT_HASH={}", deployer).unwrap();
    writeln!(file, "").unwrap();

    let mappings = vec![
        ("PairFactory", "PAIR_FACTORY"),
        ("Factory", "FACTORY"),
        ("Router", "ROUTER"),
        ("LpToken", "WCSPR"),
        ("EctoToken", "ECTO"),
        ("UsdcToken", "USDC"),
        ("WethToken", "WETH"),
        ("WbtcToken", "WBTC"),
    ];

    for (contract_name, env_prefix) in mappings {
        if let Some(address) = container.address_by_name(contract_name) {
            let addr_str = address.to_string(); 
            // format: Contract(ContractPackageHash(hex))
            let hex_part = addr_str
                .replace("Contract(ContractPackageHash(", "")
                .replace("))", "");
            let formatted_pkg_hash = format!("hash-{}", hex_part);
             
            writeln!(file, "{}_PACKAGE_HASH={}", env_prefix, formatted_pkg_hash).unwrap();
             
           let contract_hash = get_contract_hash(&node_address, &formatted_pkg_hash);
           writeln!(file, "{}_CONTRACT_HASH={}", env_prefix, contract_hash).unwrap();
        }
    }
}

fn get_contract_hash(node_address: &str, package_hash: &str) -> String {
    let output = Command::new("casper-client")
        .arg("query-global-state")
        .arg("--node-address")
        .arg(node_address)
        .arg("--key")
        .arg(package_hash)
        .output();

    match output {
        Ok(out) => {
             let output_str = String::from_utf8_lossy(&out.stdout);
             // Find the last "contract_hash": "contract-..."
             let mut last_hash = String::from("NOT_FOUND");
             for line in output_str.lines() {
                 if line.contains("contract_hash") {
                     if let Some(start) = line.find("contract-") {
                         // assume it ends with "
                         let end = line[start..].find('"').unwrap_or(line[start..].len());
                         last_hash = line[start..start+end].to_string();
                     }
                 }
             }
             last_hash
        },
        Err(_) => String::from("ERROR_CALLING_CLIENT")
    }
}
pub fn main() {
    OdraCli::new()
        .about("CLI tool for Casper DEX smart contracts")
        // Deploy scripts
        .deploy(DeployNewScript)
        // Contract references
        .contract::<Factory>()
        .contract::<PairFactory>()
        .contract::<Router>()
        .contract::<LpToken>()
        .contract::<EctoToken>()
        .contract::<UsdcToken>()
        .contract::<WethToken>()
        .contract::<WbtcToken>()
        // Scenarios
        .scenario(CreatePairScenario)
        .build()
        .run();
}
