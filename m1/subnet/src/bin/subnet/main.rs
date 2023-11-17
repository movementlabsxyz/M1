pub mod genesis;
pub mod vm_id;

use std::io;

use avalanche_types::{subnet as avasubnet};
use clap::{crate_version, Command};
use subnet::run_subnet;

pub const APP_NAME: &str = "subnet";

#[tokio::main]
async fn main() -> io::Result<()> {
    let matches = Command::new(APP_NAME)
        .version(crate_version!())
        .about("Subnet")
        .subcommands(vec![genesis::command(), vm_id::command()])
        .get_matches();

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    match matches.subcommand() {
        Some((genesis::NAME, sub_matches)) => {
            let data = sub_matches.get_one::<String>("DATA").expect("required");
            let genesis = data;
            println!("{genesis}");

            Ok(())
        }

        Some((vm_id::NAME, sub_matches)) => {
            let vm_name = sub_matches.get_one::<String>("VM_NAME").expect("required");
            let id = avasubnet::vm_name_to_id(vm_name)?;
            println!("{id}");

            Ok(())
        }

        _ => {
            log::info!("starting subnet");
            run_subnet().await?;
            
            Ok(())
        }
    }
}
