use crate::ir::Catalog;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("database connection failed: {0}")]
    Connect(String),
    #[error("introspection query failed: {0}")]
    Query(String),
}

pub trait SchemaSource {
    fn introspect(&self) -> Result<Catalog, SourceError>;
}
