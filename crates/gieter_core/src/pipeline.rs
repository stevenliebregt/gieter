use crate::Error;
use crate::config::Config;
use crate::emitter::EmitterRegistry;
use crate::ir::Catalog;
use crate::source::Source;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct RunReport {
    pub written: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

pub fn run(
    config: &Config,
    source: &dyn Source,
    registry: &EmitterRegistry,
) -> Result<RunReport, Error> {
    // Build emitters, catches bad emitter config here
    let mut emitters = Vec::with_capacity(config.emitters.len());
    for emitter_config in &config.emitters {
        let emitter = registry.build(emitter_config)?;

        let out_dir = emitter_config
            .out_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let base_dir = if out_dir.is_absolute() {
            out_dir
        } else {
            config.base_dir.join(out_dir)
        };

        emitters.push((emitter, base_dir));
    }

    // Introspect the database
    let mut catalog = source.introspect(&config.source.schemas)?;
    filter_excluded(&mut catalog, &config.source.exclude_tables)?;

    let mut report = RunReport::default();

    // Run emitters
    for (emitter, base_dir) in &emitters {
        let output = emitter.emit(&catalog)?;

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
            ..Default::default()
        }
    }

    fn catalog() -> Catalog {
        Catalog {
            schemas: vec![
                Schema {
                    name: "public".into(),
                    tables: vec![table("user"), table("post")],
                    ..Default::default()
                },
                Schema {
                    name: "auth".into(),
                    tables: vec![table("user")],
                    ..Default::default()
                },
            ],
        }
    }

    fn qualified(catalog: &Catalog) -> Vec<String> {
        let mut names = vec![];

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
