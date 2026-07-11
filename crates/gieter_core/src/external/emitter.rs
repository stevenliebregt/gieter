use crate::emitter::{EmitError, Emitter, EmitterOutput};
use crate::external::{EmitRequest, EmitResponse, IR_VERSION, subprocess};
use crate::ir::Catalog;
use std::time::Duration;

pub struct ExternalEmitter {
    command: Vec<String>,
    options: serde_json::Value,
    timeout: Duration,
}

impl Emitter for ExternalEmitter {
    fn emit(&self, catalog: &Catalog) -> Result<EmitterOutput, EmitError> {
        let request = EmitRequest {
            ir_version: IR_VERSION,
            catalog: catalog.clone(),
            options: self.options.clone(),
        };

        let stdin =
            serde_json::to_vec(&request).map_err(|error| EmitError::External(error.to_string()))?;

        let stdout =
            subprocess::run(&self.command, &stdin, self.timeout).map_err(EmitError::External)?;

        let response: EmitResponse = serde_json::from_slice(&stdout).map_err(|error| {
            EmitError::External(format!(
                "could not parse plugin output as EmitResponse: {error}"
            ))
        })?;

        if response.ir_version != IR_VERSION {
            return Err(EmitError::External(format!(
                "plugin returned output with IR version {} but this gieter expects {IR_VERSION}",
                response.ir_version
            )));
        }

        Ok(EmitterOutput {
            files: response.files,
            warnings: response.warnings,
        })
    }
}

pub fn factory(options: &toml::Table) -> Result<Box<dyn Emitter>, EmitError> {
    let command = subprocess::read_command(options).map_err(|message| EmitError::Options {
        emitter: "external".into(),
        message,
    })?;
    let timeout = subprocess::read_timeout(options).map_err(|message| EmitError::Options {
        emitter: "external".into(),
        message,
    })?;
    let options = subprocess::forward_options(options).map_err(EmitError::External)?;

    Ok(Box::new(ExternalEmitter {
        command,
        options,
        timeout,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_files_and_warnings_from_command_stdout() {
        let response =
            r#"{"ir_version":1,"files":[{"path":"out.txt","contents":"hi"}],"warnings":["w"]}"#;
        let options: toml::Table =
            toml::from_str(&format!("command = [\"printf\", \"%s\", {response:?}]")).unwrap();
        let emitter = factory(&options).unwrap();

        let output = emitter.emit(&Catalog { schemas: vec![] }).unwrap();

        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].contents, "hi");
        assert_eq!(output.warnings, vec!["w".to_string()]);
    }

    #[test]
    fn a_missing_command_is_an_error() {
        let options = toml::Table::new();
        assert!(factory(&options).is_err());
    }
}
