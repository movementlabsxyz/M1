// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliTypedResult}
};
use async_trait::async_trait;
use crate::common::types::{FaucetOptions, ProfileOptions, RestOptions};
use crate::common::utils::{fund_pub_key, wait_for_transactions};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct FaucetTool {
    #[clap(long)]
    pub_key: String,
    #[clap(flatten)]
    pub(crate) faucet_options: FaucetOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<String> for FaucetTool {
    fn command_name(&self) -> &'static str {
        "Faucet"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let profile = ProfileOptions::default();
        let hashes = fund_pub_key(
            self.faucet_options.faucet_url(&profile)?,
            self.pub_key.clone(),
        ).await?;
        let client = self.rest_options.client_raw(self.faucet_options.faucet_url(&profile)?)?;
        wait_for_transactions(&client, hashes).await?;
        return Ok(format!(
            "Added 1000_000_000 Octas to account {}", self.pub_key
        ));
    }
}

