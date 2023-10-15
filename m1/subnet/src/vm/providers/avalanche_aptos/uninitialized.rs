// aptos execution
use crate::executor::{
    executor::Executor,
    providers::aptos::{
        self,
        aptos::Aptos,
    }
};
use super::initialized::Initialized;

// avalanche state
use crate::state::avalanche::state::State;
use super::avalanche_aptos::AvalancheAptosState;

#[derive(Debug, Clone)]
pub struct Uninitialized {
    pub executor : Executor<Aptos<aptos::uninitialized::Uninitialized>>,
    pub state: State,
}

impl AvalancheAptosState for Uninitialized {}

impl Uninitialized {

    pub fn new(state : State) -> Self {
        Uninitialized {
            executor: Executor::new(Aptos::new(aptos::uninitialized::Uninitialized::default())),
            state,
        }
    }

    pub async fn initialize(self) -> Result<Initialized, anyhow::Error> {

        let initialized_executor = self.executor.initialize(self.state.config.clone()).await?;

        Ok(Initialized::new(
            initialized_executor,
            self.state,
        ))
    }

}