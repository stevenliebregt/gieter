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
}

pub trait Source {
    /// Introspects the given schemas into a [`Catalog`].
    fn introspect(&self, schemas: &[String]) -> Result<Catalog, SourceError>;
}

/// Builds a source from its connection options. The options are the source's own
/// keys from the config (url, command, ...); scope (schemas) is passed later to
/// [`Source::introspect`].
pub type SourceFactory = Box<dyn Fn(&toml::Table) -> Result<Box<dyn Source>, SourceError>>;

/// Maps a config `type` onto the factory that builds that source. Registering a
/// factory is the extension point: a custom binary can add an in-process source
/// without forking, and the built-in `postgres` factory is registered the same way.
pub struct SourceRegistry {
    factories: HashMap<String, SourceFactory>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        SourceRegistry {
            factories: HashMap::new(),
        }
    }

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

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
