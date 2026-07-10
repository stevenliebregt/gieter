use crate::ir::Catalog;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    #[error("invalid options for emitter '{emitter}': {message}")]
    Options { emitter: String, message: String },
}

#[derive(Debug)]
pub struct EmitterOutput {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub contents: String,
}

pub trait Emitter {
    fn name(&self) -> &str;

    fn emit(&self, catalog: &Catalog, options: &toml::Table) -> Result<EmitterOutput, EmitError>;
}
