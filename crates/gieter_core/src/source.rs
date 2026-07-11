use crate::Error;
use crate::config::SourceConfig;
use crate::ir::Catalog;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("database connection failed: {0}")]
    Connect(String),
    #[error("introspection query failed: {0}")]
    Query(String),
    #[error("invalid source configuration: {0}")]
    Config(String),
    #[error("external plugin error: {0}")]
    External(String),
}

pub trait Source {
    /// Introspects the given schemas into a [`Catalog`].
    fn introspect(&self, schemas: &[String]) -> Result<Catalog, SourceError>;
}

/// Construct a source from the connection options.
pub type SourceFactory = Box<dyn Fn(&toml::Table) -> Result<Box<dyn Source>, SourceError>>;

#[derive(Default)]
pub struct SourceRegistry {
    factories: HashMap<String, SourceFactory>,
}

impl SourceRegistry {
    pub fn register(
        &mut self,
        ty: impl Into<String>,
        factory: impl Fn(&toml::Table) -> Result<Box<dyn Source>, SourceError> + 'static,
    ) {
        self.factories.insert(ty.into(), Box::new(factory));
    }

    pub fn build(&self, config: &SourceConfig) -> Result<Box<dyn Source>, Error> {
        let factory = self
            .factories
            .get(&config.ty)
            .ok_or_else(|| Error::UnknownSource(config.ty.clone()))?;
        Ok(factory(&config.options)?)
    }
}
