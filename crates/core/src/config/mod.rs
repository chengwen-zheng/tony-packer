use std::collections::HashMap;

use config_regex::ConfigRegex;
use persistent_cache::PersistentCacheConfig;
use serde::{Deserialize, Serialize};

pub mod config_regex;
pub mod custom;
pub mod external;

#[derive(Clone)]
pub struct Config {
    pub input: HashMap<String, String>,
    pub output: OutputConfig,
    pub root: String,
    pub persistent_cache: Box<persistent_cache::PersistentCacheConfig>,
    pub mode: Mode,
    pub record: bool,
    pub custom: Box<HashMap<String, String>>,
    pub external: Vec<ConfigRegex>,
    pub resolve: ResolveConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct ResolveConfig {
    pub alias: HashMap<String, String>,
    pub main_fields: Vec<String>,
    pub main_files: Vec<String>,
    pub extensions: Vec<String>,
    pub conditions: Vec<String>,
    pub symlinks: bool,
    pub strict_exports: bool,
    pub auto_external_failed_resolve: bool,
}

impl Default for ResolveConfig {
    fn default() -> Self {
        Self {
            alias: HashMap::new(),
            main_fields: vec![
                String::from("browser"),
                String::from("exports"),
                String::from("module"),
                String::from("main"),
                String::from("jsnext:main"),
                String::from("jsnext"),
            ],
            main_files: vec![String::from("index")],
            extensions: vec![
                String::from("tsx"),
                String::from("ts"),
                String::from("mts"),
                String::from("cts"),
                String::from("jsx"),
                String::from("mjs"),
                String::from("js"),
                String::from("cjs"),
                String::from("json"),
                String::from("html"),
                String::from("css"),
            ],
            conditions: vec![
                String::from("development"),
                String::from("production"),
                String::from("module"),
            ],
            symlinks: true,
            strict_exports: false,
            auto_external_failed_resolve: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let root = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();

        Self {
            input: HashMap::from([("index".to_string(), "./index.html".to_string())]),
            root: root.clone(),
            output: OutputConfig::default(),
            mode: Mode::Development,
            resolve: ResolveConfig::default(),
            // define: HashMap::new(),
            external: Default::default(),
            // runtime: Default::default(),
            // script: Default::default(),
            // css: Default::default(),
            // html: Box::default(),
            // assets: Default::default(),
            // sourcemap: Default::default(),
            // partial_bundling: PartialBundlingConfig::default(),
            // lazy_compilation: true,
            // core_lib_path: None,
            // tree_shaking: true,
            // minify: Box::new(BoolOrObj::Bool(true)),
            // preset_env: Box::<PresetEnvConfig>::default(),
            record: false,
            // progress: true,
            persistent_cache: Box::new(PersistentCacheConfig::Bool(false)),
            // comments: Box::default(),
            custom: Box::<HashMap<String, String>>::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "production")]
    Production,
}
pub mod persistent_cache;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputConfig {
    pub path: String,
    pub public_path: String,
    pub entry_filename: String,
    pub filename: String,
    pub assets_filename: String,
    //   pub target_env: TargetEnv,
    //   pub format: ModuleFormat,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            entry_filename: "[entryName].[ext]".to_string(),
            // [resourceName].[contentHash].[ext]
            filename: "[resourceName].[ext]".to_string(),
            // [resourceName].[contentHash].[ext]
            assets_filename: "[resourceName].[ext]".to_string(),
            public_path: "/".to_string(),
            path: "dist".to_string(),
            //   target_env: TargetEnv::default(),
            //   format: ModuleFormat::default(),
        }
    }
}
