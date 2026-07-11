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
    pub source: SourceConfig,
    #[serde(default, rename = "emitter")]
    pub emitters: Vec<EmitterConfig>,
    /// Directory the config was loaded from; relative output dirs resolve against it.
    #[serde(skip)]
    pub base_dir: PathBuf,
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
        let mut config = Config::from_toml_str(&str)?;
        if let Some(parent) = path.parent() {
            config.base_dir = parent.to_path_buf();
        }
        Ok(config)
    }

    pub fn resolve_env(&mut self) -> Result<(), ConfigError> {
        // Source connection options (url, command, ...) are source-specific, so
        // resolve env references on every string option generically.
        for (_key, value) in self.source.options.iter_mut() {
            if let toml::Value::String(raw) = value {
                *raw = resolve_env_string(raw.as_str())?;
            }
        }
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
pub struct SourceConfig {
    /// Selects which registered source builds this catalog (e.g. "postgres").
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default = "default_schemas")]
    pub schemas: Vec<String>,
    #[serde(default)]
    pub exclude_tables: Vec<String>,
    /// Source-specific connection options (url, command, ...). Handed to the
    /// source's factory in the registry.
    #[serde(flatten)]
    pub options: toml::Table,
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
