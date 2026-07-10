<div align="center">
  <img src="assets/gieter-logo.svg" alt="gieter" width="140" height="140">

  <h1>gieter</h1>

  <p><em>Pour your database schema in, get typed code out.</em></p>

  <p>
    <a href="https://crates.io/crates/gieter"><img src="https://img.shields.io/crates/v/gieter.svg" alt="crates.io"></a>
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="license">
  </p>
</div>

---

**gieter** (Dutch for *watering can*) introspects a live database and emits typed source code from it. It reads your schema, tables, columns, enums, keys, etc. and turns it into a neutral intermediate representation, then hands that to pluggable **emitters** to generate code in the language of your choice.

Today it ships a PostgreSQL source and a TypeScript emitter.

## Features

- **Live introspection** of PostgreSQL schemas (tables, columns, enums, primary & foreign keys, comments).
- **Typed TypeScript output** with configurable `type`/`interface`, enum, casing, and null styles.
- **Branded ID types**; nominal types for primary/foreign keys so a `UserId` can't be passed where a `PostId` belongs.
- **Scalar type overrides** with automatic import hoisting.
- **Flexible file layout**; split kinds across files or emit everything into one.
- **Extensible by design**; sources and emitters are just crates implementing a `gieter_core` trait.

## Missing features

Things that are currently not supported by **gieter** but are planned are:

- **composite types** and **domain types** for PostgreSQL.
- **Python emitter**.
- **Zod emitter**.
- **Rust emitter**.
- **Plugin system** so you can create external schema sources and emitters, then load them via config options.

## Install

```sh
cargo install gieter
```

Or build from source:

```sh
git clone https://github.com/stevenliebregt/gieter
cd gieter
cargo install --path crates/gieter
```

## Quick start

Create a `gieter.toml`:

```toml
[database]
url = "postgres://user:password@localhost:5432/db"
schemas = ["public"]

[[emitter]]
type = "typescript"
out_dir = "src/db"
output = "schema.ts"
```

Then run:

```sh
gieter # uses ./gieter.toml
gieter --config path/to/gieter.toml
```

The database URL may be read from the environment with `url = "env:DATABASE_URL"`.
See [`examples/`](crates/emitters/gieter_typescript/examples) for a fully-annotated config.

## Extending gieter

Everything hangs off [`gieter_core`](crates/gieter_core), which defines the schema IR plus the `Source` and `Emitter` traits. To target a new language or database, depend on it directly:

```toml
[dependencies]
gieter_core = "0.1"
```

Implement `Emitter` (or `Source`), and register it with the pipeline.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
