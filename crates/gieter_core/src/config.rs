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
        // Options are source/emitter-specific (url, command, credentials, ...), so
        // resolve env references on every string, recursing into arrays and nested tables.
        resolve_table(&mut self.source.options)?;
        for emitter in &mut self.emitters {
            resolve_table(&mut emitter.options)?;
        }
        Ok(())
    }
}

fn resolve_table(table: &mut toml::Table) -> Result<(), ConfigError> {
    for (_key, value) in table.iter_mut() {
        resolve_value(value)?;
    }
    Ok(())
}

fn resolve_value(value: &mut toml::Value) -> Result<(), ConfigError> {
    match value {
        toml::Value::String(raw) => *raw = resolve_env_string(raw.as_str())?,
        toml::Value::Array(items) => {
            for item in items.iter_mut() {
                resolve_value(item)?;
            }
        }
        toml::Value::Table(table) => resolve_table(table)?,
        _ => {}
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_env_covers_emitter_options_and_array_entries() {
        unsafe {
            std::env::set_var("GIETER_TEST_SCRIPT", "run.py");
            std::env::set_var("GIETER_TEST_TOKEN", "secret");
        }

        let mut config = Config::from_toml_str(
            r#"
[source]
type = "external"
command = ["python3", "env:GIETER_TEST_SCRIPT"]

[[emitter]]
type = "external"
token = "env:GIETER_TEST_TOKEN"
"#,
        )
        .unwrap();

        config.resolve_env().unwrap();

        let command = config
            .source
            .options
            .get("command")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(command[1].as_str(), Some("run.py"));
        assert_eq!(
            config.emitters[0].options.get("token").unwrap().as_str(),
            Some("secret")
        );
    }
}
