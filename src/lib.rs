use std::fs;
use std::path::PathBuf;

pub mod apis;
pub mod arduino_secret;
pub mod db;
pub mod file_size;
pub mod filesystem;
pub mod macros;
pub mod models;
pub mod rolling_secret;
pub mod traits;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::apis::NeptisError;
    
    pub use crate::apis::prelude::*;
    pub use crate::arduino_secret::*;
    pub use crate::db::prelude::*;
    pub use crate::file_size::*;
    pub use crate::filesystem::*;
    
    pub use crate::models::*;
    pub use crate::rolling_secret;
    pub use crate::traits::*;
}

pub fn get_working_dir() -> PathBuf {
    let b_dir = dirs_next::home_dir()
        .map(|x| x.join(".neptis"))
        .expect("Failed to get home directory!");
    if !b_dir.exists() {
        fs::create_dir_all(&b_dir).expect("Failed to create Neptis directory!");
    }
    b_dir
}
