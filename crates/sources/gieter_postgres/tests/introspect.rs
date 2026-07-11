//! Integration tests that introspect a throwaway Dockerized Postgres.
//! Requires Docker.

use gieter_core::ir::{Catalog, ColumnType, ScalarType};
use gieter_core::source::Source;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::Container;
use testcontainers_modules::testcontainers::runners::SyncRunner;

/// Runs introspection against a throwaway Postgres seeded with `setup_query` (use it
/// to create the tables, types, etc. under test). The returned `Container` guard tears
/// the database down when dropped, so keep it bound for the life of the test.
fn introspect_with(setup_query: &str, schemas: &[&str]) -> (Container<Postgres>, Catalog) {
    let container = Postgres::default().start().unwrap();
    let port = container.get_host_port_ipv4(5432).unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    client.batch_execute(setup_query).unwrap();

    let source = gieter_postgres::PostgresSource::connect(&url).unwrap();
    let catalog = source
        .introspect(
            &schemas
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .unwrap();

    (container, catalog)
}

/// Enum types surface with their values in definition order.
#[test]
fn introspects_enum_types() {
    let (_container, catalog) =
        introspect_with("CREATE TYPE mood AS ENUM ('happy','sad');", &["public"]);
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    let mood = public.enums.iter().find(|e| e.name == "mood").unwrap();
    assert_eq!(mood.values, vec!["happy", "sad"]);
}

/// Table comments are captured.
#[test]
fn introspects_table_comments() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TABLE books (id serial PRIMARY KEY);
        COMMENT ON TABLE books IS 'a book';
        "#,
        &["public"],
    );
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();

    let books = public.tables.iter().find(|t| t.name == "books").unwrap();
    assert_eq!(books.comment.as_deref(), Some("a book"));
}

/// Array columns resolve to `Array`, for both scalar and enum element types.
#[test]
fn introspects_array_columns() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TYPE mood AS ENUM ('happy','sad');
        CREATE TABLE books (tags text[] NULL, moods mood[] NULL);
        "#,
        &["public"],
    );
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

/// A column typed as an enum resolves to an `Enum` reference.
#[test]
fn introspects_enum_reference_columns() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TYPE mood AS ENUM ('happy','sad');
        CREATE TABLE books (vibe mood NULL);
        "#,
        &["public"],
    );
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

/// Foreign keys carry their local columns, referenced table, and referenced columns.
#[test]
fn introspects_foreign_keys() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TABLE authors (id serial PRIMARY KEY, name text NOT NULL);
        CREATE TABLE books (
            id serial PRIMARY KEY,
            author_id int NOT NULL REFERENCES authors(id)
        );
        "#,
        &["public"],
    );
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

/// Composite types surface with their fields, and a column of one resolves to a
/// `Composite` reference.
#[test]
fn introspects_composite_types() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TYPE address AS (street text, number int, zip varchar(10));
        CREATE TABLE people (id serial PRIMARY KEY, home address NULL);
        "#,
        &["public"],
    );
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

/// Domains surface with their resolved base type, NOT NULL, and default, and a column
/// of one resolves to a `Domain` reference.
#[test]
fn introspects_domain_types() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE DOMAIN email AS text CHECK (VALUE ~ '@');
        CREATE DOMAIN positive AS int NOT NULL DEFAULT 1 CHECK (VALUE > 0);
        CREATE TABLE people (id serial PRIMARY KEY, contact email NULL);
        "#,
        &["public"],
    );
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

/// Every defined `ScalarType` maps from its Postgres column type, including the cases where two SQL
/// types collapse to one variant (json/jsonb, timestamp with and without a time zone) or carry a
/// modifier (varchar length, numeric precision).
#[test]
fn introspects_all_scalar_base_types() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TABLE scalars (
            flag boolean,
            i16 smallint,
            i32 int,
            i64 bigint,
            f32 real,
            f64 double precision,
            dec numeric(10,2),
            ch char(5),
            txt text,
            vch varchar(20),
            uid uuid,
            j json,
            jb jsonb,
            bin bytea,
            d date,
            t time,
            ts timestamp,
            tstz timestamptz
        );
        "#,
        &["public"],
    );
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let scalars = public.tables.iter().find(|t| t.name == "scalars").unwrap();

    let expected = [
        ("flag", ColumnType::Scalar(ScalarType::Boolean)),
        ("i16", ColumnType::Scalar(ScalarType::Int16)),
        ("i32", ColumnType::Scalar(ScalarType::Int32)),
        ("i64", ColumnType::Scalar(ScalarType::Int64)),
        ("f32", ColumnType::Scalar(ScalarType::Float32)),
        ("f64", ColumnType::Scalar(ScalarType::Float64)),
        (
            "dec",
            ColumnType::Scalar(ScalarType::Decimal {
                precision: Some(10),
                scale: Some(2),
            }),
        ),
        ("ch", ColumnType::Scalar(ScalarType::Char { len: 5 })),
        (
            "txt",
            ColumnType::Scalar(ScalarType::Text { max_len: None }),
        ),
        (
            "vch",
            ColumnType::Scalar(ScalarType::Text { max_len: Some(20) }),
        ),
        ("uid", ColumnType::Scalar(ScalarType::Uuid)),
        ("j", ColumnType::Scalar(ScalarType::Json)),
        ("jb", ColumnType::Scalar(ScalarType::Json)),
        ("bin", ColumnType::Scalar(ScalarType::Bytes)),
        ("d", ColumnType::Scalar(ScalarType::Date)),
        (
            "t",
            ColumnType::Scalar(ScalarType::Time { precision: None }),
        ),
        (
            "ts",
            ColumnType::Scalar(ScalarType::Timestamp {
                tz: false,
                precision: None,
            }),
        ),
        (
            "tstz",
            ColumnType::Scalar(ScalarType::Timestamp {
                tz: true,
                precision: None,
            }),
        ),
    ];

    for (name, ty) in expected {
        let column = scalars.columns.iter().find(|c| c.name == name).unwrap();
        assert_eq!(column.ty, ty, "column {name}");
    }
}

/// Column nullability follows the NOT NULL constraint.
#[test]
fn introspects_column_nullability() {
    let (_container, catalog) = introspect_with(
        "CREATE TABLE t (req int NOT NULL, opt int NULL);",
        &["public"],
    );
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let t = public.tables.iter().find(|t| t.name == "t").unwrap();

    let req = t.columns.iter().find(|c| c.name == "req").unwrap();
    let opt = t.columns.iter().find(|c| c.name == "opt").unwrap();
    assert!(!req.nullable);
    assert!(opt.nullable);
}

/// Primary key columns are captured.
#[test]
fn introspects_primary_keys() {
    let (_container, catalog) = introspect_with(
        "CREATE TABLE metrics (id serial PRIMARY KEY, ratio numeric NOT NULL);",
        &["public"],
    );
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    assert_eq!(metrics.primary_key, vec!["id".to_string()]);
}

/// Columns keep their definition order rather than being sorted.
#[test]
fn preserves_column_definition_order() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE TABLE metrics (
            id serial PRIMARY KEY,
            ratio numeric(10,2) NOT NULL,
            token uuid NOT NULL,
            recorded_at timestamptz NOT NULL,
            owner_id int NOT NULL
        );
        "#,
        &["public"],
    );
    let public = catalog.schemas.iter().find(|s| s.name == "public").unwrap();
    let metrics = public.tables.iter().find(|t| t.name == "metrics").unwrap();

    let names: Vec<&str> = metrics.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "ratio", "token", "recorded_at", "owner_id"]);
}

/// A foreign key that points into another schema resolves across schemas.
#[test]
fn introspects_cross_schema_foreign_keys() {
    let (_container, catalog) = introspect_with(
        r#"
        CREATE SCHEMA auth;
        CREATE TABLE auth.users (id serial PRIMARY KEY, email text NOT NULL);
        CREATE TABLE metrics (
            id serial PRIMARY KEY,
            owner_id int NOT NULL REFERENCES auth.users(id)
        );
        "#,
        &["public", "auth"],
    );
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
