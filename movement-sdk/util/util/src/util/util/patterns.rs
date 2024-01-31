pub mod constructor {

    use crate::util::util::Version;

    pub trait ConstructorOperations {

        type Artifact;
        type Config;

        fn default_with_version(version : &Version) -> Self::Artifact;

        fn sub_default_with_version<T>(version : &Version) -> Self::Artifact where T : ConstructorOperations<Artifact=Self::Artifact, Config=Self::Config> {
            T::default_with_version(version)
        }
    
        fn default() -> Self::Artifact {
            Self::default_with_version(&Version::Latest)
        }
    
        fn sub_default<T>() -> Self::Artifact where T : ConstructorOperations<Artifact=Self::Artifact, Config=Self::Config> {
            T::default()
        }
    
        fn from_config(version : &Version, config : &Self::Config) -> Self::Artifact;
    
        fn sub_from_config<T>(version : &Version, config : &Self::Config) -> Self::Artifact where T : ConstructorOperations<Artifact=Self::Artifact, Config=Self::Config> {
            T::from_config(version, config)
        }
    
    }

}