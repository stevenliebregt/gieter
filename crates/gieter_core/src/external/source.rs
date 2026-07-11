use crate::external::{IR_VERSION, SourceRequest, SourceResponse, subprocess};
use crate::ir::Catalog;
use crate::source::{Source, SourceError};
use std::time::Duration;

pub struct ExternalSource {
    command: Vec<String>,
    options: serde_json::Value,
    timeout: Duration,
}

impl Source for ExternalSource {
    fn introspect(&self, schemas: &[String]) -> Result<Catalog, SourceError> {
        let request = SourceRequest {
            ir_version: IR_VERSION,
            options: self.options.clone(),
            schemas: schemas.to_vec(),
        };

        let stdin = serde_json::to_vec(&request)
            .map_err(|error| SourceError::External(error.to_string()))?;

        let stdout =
            subprocess::run(&self.command, &stdin, self.timeout).map_err(SourceError::External)?;

        let response: SourceResponse = serde_json::from_slice(&stdout).map_err(|error| {
            SourceError::External(format!(
                "could not parse plugin output as SourceResponse: {error}"
            ))
        })?;

        if response.ir_version != IR_VERSION {
            return Err(SourceError::External(format!(
                "plugin returned output with IR version {} but this gieter expects {IR_VERSION}",
                response.ir_version
            )));
        }

        Ok(response.catalog)
    }
}

pub fn factory(options: &toml::Table) -> Result<Box<dyn Source>, SourceError> {
    let command = subprocess::read_command(options).map_err(SourceError::Config)?;
    let timeout = subprocess::read_timeout(options).map_err(SourceError::Config)?;
    let options = subprocess::forward_options(options).map_err(SourceError::External)?;
    Ok(Box::new(ExternalSource {
        command,
        options,
        timeout,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_catalog_from_command_stdout() {
        let response = r#"{"ir_version":1,"catalog":{"schemas":[{"name":"public","tables":[],"enums":[],"views":[],"composites":[],"domains":[]}]}}"#;
        let options: toml::Table =
            toml::from_str(&format!("command = [\"printf\", \"%s\", {response:?}]")).unwrap();
        let source = factory(&options).unwrap();

        let catalog = source.introspect(&["public".into()]).unwrap();

        assert_eq!(catalog.schemas.len(), 1);
        assert_eq!(catalog.schemas[0].name, "public");
    }

    #[test]
    fn a_version_mismatch_is_an_error() {
        let response = r#"{"ir_version":999,"catalog":{"schemas":[]}}"#;
        let options: toml::Table =
            toml::from_str(&format!("command = [\"printf\", \"%s\", {response:?}]")).unwrap();
        let source = factory(&options).unwrap();

        assert!(source.introspect(&[]).is_err());
    }

    #[test]
    fn a_missing_command_is_a_config_error() {
        let options = toml::Table::new();
        assert!(factory(&options).is_err());
    }
}
