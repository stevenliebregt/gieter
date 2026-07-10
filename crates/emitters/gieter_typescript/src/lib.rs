use crate::options::Options;
use gieter_core::emitter::{EmitError, Emitter, EmitterOutput};
use gieter_core::ir::Catalog;

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
