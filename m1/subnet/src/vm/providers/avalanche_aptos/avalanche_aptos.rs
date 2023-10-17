// aptos execution
use super::initialized::Initialized;
use super::uninitialized::Uninitialized;
use std::sync::Arc;
use tokio::sync::RwLock;
use avalanche_types::subnet::rpc::snow;

pub trait AvalancheAptosState {}

#[derive(Debug, Clone)]
pub struct AvalancheAptos<S : AvalancheAptosState> {
    pub state: S,
}

impl <S> AvalancheAptos<S> where S : AvalancheAptosState {
    pub fn new(state : S) -> Self {
        Aptos {
            state,
        }
    }
}

/// The AvalancheAptosRuntime wraps the various states of AvalancheAptos
/// so that type-state restrictions can be enforced at runtime instead of at compile time.
#[derive(Debug, Clone)]
pub enum AvalancheAptosRuntime {
    Uninitialized(AvalancheAptos<Uninitialized>),
    Initialized(AvalancheAptos<Initialized>),
}

#[derive(Debug, Clone)]
pub struct AvalancheAptosVm {
    pub runtime: Arc<RwLock<AvalancheAptosRuntime>>,

    // todo: we're not really doing anything with this at the moment
    pub snow_state: Arc<RwLock<snow::State>>,
}

impl AvalancheAptosVm {
    pub fn new(initial_runtime: AvalancheAptosRuntime) -> Self {
        AvalancheAptosVm {
            runtime: Arc::new(RwLock::new(initial_runtime)),
            snow_state: Arc::new(RwLock::new(
                snow::State::Initializing
            )),
        }
    }

    pub async fn set_runtime(
        &self, 
        new_runtime: AvalancheAptosRuntime
    ) -> Result<(), anyhow::Error> {
        let runtime = self.runtime.write().await?;
        *runtime = new_runtime;
        Ok(())
    }
    
    pub async fn get_runtime(&self) -> Result<AvalancheAptosRuntime, anyhow::Error> {
        let runtime = self.runtime.read().await?;
        Ok(runtime.clone())
    }
    

    pub async fn set_snow_state(
        &self, 
        new_snow_state: snow::State
    ) -> Result<(), anyhow::Error> {
        let snow_state = self.snow_state.write().await?;
        *snow_state = new_snow_state;
    }

    pub async fn get_snow_state(&self) -> Result<snow::State, anyhow::Error> {
        let snow_state = self.snow_state.read().await?;
        Ok(snow_state.clone())
    }

}
