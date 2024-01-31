use async_trait::async_trait;
use crate::builder::script::ScriptPart;
use crate::artifact::ArtifactDependency;
use crate::movement_dir::MovementDir;

#[derive(Debug, Clone)]
pub struct Service {
    pub name : String,
    pub executor : Executor,
    pub artifact_dependencies : Vec<ArtifactDependency>,
}

impl Service {
        
    pub fn new(
        name : String,
        executor : Executor,
        artifact_dependencies : Vec<ArtifactDependency>,
    ) -> Self {
        Self {
            name,
            executor,
            artifact_dependencies,
        }
    }

    pub fn foreground(
        name : String,
        script : String, 
        artifact_dependencies : Vec<ArtifactDependency>,
    ) -> Self {
        Self::new(
            name,
            Executor::Scripts(
                Scripts {
                    start_script : script.into(),
                    stop_script : "echo 'Cannot stop a foreground service. If it is running, use ctrl-c to stop it.'".to_string().into(),
                    status_script : "echo 'Cannot check status of a foreground service. It is either running in the foreground or not.'".to_string().into(),
                }
            ),
            artifact_dependencies,
        )
    }
    
}

#[async_trait::async_trait]
impl ServiceOperations for Service {
    
    async fn get_name(&self) -> String {
        self.name.clone()
    }
    
    async fn start(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.executor.start(movement_dir).await
    }
    
    async fn stop(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.executor.stop(movement_dir).await
    }
    
    async fn status(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.executor.status(movement_dir).await
    }
    
}


#[async_trait]
pub trait ServiceOperations {
    
    async fn get_name(&self) -> String;
    
    async fn start(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error>;
    
    async fn stop(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error>;
    
    async fn status(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error>;
    
}

#[derive(Debug, Clone)]
pub enum Executor {
    Scripts(Scripts),
    Noop,
}

#[async_trait::async_trait]
impl ServiceOperations for Executor {

    async fn get_name(&self) -> String {
        match self {
            Executor::Scripts(scripts) => scripts.get_name().await,
            Executor::Noop => "noop".to_string(),
        }
    }
    
    async fn start(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        match self {
            Executor::Scripts(scripts) => scripts.start(movement_dir).await,
            Executor::Noop => Ok(()),
        }
    }
    
    async fn stop(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        match self {
            Executor::Scripts(scripts) => scripts.stop(movement_dir).await,
            Executor::Noop => Ok(()),
        }
    }
    
    async fn status(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        match self {
            Executor::Scripts(scripts) => scripts.status(movement_dir).await,
            Executor::Noop => Ok(()),
        }
    }

}

#[derive(Debug, Clone)]
pub struct Scripts {
    pub start_script : ScriptPart,
    pub stop_script : ScriptPart,
    pub status_script : ScriptPart,
}

#[async_trait::async_trait]
impl ServiceOperations for Scripts {

    async fn get_name(&self) -> String {
        "scripts".to_string()
    }
    
    async fn start(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.start_script.exec(movement_dir).await
    }
    
    async fn stop(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.stop_script.exec(movement_dir).await
    }
    
    async fn status(&self, movement_dir : &MovementDir) -> Result<(), anyhow::Error> {
        self.status_script.exec(movement_dir).await
    }

}