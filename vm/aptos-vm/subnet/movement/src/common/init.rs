// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::key_rotation::lookup_address,
    common::{
        types::{
            account_address_from_public_key, CliCommand, CliConfig, CliError, CliTypedResult,
            ConfigSearchMode, EncodingOptions, PrivateKeyInputOptions, ProfileConfig,
            ProfileOptions, PromptOptions, RngArgs, DEFAULT_PROFILE,
        },
        utils::{fund_account, prompt_yes_with_override, read_line, wait_for_transactions},
    },
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, ValidCryptoMaterialStringExt};
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode},
    error::{AptosErrorResponse, RestError},
};
use async_trait::async_trait;
use clap::Parser;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

// 
const SEED_NODE_1_REST : &str = "https://seed-node1.movementlabs.xyz";

/// 1 APT (might not actually get that much, depending on the faucet)
const NUM_DEFAULT_OCTAS: u64 = 100000000;

/// Tool to initialize current directory for the aptos tool
///
/// Configuration will be pushed into .aptos/config.yaml
#[derive(Debug, Parser)]
pub struct InitTool {
    /// Network to use for default settings
    ///
    /// If custom `rest_url` and `faucet_url` are wanted, use `custom`
    #[clap(long)]
    pub network: Option<Network>,

    /// URL to a fullnode on the network
    #[clap(long)]
    pub rest_url: Option<Url>,

    /// URL for the Faucet endpoint
    #[clap(long)]
    pub faucet_url: Option<Url>,

    /// Whether to skip the faucet for a non-faucet endpoint
    #[clap(long)]
    pub skip_faucet: bool,

    #[clap(flatten)]
    pub rng_args: RngArgs,
    #[clap(flatten)]
    pub(crate) private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
}

#[async_trait]
impl CliCommand<()> for InitTool {
    fn command_name(&self) -> &'static str {
        "MovementInit"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let mut config = if CliConfig::config_exists(ConfigSearchMode::CurrentDir) {
            CliConfig::load(ConfigSearchMode::CurrentDir)?
        } else {
            CliConfig::default()
        };

        let profile_name = self
            .profile_options
            .profile_name()
            .unwrap_or(DEFAULT_PROFILE);

        // Select profile we're using
        let mut profile_config = if let Some(profile_config) = config.remove_profile(profile_name) {
            prompt_yes_with_override(&format!("Movement already initialized for profile {}, do you want to overwrite the existing config?", profile_name), self.prompt_options)?;
            profile_config
        } else {
            ProfileConfig::default()
        };

        eprintln!("Configuring for profile {}", profile_name);

        // Choose a network
        let network = if let Some(network) = self.network {
            eprintln!("Configuring for network {:?}", network);
            network
        } else {
            eprintln!(
                "Choose network from [devnet, testnet, mainnet, local, custom | defaults to devnet]"
            );
            let input = read_line("network")?;
            let input = input.trim();
            if input.is_empty() {
                eprintln!("No network given, using devnet...");
                Network::Devnet
            } else {
                Network::from_str(input)?
            }
        };

        // Ensure that there is at least a REST URL set for the network
        match network {
            Network::Mainnet => {
                profile_config.rest_url =
                    Some(SEED_NODE_1_REST.to_string());
                profile_config.faucet_url = 
                    Some(SEED_NODE_1_REST.to_string());
            },
            Network::Testnet => {
                profile_config.rest_url =
                    Some(SEED_NODE_1_REST.to_string());
                profile_config.faucet_url =
                    Some(SEED_NODE_1_REST.to_string());
            },
            Network::Devnet => {
                profile_config.rest_url = Some(SEED_NODE_1_REST.to_string());
                profile_config.faucet_url = Some(SEED_NODE_1_REST.to_string());
            },
            Network::Local => {
                profile_config.rest_url = Some("http://localhost:8080".to_string());
                profile_config.faucet_url = Some("http://localhost:8081".to_string());
            },
            Network::Custom => self.custom_network(&mut profile_config)?,
        }

        // Private key
        let private_key = if let Some(private_key) = self
            .private_key_options
            .extract_private_key_cli(self.encoding_options.encoding)?
        {
            eprintln!("Using command line argument for private key");
            private_key
        } else {
            eprintln!("Enter your private key as a hex literal (0x...) [Current: {} | No input: Generate new key (or keep one if present)]", profile_config.private_key.as_ref().map(|_| "Redacted").unwrap_or("None"));
            let input = read_line("Private key")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(private_key) = profile_config.private_key {
                    eprintln!("No key given, keeping existing key...");
                    private_key
                } else {
                    eprintln!("No key given, generating key...");
                    self.rng_args
                        .key_generator()?
                        .generate_ed25519_private_key()
                }
            } else {
                Ed25519PrivateKey::from_encoded_string(input)
                    .map_err(|err| CliError::UnableToParse("Ed25519PrivateKey", err.to_string()))?
            }
        };
        let public_key = private_key.public_key();

        let client = aptos_rest_client::Client::new(
            Url::parse(
                profile_config
                    .rest_url
                    .as_ref()
                    .expect("Must have rest client as created above"),
            )
            .map_err(|err| CliError::UnableToParse("rest_url", err.to_string()))?,
        );

        // lookup the address from onchain instead of deriving it
        // if this is the rotated key, deriving it will outputs an incorrect address
        let derived_address = account_address_from_public_key(&public_key);
        let address = lookup_address(&client, derived_address, false).await?;

        profile_config.private_key = Some(private_key);
        profile_config.public_key = Some(public_key);
        profile_config.account = Some(address);

        // Create account if it doesn't exist (and there's a faucet)
        // Check if account exists
        let account_exists = match client.get_account(address).await {
            Ok(_) => true,
            Err(err) => {
                if let RestError::Api(AptosErrorResponse {
                    error:
                        AptosError {
                            error_code: AptosErrorCode::ResourceNotFound,
                            ..
                        },
                    ..
                })
                | RestError::Api(AptosErrorResponse {
                    error:
                        AptosError {
                            error_code: AptosErrorCode::AccountNotFound,
                            ..
                        },
                    ..
                }) = err
                {
                    false
                } else {
                    return Err(CliError::UnexpectedError(format!(
                        "Failed to check if account exists: {:?}",
                        err
                    )));
                }
            },
        };

        // If you want to create a private key, but not fund the account, skipping the faucet is still possible
        let maybe_faucet_url = if self.skip_faucet {
            None
        } else {
            profile_config.faucet_url.as_ref()
        };

        if let Some(faucet_url) = maybe_faucet_url {
            if account_exists {
                eprintln!("Account {} has been already found onchain", address);
            } else {
                eprintln!(
                    "Account {} doesn't exist, creating it and funding it with {} Octas",
                    address, NUM_DEFAULT_OCTAS
                );
                let hashes = fund_account(
                    Url::parse(faucet_url)
                        .map_err(|err| CliError::UnableToParse("rest_url", err.to_string()))?,
                    NUM_DEFAULT_OCTAS,
                    address,
                )
                .await?;
                wait_for_transactions(&client, hashes).await?;
                eprintln!("Account {} funded successfully", address);
            }
        } else if account_exists {
            eprintln!("Account {} has been already found onchain", address);
        } else if network == Network::Mainnet {
            eprintln!("Account {} does not exist, you will need to create and fund the account by transferring funds from another account", address);
        } else {
            eprintln!("Account {} has been initialized locally, but you must transfer coins to it to create the account onchain", address);
        }

        // Ensure the loaded config has profiles setup for a possible empty file
        if config.profiles.is_none() {
            config.profiles = Some(BTreeMap::new());
        }
        config
            .profiles
            .as_mut()
            .expect("Must have profiles, as created above")
            .insert(profile_name.to_string(), profile_config);
        config.save()?;
        eprintln!("\n---\nMovement CLI is now set up for account {} as profile {}!  Run `movement --help` for more information about commands", address, self.profile_options.profile_name().unwrap_or(DEFAULT_PROFILE));
        Ok(())
    }
}

impl InitTool {
    /// Custom network created, which requires a REST URL
    fn custom_network(&self, profile_config: &mut ProfileConfig) -> CliTypedResult<()> {
        // Rest Endpoint
        let rest_url = if let Some(ref rest_url) = self.rest_url {
            eprintln!("Using command line argument for rest URL {}", rest_url);
            Some(rest_url.to_string())
        } else {
            let current = profile_config.rest_url.as_deref();
            eprintln!(
                    "Enter your rest endpoint [Current: {} | No input: Exit (or keep the existing if present)]",
                    current.unwrap_or("None"),
                );
            let input = read_line("Rest endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(current) = current {
                    eprintln!("No rest url given, keeping the existing url...");
                    Some(current.to_string())
                } else {
                    eprintln!("No rest url given, exiting...");
                    return Err(CliError::AbortedError);
                }
            } else {
                Some(
                    reqwest::Url::parse(input)
                        .map_err(|err| CliError::UnableToParse("Rest Endpoint", err.to_string()))?
                        .to_string(),
                )
            }
        };
        profile_config.rest_url = rest_url;

        // Faucet Endpoint
        let faucet_url = if self.skip_faucet {
            eprintln!("Not configuring a faucet because --skip-faucet was provided");
            None
        } else if let Some(ref faucet_url) = self.faucet_url {
            eprintln!("Using command line argument for faucet URL {}", faucet_url);
            Some(faucet_url.to_string())
        } else {
            let current = profile_config.faucet_url.as_deref();
            eprintln!(
                    "Enter your faucet endpoint [Current: {} | No input: Skip (or keep the existing one if present) | 'skip' to not use a faucet]",
                    current
                        .unwrap_or("None"),
                );
            let input = read_line("Faucet endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(current) = current {
                    eprintln!("No faucet url given, keeping the existing url...");
                    Some(current.to_string())
                } else {
                    eprintln!("No faucet url given, skipping faucet...");
                    None
                }
            } else if input.to_lowercase() == "skip" {
                eprintln!("Skipping faucet...");
                None
            } else {
                Some(
                    reqwest::Url::parse(input)
                        .map_err(|err| CliError::UnableToParse("Faucet Endpoint", err.to_string()))?
                        .to_string(),
                )
            }
        };
        profile_config.faucet_url = faucet_url;
        Ok(())
    }
}

/// A simplified list of all networks supported by the CLI
///
/// Any command using this, will be simpler to setup as profiles
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Local,
    Custom,
}

impl FromStr for Network {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().trim() {
            "mainnet" => Self::Mainnet,
            "testnet" => Self::Testnet,
            "devnet" => Self::Devnet,
            "local" => Self::Local,
            "custom" => Self::Custom,
            str => {
                return Err(CliError::CommandArgumentError(format!(
                    "Invalid network {}.  Must be one of [devnet, testnet, mainnet, local, custom]",
                    str
                )));
            },
        })
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::Devnet
    }
}
