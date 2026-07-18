use crate::options::Options;
use gieter_core::emitter::{EmitError, Emitter, EmitterOutput};
use gieter_core::ir::Catalog;

mod naming;
mod options;
mod render;
mod typemap;

pub struct TypescriptEmitter {
    options: Options,
}

impl Emitter for TypescriptEmitter {
    fn emit(&self, catalog: &Catalog) -> Result<EmitterOutput, EmitError> {
        render::render(catalog, &self.options)
    }
}

pub fn factory(options: &toml::Table) -> Result<Box<dyn Emitter>, EmitError> {
    Ok(Box::new(TypescriptEmitter {
        options: Options::from_table(options)?,
    }))
}
