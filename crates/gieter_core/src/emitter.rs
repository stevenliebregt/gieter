use crate::Error;
use crate::config::EmitterConfig;
use crate::ir::Catalog;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    #[error("invalid options for emitter '{emitter}': {message}")]
    Options { emitter: String, message: String },
    #[error("external plugin error: {0}")]
    External(String),
}

#[derive(Debug)]
pub struct EmitterOutput {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub contents: String,
}

pub trait Emitter {
    fn emit(&self, catalog: &Catalog) -> Result<EmitterOutput, EmitError>;
}

pub type EmitterFactory = Box<dyn Fn(&toml::Table) -> Result<Box<dyn Emitter>, EmitError>>;

#[derive(Default)]
pub struct EmitterRegistry {
    factories: HashMap<String, EmitterFactory>,
}

impl EmitterRegistry {
    pub fn register(
        &mut self,
        ty: impl Into<String>,
        factory: impl Fn(&toml::Table) -> Result<Box<dyn Emitter>, EmitError> + 'static,
    ) {
        self.factories.insert(ty.into(), Box::new(factory));
    }

    pub fn build(&self, config: &EmitterConfig) -> Result<Box<dyn Emitter>, Error> {
        let factory = self
            .factories
            .get(&config.ty)
            .ok_or_else(|| Error::UnknownEmitter(config.ty.clone()))?;
        Ok(factory(&config.options)?)
    }
}
