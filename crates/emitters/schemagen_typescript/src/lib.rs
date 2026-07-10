use crate::options::Options;
use schemagen_core::emitter::{EmitError, Emitter, EmitterOutput};
use schemagen_core::ir::Catalog;

mod brand;
mod naming;
mod options;
mod render;
mod typemap;

pub struct TypescriptEmitter;

impl Emitter for TypescriptEmitter {
    fn name(&self) -> &str {
        "typescript"
    }

    fn emit(&self, catalog: &Catalog, options: &toml::Table) -> Result<EmitterOutput, EmitError> {
        let options = Options::from_table(options)?;
        render::render(catalog, &options)
    }
}
