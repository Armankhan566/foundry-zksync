use std::fmt::Debug;
use std::process;

use super::build::CoreBuildArgs;
use super::zksolc::{ZkSolc, ZkSolcOpts};
use super::zksolc_manager::{ZkSolcManagerBuilder, ZkSolcManagerOpts};
use crate::cmd::{Cmd, LoadConfig};
use clap::Parser;
use foundry_config::{
    figment::{
        self,
        error::Kind::InvalidType,
        value::{Dict, Map, Value},
        Metadata, Profile, Provider,
    },
    Config,
};
use serde::Serialize;

foundry_config::merge_impl_figment_convert!(ZkBuildArgs, args);

#[derive(Debug, Clone, Parser, Serialize, Default)]
#[clap(next_help_heading = "ZkBuild options", about = None)]
pub struct ZkBuildArgs {
    /// Contract filename from project src/ ex: 'Contract.sol'
    #[clap(
        help_heading = "Contract Name",
        help = "Contract filename from project src/ ex: 'Contract.sol'",
        value_name = "CONTRACT_FILENAME"
    )]
    pub contract_name: String,
    /// Specify the solc version, or a path to a local solc, to build with.
    ///
    /// Valid values are in the format `x.y.z`, `solc:x.y.z` or `path/to/solc`.
    #[clap(
        help_heading = "ZkSync Compiler options",
        value_name = "ZK_SOLC_VERSION",
        long = "use-zksolc",
        default_value = Some("v1.3.9")
    )]
    #[serde(skip)]
    pub use_zksolc: Option<String>,

    #[clap(
        help_heading = "ZkSync Compiler options",
        help = "Compile contract with in system mode",
        long = "is-system",
        value_name = "SYSTEM_MODE"
    )]
    pub is_system: bool,

    #[clap(flatten)]
    #[serde(flatten)]
    pub args: CoreBuildArgs,
}

impl Cmd for ZkBuildArgs {
    type Output = String;

    fn run(self) -> eyre::Result<String> {
        let config = self.try_load_config_emit_warnings()?;
        let project = config.project()?;

        let zksolc_manager_opts = ZkSolcManagerOpts { version: self.use_zksolc.unwrap() };

        let zksolc_manager_builder = ZkSolcManagerBuilder::new(zksolc_manager_opts);
        let zksolc_manager = zksolc_manager_builder.build();

        match zksolc_manager {
            Ok(zksolc_manager) => {
                if let Err(err) = zksolc_manager.clone().check_setup_compilers_dir() {
                    eprintln!("Failed to setup compilers directory: {}", err);
                    process::exit(1);
                }

                if !zksolc_manager.exists() {
                    println!("Downloading zksolc compiler");

                    match zksolc_manager.clone().download() {
                        Ok(zksolc_manager) => zksolc_manager,
                        Err(err) => {
                            eprintln!("Failed to download the file: {}", err);
                            process::exit(1);
                        }
                    }
                }

                println!("Compiling smart contracts...");

                let zksolc_opts = ZkSolcOpts {
                    compiler_path: zksolc_manager.get_full_compiler_path(),
                    // config: &config,
                    is_system: self.is_system,
                    // force_evmla: todo!(),
                    project: &project,
                    config: &config,
                    contract_name: self.contract_name, // contracts_path: todo!(),
                };

                let mut zksolc = ZkSolc::new(zksolc_opts);

                if let Err(err) = zksolc.parse_json_input() {
                    eprintln!("Failed to parse json input for zksolc compiler: {}", err);
                    process::exit(1);
                }

                match zksolc.compile() {
                    Ok(_) => println!("Compiled Successfully"),
                    Err(err) => {
                        eprintln!("Failed to compile smart contracts with zksolc: {}", err);
                        process::exit(1);
                    }
                }
            }
            Err(e) => eprintln!("Error building zksolc_manager: {}", e),
        }

        Ok("".to_owned())
    }
}

// Make this args a `figment::Provider` so that it can be merged into the `Config`
impl Provider for ZkBuildArgs {
    fn metadata(&self) -> Metadata {
        Metadata::named("Build Args Provider")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let value = Value::serialize(self)?;
        let error = InvalidType(value.to_actual(), "map".into());
        let dict = value.into_dict().ok_or(error)?;

        // if self.names {
        //     dict.insert("names".to_string(), true.into());
        // }

        // if self.sizes {
        //     dict.insert("sizes".to_string(), true.into());
        // }

        Ok(Map::from([(Config::selected_profile(), dict)]))
    }
}