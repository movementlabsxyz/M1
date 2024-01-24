use util::{
    service::Service,
    util::util::patterns::constructor::ConstructorOperations
};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Service;
    type Config = Config;

    fn default() -> Self::Artifact {

        Service::foreground(
            "testnet".to_string(), 
            r#"
            echo $MOVEMENT_DIR
            $MOVEMENT_DIR/bin/avalanchego --fuji --track-subnets=
            "#.to_string(), 
            vec![]
        )

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        Self::default()
    }

    fn from_config(_ : &Self::Config) -> Self::Artifact {
        Self::default()
    }

}
