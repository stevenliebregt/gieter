use clap::{Parser, Subcommand, ValueEnum};
use gieter_core::config::Config;
use gieter_core::emitter::EmitterRegistry;
use gieter_core::external;
use gieter_core::pipeline::{self};
use gieter_core::source::SourceRegistry;
use std::path::{Path, PathBuf};

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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Regenerate a contract schema for the external sources or emitters and print it to stdout.
    Schema { message: SchemaMessage },
    /// Developer commands
    Dev {
        #[command(subcommand)]
        dev: DevCommands,
    },
}

#[derive(Clone, ValueEnum)]
enum SchemaMessage {
    SourceRequest,
    SourceResponse,
    EmitRequest,
    EmitResponse,
}

#[derive(Subcommand)]
enum DevCommands {
    /// Regenerate all schemas and write them to the schema folder.
    GenerateSchemas,
}

fn main() -> std::process::ExitCode {
    let Cli { config, command } = Cli::parse();

    let result = match command {
        Some(Commands::Schema { message: schema }) => print_schema(schema),
        Some(Commands::Dev {
            dev: DevCommands::GenerateSchemas,
        }) => generate_schemas(),
        None => run(&config),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn print_schema(schema: SchemaMessage) -> Result<(), Box<dyn std::error::Error>> {
    let generated = match schema {
        SchemaMessage::SourceRequest => external::source_request_schema_json()?,
        SchemaMessage::SourceResponse => external::source_response_schema_json()?,
        SchemaMessage::EmitRequest => external::emit_request_schema_json()?,
        SchemaMessage::EmitResponse => external::emit_response_schema_json()?,
    };
    println!("{generated}");
    Ok(())
}

fn generate_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas");
    std::fs::create_dir_all(dir)?;
    for (file, contents) in external::schemas()? {
        let path = format!("{dir}/{file}");
        std::fs::write(&path, contents)?;
        println!("wrote {path}");
    }
    Ok(())
}

fn run(config_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::from_path(config_path)?;
    config.resolve_env()?;

    let mut sources = SourceRegistry::default();
    sources.register("external", external::source::factory);
    sources.register("postgres", gieter_postgres::factory);
    let source = sources.build(&config.source)?;

    let mut emitters = EmitterRegistry::default();
    emitters.register("external", external::emitter::factory);
    emitters.register("typescript", gieter_typescript::factory);

    let report = pipeline::run(&config, source.as_ref(), &emitters)?;

    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    println!("generated {} file(s)", report.written.len());

    Ok(())
}
