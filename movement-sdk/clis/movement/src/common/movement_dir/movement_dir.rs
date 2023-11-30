pub struct MovementDir {
    pub path: PathBuf,
}

impl MovementDir {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }

    pub fn path_str_with_trailing_slash(&self) -> String {
        format!("{}/", self.path_str())
    }

    pub fn path_str_without_trailing_slash(&self) -> String {
        self.path_str().to_string()
    }

}

impl Default for MovementDir {
    fn default() -> Self {
        Self {
            // ! default path is $HOME/.movement
            path : 
        }
    }
}
