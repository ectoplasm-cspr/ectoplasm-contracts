
use ectoplasm_contracts::dex::factory::{Factory, FactoryInitArgs};
use ectoplasm_contracts::dex::pair::PairFactory;
use ectoplasm_contracts::dex::router::{Router, RouterInitArgs};
use ectoplasm_contracts::token::LpToken;
use ectoplasm_contracts::tokens::{EctoToken, UsdcToken, WethToken, WbtcToken};
use ectoplasm_contracts::launchpad::token_factory::{TokenFactory, TokenFactoryInitArgs};
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

/// Scenario to deploy Launchpad contracts (TokenFactory only for now)
/// Does NOT redeploy DEX contracts - uses existing Router/Factory from environment
/// Usage: cargo run --bin ectoplasm_contracts_cli -- scenarios deploy-launchpad
pub struct DeployLaunchpadScenario;

impl Scenario for DeployLaunchpadScenario {
    fn args(&self) -> Vec<CommandArg> {
        // No additional args needed - we read from environment
        vec![]
    }

    fn run(
        &self,
        env: &HostEnv,
        _container: &DeployedContractsContainer,
        _args: Args
    ) -> Result<(), Error> {
        println!("==> Deploying Launchpad Contracts");
        
        // Get existing DEX addresses from environment
        let router_hash = std::env::var("ROUTER_PACKAGE_HASH")
            .or_else(|_| std::env::var("VITE_ROUTER_PACKAGE_HASH"))
            .map_err(|_| Error::OdraError { message: "ROUTER_PACKAGE_HASH not set - run DEX deployment first".to_string() })?;
        let factory_hash = std::env::var("FACTORY_PACKAGE_HASH")
            .or_else(|_| std::env::var("VITE_FACTORY_PACKAGE_HASH"))
            .map_err(|_| Error::OdraError { message: "FACTORY_PACKAGE_HASH not set - run DEX deployment first".to_string() })?;
        
        println!("Using existing DEX Router: {}", router_hash);
        println!("Using existing DEX Factory: {}", factory_hash);
        
        // Parse addresses using PackageHash (Odra 2.5 naming)
        use odra::casper_types::PackageHash;
        
        // Remove "hash-" prefix and parse
        let router_hex = router_hash.replace("hash-", "");
        let factory_hex = factory_hash.replace("hash-", "");
        
        let router_pkg = PackageHash::from_formatted_str(&format!("package-{}", router_hex))
            .map_err(|e| Error::OdraError { message: format!("Failed to parse router package hash: {:?}", e) })?;
        let factory_pkg = PackageHash::from_formatted_str(&format!("package-{}", factory_hex))
            .map_err(|e| Error::OdraError { message: format!("Failed to parse factory package hash: {:?}", e) })?;
        
        let router_addr = Address::from(router_pkg);
        let factory_addr = Address::from(factory_pkg);
        
        // Deploy TokenFactory using Deployer trait directly
        println!("==> Deploying TokenFactory");
        env.set_gas(800_000_000_000); // 800 CSPR
        
        let token_factory = TokenFactory::deploy(
            env,
            TokenFactoryInitArgs {
                dex_router: router_addr,
                dex_factory: factory_addr,
            },
        );
        println!("TokenFactory deployed at: {:?}", token_factory.address());
        
        // Write the deployed address to env file
        let node_address = std::env::var("ODRA_CASPER_LIVENET_NODE_ADDRESS")
            .or_else(|_| std::env::var("NODE_ADDRESS"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        let mut file = File::create("scripts/deploy-launchpad.out.env")
            .map_err(|e| Error::OdraError { message: format!("Failed to create env file: {:?}", e) })?;
        
        writeln!(file, "# Launchpad Deployment Output").unwrap();
        writeln!(file, "").unwrap();
        
        let addr_str = token_factory.address().to_string();
        let hex_part = addr_str
            .replace("Contract(ContractPackageHash(", "")
            .replace("))", "");
        let formatted_pkg_hash = format!("hash-{}", hex_part);
         
        writeln!(file, "LAUNCHPAD_TOKEN_FACTORY_PACKAGE_HASH={}", formatted_pkg_hash).unwrap();
         
        let contract_hash = get_contract_hash(&node_address, &formatted_pkg_hash);
        writeln!(file, "LAUNCHPAD_TOKEN_FACTORY_CONTRACT_HASH={}", contract_hash).unwrap();
        
        println!("==> Launchpad env file generated at scripts/deploy-launchpad.out.env");
        println!("Add these to your frontend .env to enable launchpad features.");
        
        Ok(())
    }
}

impl ScenarioMetadata for DeployLaunchpadScenario {
    const NAME: &'static str = "deploy-launchpad";
    const DESCRIPTION: &'static str = "Deploys the Launchpad TokenFactory contract (requires DEX to be deployed first)";
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

/// Scenario to create a new token launch.
pub struct CreateLaunchScenario;

impl Scenario for CreateLaunchScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new(
                "name",
                "Token name",
                NamedCLType::String,
            ),
            CommandArg::new(
                "symbol",
                "Token symbol (max 6 chars)",
                NamedCLType::String,
            ),
            CommandArg::new(
                "curve_type",
                "Curve type: 0=Linear, 1=Sigmoid, 2=Steep",
                NamedCLType::U8,
            ),
        ]
    }

    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        args: Args
    ) -> Result<(), Error> {
        let mut token_factory = container.contract_ref::<TokenFactory>(env)?;
        let name = args.get_single::<String>("name")?;
        let symbol = args.get_single::<String>("symbol")?;
        let curve_type = args.get_single::<u8>("curve_type")?;

        env.set_gas(100_000_000_000); // 100 CSPR
        let launch_id = token_factory.create_launch(
            name.clone(),
            symbol.clone(),
            curve_type,
            None,
            None,
            None,
        );
        
        println!("Launch created successfully!");
        println!("Launch ID: {}", launch_id);
        println!("Token: {}", name);
        println!("Symbol: {}", symbol);
        Ok(())
    }
}

impl ScenarioMetadata for CreateLaunchScenario {
    const NAME: &'static str = "create-launch";
    const DESCRIPTION: &'static str = "Creates a new token launch on the launchpad";
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
        .about("CLI tool for Casper DEX and Launchpad smart contracts")
        // Deploy script (only one allowed by Odra)
        .deploy(DeployNewScript)
        // DEX Contract references
        .contract::<Factory>()
        .contract::<PairFactory>()
        .contract::<Router>()
        .contract::<LpToken>()
        .contract::<EctoToken>()
        .contract::<UsdcToken>()
        .contract::<WethToken>()
        .contract::<WbtcToken>()
        // Launchpad Contract references
        .contract::<TokenFactory>()
        // Scenarios (use scenarios for additional deployments)
        .scenario(CreatePairScenario)
        .scenario(CreateLaunchScenario)
        .scenario(DeployLaunchpadScenario)
        .build()
        .run();
}

