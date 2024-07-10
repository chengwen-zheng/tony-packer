use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub struct Config {
    pub input: HashMap<String, String>,
    pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "production")]
    Production,
}
