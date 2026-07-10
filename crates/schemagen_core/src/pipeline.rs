use crate::Error;
use crate::config::Config;
use crate::emitter::Emitter;
use crate::ir::Catalog;
use crate::source::SchemaSource;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Registry {
    emitters: HashMap<String, Box<dyn Emitter>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            emitters: HashMap::new(),
        }
    }

    pub fn register(&mut self, emitter: Box<dyn Emitter>) {
        self.emitters.insert(emitter.name().to_string(), emitter);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Emitter> {
        self.emitters.get(name).map(|boxed| boxed.as_ref())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct RunReport {
    pub written: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

pub fn run(
    config: &Config,
    source: &dyn SchemaSource,
    registry: &Registry,
) -> Result<RunReport, Error> {
    let mut catalog = source.introspect()?;

    filter_excluded(&mut catalog, &config.database.exclude_tables)?;

    let mut report = RunReport::default();

    for emitter_config in &config.emitters {
        let emitter = registry
            .get(&emitter_config.ty)
            .ok_or_else(|| Error::UnknownEmitter(emitter_config.ty.clone()))?;

        let base_dir = emitter_config
            .out_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let output = emitter.emit(&catalog, &emitter_config.options)?;

        for generated_file in output.files {
            let full_path = if generated_file.path.is_absolute() {
                generated_file.path.clone()
            } else {
                base_dir.join(&generated_file.path)
            };

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| Error::Write {
                    path: full_path.display().to_string(),
                    source,
                })?;
            }

            std::fs::write(&full_path, generated_file.contents).map_err(|source| Error::Write {
                path: full_path.display().to_string(),
                source,
            })?;
            report.written.push(full_path);
        }

        report.warnings.extend(output.warnings);
    }

    Ok(report)
}

fn filter_excluded(catalog: &mut Catalog, patterns: &[String]) -> Result<(), Error> {
    if patterns.is_empty() {
        return Ok(());
    }

    let mut builder = globset::GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(globset::Glob::new(pattern).map_err(|_| Error::BadGlob(pattern.clone()))?);
    }

    let set = builder
        .build()
        .map_err(|_| Error::BadGlob(patterns.join(",")))?;

    for schema in &mut catalog.schemas {
        schema.tables.retain(|table| !set.is_match(&table.name));
    }

    Ok(())
}
