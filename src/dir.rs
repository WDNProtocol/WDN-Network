use std::{
    env, fs,
    path::{self, Path},
};

#[derive(Debug, PartialEq)]
// data directories
pub struct Directories {
    /// Base dir
    pub base: String,
    /// Database dir
    pub db: String,
}

impl Directories {
    pub fn new(base_path: String) -> Self {
        Directories {
            db: Path::new(&base_path)
                .join("database")
                .to_str()
                .unwrap()
                .to_string(),
            base: base_path,
        }
    }
}

impl Default for Directories {
    fn default() -> Self {
        let exe = env::current_exe().expect("Get executable failed");
        let dir = exe.parent().expect("Get executable failed");
        let data_dir = dir.join("node");

        Directories {
            base: data_dir.to_str().unwrap().to_string(),
            db: data_dir.join("database").to_str().unwrap().to_string(),
        }
    }
}

impl Directories {
    pub fn create_dirs(&self) -> Result<(), String> {
        fs::create_dir_all(&self.base).map_err(|e| e.to_string())?;
        fs::create_dir_all(&self.db).map_err(|e| e.to_string())?;
        Ok(())
    }
}
