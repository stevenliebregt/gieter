use schemagen_core::ir::Catalog;
use schemagen_core::source::{SchemaSource, SourceError};

struct PostgresSchemaSource;

impl SchemaSource for PostgresSchemaSource {
    fn introspect(&self) -> Result<Catalog, SourceError> {
        todo!("implement this")
    }
}
