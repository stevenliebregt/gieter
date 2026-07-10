use clap::Parser;
use gieter_core::config::Config;
use gieter_core::pipeline::{self, Registry, RunReport};
use gieter_postgres::PostgresSource;
use gieter_typescript::TypescriptEmitter;
use std::path::PathBuf;

/// Pour your database schema in, get typed code out.
///
/// gieter introspects a live database and generates typed source code from its
/// schema, driven by a TOML config that describes the database connection and
/// the emitters to run.
#[derive(Parser)]
#[command(name = "gieter", version)]
struct Cli {
    /// Path to the TOML config file
    #[arg(short, long, default_value = "gieter.toml")]
    config: PathBuf,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(report) => {
            for warning in &report.warnings {
                eprintln!("warning: {warning}");
            }
            println!("generated {} file(s)", report.written.len());
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<RunReport, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut config = Config::from_path(&cli.config)?;
    config.resolve_env()?;

    let source = PostgresSource::connect(&config.database.url, config.database.schemas.clone())?;

    let mut registry = Registry::new();
    registry.register(Box::new(TypescriptEmitter));

    Ok(pipeline::run(&config, &source, &registry)?)
}
