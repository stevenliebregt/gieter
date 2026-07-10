use crate::Error;
use crate::config::Config;
use crate::emitter::Emitter;
use crate::ir::Catalog;
use crate::source::Source;
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

pub fn run(config: &Config, source: &dyn Source, registry: &Registry) -> Result<RunReport, Error> {
    let mut catalog = source.introspect()?;

    filter_excluded(&mut catalog, &config.database.exclude_tables)?;

    let mut report = RunReport::default();

    for emitter_config in &config.emitters {
        let emitter = registry
            .get(&emitter_config.ty)
            .ok_or_else(|| Error::UnknownEmitter(emitter_config.ty.clone()))?;

        let out_dir = emitter_config
            .out_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let base_dir = if out_dir.is_absolute() {
            out_dir
        } else {
            config.base_dir.join(out_dir)
        };

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
        let schema_name = schema.name.clone();
        schema.tables.retain(|table| {
            // Match the bare name (excludes across all schemas) or the `schema.table`
            // qualified name (targets one schema).
            let qualified = format!("{schema_name}.{}", table.name);
            !(set.is_match(&table.name) || set.is_match(&qualified))
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Schema, Table};

    fn table(name: &str) -> Table {
        Table {
            name: name.into(),
            schema: String::new(),
            columns: vec![],
            primary_key: vec![],
            foreign_keys: vec![],
            comment: None,
        }
    }

    fn catalog() -> Catalog {
        Catalog {
            schemas: vec![
                Schema {
                    name: "public".into(),
                    tables: vec![table("user"), table("post")],
                    enums: vec![],
                    views: vec![],
                },
                Schema {
                    name: "auth".into(),
                    tables: vec![table("user")],
                    enums: vec![],
                    views: vec![],
                },
            ],
        }
    }

    fn qualified(catalog: &Catalog) -> Vec<String> {
        let mut names = Vec::new();
        for schema in &catalog.schemas {
            for table in &schema.tables {
                names.push(format!("{}.{}", schema.name, table.name));
            }
        }
        names
    }

    #[test]
    fn a_schema_qualified_exclude_targets_one_schema() {
        let mut catalog = catalog();
        filter_excluded(&mut catalog, &["public.user".into()]).unwrap();
        assert_eq!(qualified(&catalog), ["public.post", "auth.user"]);
    }

    #[test]
    fn a_bare_exclude_matches_every_schema() {
        let mut catalog = catalog();
        filter_excluded(&mut catalog, &["user".into()]).unwrap();
        assert_eq!(qualified(&catalog), ["public.post"]);
    }
}
