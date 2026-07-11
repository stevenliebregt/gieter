use crate::emitter::GeneratedFile;
use crate::ir::Catalog;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod emitter;
pub mod source;
mod subprocess;

pub const IR_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SourceRequest {
    ir_version: u32,
    options: serde_json::Value,
    schemas: Vec<String>,
}

pub fn source_request_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(SourceRequest))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SourceResponse {
    ir_version: u32,
    catalog: Catalog,
}

pub fn source_response_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(SourceResponse))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmitRequest {
    ir_version: u32,
    catalog: Catalog,
    options: serde_json::Value,
}

pub fn emit_request_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(EmitRequest))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmitResponse {
    ir_version: u32,
    files: Vec<GeneratedFile>,
    warnings: Vec<String>,
}

pub fn emit_response_schema_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&schemars::schema_for!(EmitResponse))
}

pub fn schemas() -> Result<[(&'static str, String); 4], serde_json::Error> {
    Ok([
        ("source-request.schema.json", source_request_schema_json()?),
        (
            "source-response.schema.json",
            source_response_schema_json()?,
        ),
        ("emit-request.schema.json", emit_request_schema_json()?),
        ("emit-response.schema.json", emit_response_schema_json()?),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Any IR change that alters the request/response shape fails here until the committed schema
    /// is regenerated.
    #[test]
    fn committed_schemas_are_current() {
        for (file, current) in schemas().unwrap() {
            let path = format!("{}/../../schemas/{file}", env!("CARGO_MANIFEST_DIR"));
            let committed: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let current: serde_json::Value = serde_json::from_str(&current).unwrap();
            assert_eq!(
                committed, current,
                "{file} is stale; regenerate it. Consider if the IR version needs to be upgraded"
            );
        }
    }
}
