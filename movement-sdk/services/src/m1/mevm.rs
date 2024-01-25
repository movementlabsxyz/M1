use util::{
    service::Service,
    util::util::patterns::constructor::ConstructorOperations,
    util::util::version
};
use artifacts::known_artifacts::m1::m1_with_submodules;

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Service;
    type Config = Config;

    fn default() -> Self::Artifact {

       Self::default_with_version(&version::Version::Latest)

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        
        Service::foreground(
            "mevm".to_string(), 
            r#"
            cd $MOVEMENT_DIR/src/m1-with-submodules/m1/infrastructure/evm-rpc
            npm install
            npm run start
            "#.to_string(), 
            vec![
                m1_with_submodules::Constructor::default_with_version(
                    version
                ).into()
            ]
        )

    }

    fn from_config(version : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
        Self::default_with_version(version)
    }

}
