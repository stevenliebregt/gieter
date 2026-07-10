use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file '{path}': {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("environment variable '{0}' referenced by config is not set")]
    MissingEnv(String),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    #[serde(default, rename = "emitter")]
    pub emitters: Vec<EmitterConfig>,
}

impl Config {
    pub fn from_toml_str(src: &str) -> Result<Config, ConfigError> {
        Ok(toml::from_str(src)?)
    }

    pub fn from_path(path: &std::path::Path) -> Result<Config, ConfigError> {
        let str = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Config::from_toml_str(&str)
    }

    pub fn resolve_env(&mut self) -> Result<(), ConfigError> {
        self.database.url = resolve_env_string(&self.database.url)?;
        Ok(())
    }
}

fn resolve_env_string(raw: &str) -> Result<String, ConfigError> {
    if let Some(var) = raw.strip_prefix("env:") {
        return std::env::var(var).map_err(|_| ConfigError::MissingEnv(var.to_string()));
    }

    if let (Some(rest), true) = (raw.strip_prefix("${"), raw.ends_with('}')) {
        let var = &rest[..rest.len() - 1];
        return std::env::var(var).map_err(|_| ConfigError::MissingEnv(var.to_string()));
    }

    Ok(raw.to_string())
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_schemas")]
    pub schemas: Vec<String>,
    #[serde(default)]
    pub exclude_tables: Vec<String>,
}

fn default_schemas() -> Vec<String> {
    vec!["public".into()]
}

#[derive(Debug, Deserialize)]
pub struct EmitterConfig {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub out_dir: Option<PathBuf>,
    #[serde(flatten)]
    pub options: toml::Table,
}
