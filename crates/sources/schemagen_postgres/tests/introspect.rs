//! Integration tests that introspect a throwaway Dockerized Postgres.
//! Requires Docker. Run with `cargo test -p schemagen_postgres`.

use schemagen_core::ir::{ColumnType, ScalarType};
use schemagen_core::source::SchemaSource;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::Container;
use testcontainers_modules::testcontainers::runners::SyncRunner;

fn start_pg() -> (Container<Postgres>, String) {
    let node = Postgres::default().start().unwrap();
    let port = node.get_host_port_ipv4(5432).unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    (node, url)
}

#[test]
fn container_boots_and_accepts_queries() {
    let (_node, url) = start_pg();
    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    let row = client.query_one("SELECT 1::int AS one", &[]).unwrap();
    let one: i32 = row.get("one");
    assert_eq!(one, 1);
}

#[test]
fn introspects_tables_enums_arrays_fks() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TYPE mood AS ENUM ('happy','sad');
            CREATE TABLE authors (id serial PRIMARY KEY, name text NOT NULL);
            CREATE TABLE books (
                id serial PRIMARY KEY,
                author_id int NOT NULL REFERENCES authors(id),
                tags text[] NULL,
                vibe mood NULL
            );
            COMMENT ON TABLE books IS 'a book';
        "#,
        )
        .unwrap();

    let source =
        schemagen_postgres::PostgresSchemaSource::connect(&url, vec!["public".into()]).unwrap();
    let catalog = source.introspect().unwrap();

    let public = &catalog.schemas[0];
    assert_eq!(public.name, "public");
    assert!(
        public
            .enums
            .iter()
            .any(|e| e.name == "mood" && e.values == vec!["happy", "sad"])
    );

    let books = public.tables.iter().find(|t| t.name == "books").unwrap();
    assert_eq!(books.comment.as_deref(), Some("a book"));

    let tags = books.columns.iter().find(|c| c.name == "tags").unwrap();
    assert_eq!(
        tags.ty,
        ColumnType::Array(Box::new(ColumnType::Scalar(ScalarType::Text {
            max_len: None
        })))
    );
    assert!(tags.nullable);

    let vibe = books.columns.iter().find(|c| c.name == "vibe").unwrap();
    assert_eq!(
        vibe.ty,
        ColumnType::Enum {
            schema: "public".into(),
            name: "mood".into()
        }
    );

    assert!(
        books
            .foreign_keys
            .iter()
            .any(|fk| fk.columns == vec!["author_id".to_string()]
                && fk.ref_table == ("public".to_string(), "authors".to_string())
                && fk.ref_columns == vec!["id".to_string()])
    );
}

#[test]
fn introspects_broad_types_and_cross_schema_references() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE SCHEMA auth;
            CREATE TYPE mood AS ENUM ('happy','sad');
            CREATE TABLE auth.users (id serial PRIMARY KEY, email text NOT NULL);
            CREATE TABLE metrics (
                id serial PRIMARY KEY,
                ratio numeric(10,2) NOT NULL,
                token uuid NOT NULL,
                recorded_at timestamptz NOT NULL,
                moods mood[] NULL,
                owner_id int NOT NULL REFERENCES auth.users(id)
            );
        "#,
        )
        .unwrap();

    let source = schemagen_postgres::PostgresSchemaSource::connect(
        &url,
        vec!["public".into(), "auth".into()],
    )
    .unwrap();
    let catalog = source.introspect().unwrap();

    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    assert_eq!(metrics.primary_key, vec!["id".to_string()]);

    let ratio = metrics.columns.iter().find(|c| c.name == "ratio").unwrap();
    assert_eq!(
        ratio.ty,
        ColumnType::Scalar(ScalarType::Decimal {
            precision: Some(10),
            scale: Some(2)
        })
    );

    let token = metrics.columns.iter().find(|c| c.name == "token").unwrap();
    assert_eq!(token.ty, ColumnType::Scalar(ScalarType::Uuid));

    let recorded_at = metrics
        .columns
        .iter()
        .find(|c| c.name == "recorded_at")
        .unwrap();
    assert_eq!(
        recorded_at.ty,
        ColumnType::Scalar(ScalarType::Timestamp {
            tz: true,
            precision: None
        })
    );

    let moods = metrics.columns.iter().find(|c| c.name == "moods").unwrap();
    assert!(moods.nullable);
    assert_eq!(
        moods.ty,
        ColumnType::Array(Box::new(ColumnType::Enum {
            schema: "public".into(),
            name: "mood".into()
        }))
    );

    // Columns keep their definition order rather than being sorted alphabetically.
    let names: Vec<&str> = metrics.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["id", "ratio", "token", "recorded_at", "moods", "owner_id"]
    );

    // The foreign key points across schemas into auth.users.
    let foreign_key = &metrics.foreign_keys[0];
    assert_eq!(foreign_key.columns, vec!["owner_id".to_string()]);
    assert_eq!(
        foreign_key.ref_table,
        ("auth".to_string(), "users".to_string())
    );
    assert_eq!(foreign_key.ref_columns, vec!["id".to_string()]);
}
