use crate::emitter::EmitError;
use crate::source::SourceError;

pub mod config;
pub mod emitter;
pub mod external;
pub mod ir;
pub mod pipeline;
pub mod source;
pub mod writer;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Emit(#[from] EmitError),
    #[error("unknown emitter name '{0}' (no emitter registered under that name)")]
    UnknownEmitter(String),
    #[error("unknown source type '{0}' (no source registered under that name)")]
    UnknownSource(String),
    #[error("failed to write '{path}': {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid exclude pattern '{0}'")]
    BadGlob(String),
}
