<div align="center">
  <img src="https://raw.githubusercontent.com/stevenliebregt/gieter/HEAD/assets/gieter-logo.svg" alt="gieter" width="140" height="140">

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
- **Language-agnostic plugins**; an external process can act as a source or emitter by exchanging the schema IR as JSON over stdin/stdout, so a plugin can be written in any language.

## Missing features

Things that are currently not supported by **gieter** but are planned are:

- **Python emitter**.
- **Zod emitter**.
- **Rust emitter**.

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
[source]
type = "postgres"
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

There are two ways to add a source or emitter: in-process in Rust, or an external process in any language.

### In-process (Rust)

Everything hangs off [`gieter_core`](crates/gieter_core), which defines the schema IR plus the `Source` and `Emitter` traits. Depend on it, implement the trait, and register a factory in your own binary alongside the built-ins:

```toml
[dependencies]
gieter_core = "<version>"
```

```rust
let mut sources = SourceRegistry::default();
sources.register("my-db", my_crate::factory);
```

### External process (any language)

An external process can be a source or emitter without any Rust. gieter runs the command you configure and exchanges the schema IR as JSON over the process's stdin and stdout, so the plugin can be written in Python, Node, Go, or anything that reads stdin and writes stdout. The `command` is an argv array, so it works the same across platforms.

```toml
[source]
type = "external"
command = ["python3", "introspect.py"]
schemas = ["public"]

[[emitter]]
type = "external"
command = ["node", "emit.js"]
out_dir = "generated"
```

- A **source** receives a `SourceRequest` (`{ ir_version, schemas, options }`) on stdin and writes a `SourceResponse` (`{ ir_version, catalog }`) to stdout.
- An **emitter** receives an `EmitRequest` (`{ ir_version, catalog, options }`) and writes an `EmitResponse` (`{ ir_version, files, warnings }`).

Every key in the config block except the transport keys `command` and `timeout` is forwarded to the plugin as its `options`. A plugin that runs longer than `timeout` seconds (default 120) is stopped with an error. Print the exact JSON Schema for any message so you can generate types for your plugin:

_See the `examples/external/markdown-emitter` example which has both an external source and an external emitter._

```sh
gieter schema source-request
gieter schema emit-response
```

The `ir_version` field guards the contract: if a plugin speaks a version gieter does not expect, gieter stops with a clear message instead of misreading the data.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
