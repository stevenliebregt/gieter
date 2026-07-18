use crate::output::{Kind, Output, target_file};
use gieter_core::emitter::{EmitError, EmitterOutput, GeneratedFile};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Import {
    Named { target: String, symbol: String },
    Raw(String),
}

#[derive(Debug, Default)]
pub struct Fragment {
    pub code: String,
    pub imports: Vec<Import>,
}

#[derive(Debug, Default)]
struct FileContents {
    named_imports: BTreeMap<String, BTreeSet<String>>,
    raw_imports: BTreeSet<String>,
    declarations: Vec<String>,
}

impl FileContents {
    fn render(self, header: &str, format_import: fn(&str, &[&str]) -> String) -> String {
        // Merge named imports from the same target together
        let mut import_lines = self.raw_imports;
        for (module, symbols) in &self.named_imports {
            let symbols: Vec<&str> = symbols.iter().map(String::as_str).collect();
            import_lines.insert(format_import(module, &symbols));
        }

        let mut output = String::from(header);

        if !import_lines.is_empty() {
            for import in &import_lines {
                output.push_str(import);
                output.push('\n');
            }
            output.push('\n');
        }

        output.push_str(&self.declarations.join("\n"));

        output
    }
}

#[derive(Debug)]
pub struct Files {
    emitter: String,
    format_import: fn(&str, &[&str]) -> String,
    files: BTreeMap<String, FileContents>,
    warnings: Vec<String>,
}

impl Files {
    pub fn new(emitter: &str, format_import: fn(&str, &[&str]) -> String) -> Self {
        Self {
            emitter: emitter.into(),
            format_import,
            files: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn push(&mut self, file: &str, fragment: Fragment) {
        let file_contents = self.files.entry(file.to_string()).or_default();
        file_contents.declarations.push(fragment.code);
        for import in fragment.imports {
            match import {
                Import::Named { target, symbol } => {
                    file_contents
                        .named_imports
                        .entry(target)
                        .or_default()
                        .insert(symbol);
                }
                Import::Raw(import) => {
                    file_contents.raw_imports.insert(import);
                }
            }
        }
    }

    pub fn emit(
        &mut self,
        output: &Output,
        kind: Kind,
        render: impl FnOnce(&str) -> Fragment,
    ) -> Result<(), EmitError> {
        let file = target_file(output, kind, &self.emitter)?;
        let fragment = render(file);
        self.push(file, fragment);
        Ok(())
    }

    pub fn finish(self, header: &str) -> EmitterOutput {
        let format_import = self.format_import;

        let files = self
            .files
            .into_iter()
            .map(|(name, contents)| GeneratedFile {
                path: PathBuf::from(name),
                contents: contents.render(header, format_import),
            })
            .collect::<Vec<_>>();

        EmitterOutput {
            files,
            warnings: self.warnings,
        }
    }
}
