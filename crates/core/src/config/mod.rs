use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Config {
    pub input: HashMap<String, String>,
    pub output: String,
    pub root: String,
    pub persistent_cache: Box<persistent_cache::PersistentCacheConfig>,
    pub mode: Mode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "production")]
    Production,
}
pub mod persistent_cache;
