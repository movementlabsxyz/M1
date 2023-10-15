use tonic::async_trait;


#[async_trait]
pub trait Initializing {
    // pass
}

#[async_trait]
pub trait StateSyncing {
    // this will be unimplemented
}

#[async_trait]
pub trait Bootstrapping {
    // methods specific to the initialized state
}

#[async_trait]
pub trait Running {
    // methods specific to the initialized state
}

pub struct Vm<S> {
    state : S
}

impl <S> Vm<S> {
    pub fn new(state : S) -> Self {
        Vm {
            state,
        }
    }
}

impl<S : Uninitialized> Vm<S> {

    async fn initialize(self, config : S::Config) -> Result<Vm<S::Initialized>, anyhow::Error> {
        Ok(Vm::new(
            self.state.initialize(config).await?
        ))
    }

}