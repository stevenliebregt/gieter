//! Integration tests that introspect a throwaway Dockerized Postgres.
//! Requires Docker. Run with `cargo test -p gieter_postgres`.

use gieter_core::ir::{ColumnType, ScalarType};
use gieter_core::source::Source;
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
fn introspects_enum_types() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute("CREATE TYPE mood AS ENUM ('happy','sad');")
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    let mood = public.enums.iter().find(|e| e.name == "mood").unwrap();
    assert_eq!(mood.values, vec!["happy", "sad"]);
}

#[test]
fn introspects_table_comments() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TABLE books (id serial PRIMARY KEY);
            COMMENT ON TABLE books IS 'a book';
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    let books = public.tables.iter().find(|t| t.name == "books").unwrap();
    assert_eq!(books.comment.as_deref(), Some("a book"));
}

#[test]
fn introspects_array_columns() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TYPE mood AS ENUM ('happy','sad');
            CREATE TABLE books (tags text[] NULL, moods mood[] NULL);
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let books = public.tables.iter().find(|t| t.name == "books").unwrap();

    // Array of a scalar element.
    let tags = books.columns.iter().find(|c| c.name == "tags").unwrap();
    assert_eq!(
        tags.ty,
        ColumnType::Array(Box::new(ColumnType::Scalar(ScalarType::Text {
            max_len: None
        })))
    );

    // Array of an enum element resolves the element as a reference.
    let moods = books.columns.iter().find(|c| c.name == "moods").unwrap();
    assert_eq!(
        moods.ty,
        ColumnType::Array(Box::new(ColumnType::Enum {
            schema: "public".into(),
            name: "mood".into()
        }))
    );
}

#[test]
fn introspects_enum_reference_columns() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TYPE mood AS ENUM ('happy','sad');
            CREATE TABLE books (vibe mood NULL);
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let books = public.tables.iter().find(|t| t.name == "books").unwrap();

    let vibe = books.columns.iter().find(|c| c.name == "vibe").unwrap();
    assert_eq!(
        vibe.ty,
        ColumnType::Enum {
            schema: "public".into(),
            name: "mood".into()
        }
    );
}

#[test]
fn introspects_foreign_keys() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TABLE authors (id serial PRIMARY KEY, name text NOT NULL);
            CREATE TABLE books (
                id serial PRIMARY KEY,
                author_id int NOT NULL REFERENCES authors(id)
            );
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let books = public.tables.iter().find(|t| t.name == "books").unwrap();

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
fn introspects_composite_types() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TYPE address AS (street text, number int, zip varchar(10));
            CREATE TABLE people (id serial PRIMARY KEY, home address NULL);
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    // Fields keep definition order and are always nullable.
    let address = public
        .composites
        .iter()
        .find(|c| c.name == "address")
        .unwrap();
    let field_names: Vec<&str> = address.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, ["street", "number", "zip"]);
    assert_eq!(address.fields[1].ty, ColumnType::Scalar(ScalarType::Int32));
    assert!(address.fields.iter().all(|f| f.nullable));

    // A column of the composite resolves to a Composite reference.
    let people = public.tables.iter().find(|t| t.name == "people").unwrap();
    let home = people.columns.iter().find(|c| c.name == "home").unwrap();
    assert_eq!(
        home.ty,
        ColumnType::Composite {
            schema: "public".into(),
            name: "address".into()
        }
    );
}

#[test]
fn introspects_domain_types() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE DOMAIN email AS text CHECK (VALUE ~ '@');
            CREATE DOMAIN positive AS int NOT NULL DEFAULT 1 CHECK (VALUE > 0);
            CREATE TABLE people (id serial PRIMARY KEY, contact email NULL);
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    // Base type resolved; no NOT NULL.
    let email = public.domains.iter().find(|d| d.name == "email").unwrap();
    assert_eq!(
        email.base,
        ColumnType::Scalar(ScalarType::Text { max_len: None })
    );
    assert!(!email.not_null);

    // NOT NULL and default captured.
    let positive = public
        .domains
        .iter()
        .find(|d| d.name == "positive")
        .unwrap();
    assert_eq!(positive.base, ColumnType::Scalar(ScalarType::Int32));
    assert!(positive.not_null);
    assert_eq!(positive.default.as_deref(), Some("1"));

    // A column of the domain resolves to a Domain reference.
    let people = public.tables.iter().find(|t| t.name == "people").unwrap();
    let contact = people.columns.iter().find(|c| c.name == "contact").unwrap();
    assert_eq!(
        contact.ty,
        ColumnType::Domain {
            schema: "public".into(),
            name: "email".into()
        }
    );
}

#[test]
fn introspects_scalar_column_types() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TABLE metrics (
                ratio numeric(10,2) NOT NULL,
                token uuid NOT NULL,
                recorded_at timestamptz NOT NULL
            );
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

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
}

#[test]
fn introspects_column_nullability() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute("CREATE TABLE t (req int NOT NULL, opt int NULL);")
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let t = public.tables.iter().find(|t| t.name == "t").unwrap();

    let req = t.columns.iter().find(|c| c.name == "req").unwrap();
    let opt = t.columns.iter().find(|c| c.name == "opt").unwrap();
    assert!(!req.nullable);
    assert!(opt.nullable);
}

#[test]
fn introspects_primary_keys() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute("CREATE TABLE metrics (id serial PRIMARY KEY, ratio numeric NOT NULL);")
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    assert_eq!(metrics.primary_key, vec!["id".to_string()]);
}

#[test]
fn preserves_column_definition_order() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE TABLE metrics (
                id serial PRIMARY KEY,
                ratio numeric(10,2) NOT NULL,
                token uuid NOT NULL,
                recorded_at timestamptz NOT NULL,
                owner_id int NOT NULL
            );
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source.introspect(&["public".into()]).unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    // Columns keep their definition order rather than being sorted alphabetically.
    let names: Vec<&str> = metrics.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "ratio", "token", "recorded_at", "owner_id"]);
}

#[test]
fn introspects_cross_schema_foreign_keys() {
    let (_node, url) = start_pg();

    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin
        .batch_execute(
            r#"
            CREATE SCHEMA auth;
            CREATE TABLE auth.users (id serial PRIMARY KEY, email text NOT NULL);
            CREATE TABLE metrics (
                id serial PRIMARY KEY,
                owner_id int NOT NULL REFERENCES auth.users(id)
            );
        "#,
        )
        .unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source
        .introspect(&["public".into(), "auth".into()])
        .unwrap();
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    // The foreign key points across schemas into auth.users.
    let foreign_key = &metrics.foreign_keys[0];
    assert_eq!(foreign_key.columns, vec!["owner_id".to_string()]);
    assert_eq!(
        foreign_key.ref_table,
        ("auth".to_string(), "users".to_string())
    );
    assert_eq!(foreign_key.ref_columns, vec!["id".to_string()]);
}
