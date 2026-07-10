use schemagen_core::emitter::{EmitError, Emitter, EmitterOutput};
use schemagen_core::ir::Catalog;

struct TypescriptEmitter;

impl Emitter for TypescriptEmitter {
    fn name(&self) -> &str {
        "typescript"
    }

    fn emit(&self, catalog: &Catalog, options: &toml::Table) -> Result<EmitterOutput, EmitError> {
        todo!()
    }
}
