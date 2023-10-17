// aptos execution
use super::initialized::Initialized;
use super::uninitialized::Uninitialized;
use std::{sync::Arc, f32::consts::E};
use tokio::sync::RwLock;
use tonic::async_trait;
use crate::executor::executor::{Initialized, Uninitialized};
use crate::util::types::block::Block;
use crate::state::avalanche::avalanche_block::AvalancheBlock;


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

impl <S : Uninitialized> AvalancheAptos<S> {
    
    async fn initialize(self) -> Result<AvalancheAptos<Initialized>, anyhow::Error> {
        Ok(AvalancheAptos::new(self.state.initialize().await?))
    }

}

impl <S : Initialized> AvalancheAptos<S> {

    async fn build_block(&self) -> Result<AvalancheBlock, anyhow::Error> {
        self.state.build_block().await
    }

}

use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub enum AvalancheAptosRuntime {
    Uninitialized(AvalancheAptos<Uninitialized>),
    Initialized(AvalancheAptos<Initialized>),
}

#[derive(Debug, Clone)]
pub struct AvalancheAptosVm {
    pub runtime: Arc<RwLock<AvalancheAptosRuntime>>,
}

// BasicOperations
impl AvalancheAptosVm {
    pub fn new(initial_runtime: AvalancheAptosRuntime) -> Self {
        AvalancheAptosVm {
            runtime: Arc::new(RwLock::new(initial_runtime)),
        }
    }

    pub fn set_runtime(&self, new_runtime: AvalancheAptosRuntime) {
        if let Ok(mut write_lock) = self.runtime.write() {
            *write_lock = new_runtime;
        }
    }

    pub fn get_runtime(&self) -> AvalancheAptosRuntime {
        if let Ok(read_lock) = self.runtime.read() {
            read_lock.clone()
        } else {
            // Handle read lock error (e.g., by returning a default value)
            AvalancheAptosRuntime::Uninitialized(AvalancheAptos::default())
        }
    }
}

// Initialization
impl AvalancheAptosVm {
    pub async fn initialize(&self) -> Result<(), anyhow::Error> {
        match self.get_runtime() {
            AvalancheAptosRuntime::Uninitialized(uninitialized) => {
                let initialized = uninitialized.initialize().await?;
                self.set_runtime(AvalancheAptosRuntime::Initialized(initialized));
                Ok(())
            },
            _ => Err(anyhow::anyhow!("AvalancheAptosVm is already initialized")),
        }
    }
}

// BlockBuilding
impl AvalancheAptosVm {

    pub async fn build_block(&self) -> Result<AvalancheBlock, anyhow::Error> {
        match self.get_runtime() {
            AvalancheAptosRuntime::Initialized(initialized) => {
                let block = initialized.build_block().await?;
                Ok(block)
            },
            _ => Err(anyhow::anyhow!("AvalancheAptosVm is not initialized")),
        }
    }

}