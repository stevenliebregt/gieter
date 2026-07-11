use crate::rows::{
    ColumnRow, DomainRow, EnumRow, ForeignKeyRow, FromRow, PrimaryKeyRow, RelationRow,
};
use crate::types::{TypeInfo, column_type, resolve_type};
use gieter_core::ir::{Catalog, Column, Composite, Domain, Enum, ForeignKey, Schema, Table, View};
use gieter_core::source::{Source, SourceError};
use postgres::{Client, NoTls};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;

mod rows;
mod types;

const QUERY_COLUMNS: &str = include_str!("./queries/columns.sql");
const QUERY_COMPOSITES: &str = include_str!("./queries/composites.sql");
const QUERY_DOMAINS: &str = include_str!("./queries/domains.sql");
const QUERY_ENUMS: &str = include_str!("./queries/enums.sql");
const QUERY_FOREIGN_KEYS: &str = include_str!("./queries/foreign_keys.sql");
const QUERY_PRIMARY_KEYS: &str = include_str!("./queries/primary_keys.sql");
const QUERY_RELATIONS: &str = include_str!("./queries/relations.sql");

pub struct PostgresSource {
    client: RefCell<Client>,
}

impl PostgresSource {
    pub fn connect(url: &str) -> Result<Self, SourceError> {
        let client =
            Client::connect(url, NoTls).map_err(|e| SourceError::Connect(e.to_string()))?;

        Ok(PostgresSource {
            client: RefCell::new(client),
        })
    }
}

/// Builds a Postgres source from config options for a `SourceRegistry`. Expects a
/// `url` string option.
pub fn factory(options: &toml::Table) -> Result<Box<dyn Source>, SourceError> {
    let url = options
        .get("url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| SourceError::Config("postgres source requires a 'url'".into()))?;
    Ok(Box::new(PostgresSource::connect(url)?))
}

impl Source for PostgresSource {
    fn introspect(&self, schemas: &[String]) -> Result<Catalog, SourceError> {
        let mut client = self.client.borrow_mut();

        let columns: Vec<ColumnRow> = query(&mut client, QUERY_COLUMNS, schemas)?;
        let composites: Vec<ColumnRow> = query(&mut client, QUERY_COMPOSITES, schemas)?;
        let domains: Vec<DomainRow> = query(&mut client, QUERY_DOMAINS, schemas)?;
        let enums: Vec<EnumRow> = query(&mut client, QUERY_ENUMS, schemas)?;
        let foreign_keys: Vec<ForeignKeyRow> = query(&mut client, QUERY_FOREIGN_KEYS, schemas)?;
        let primary_keys: Vec<PrimaryKeyRow> = query(&mut client, QUERY_PRIMARY_KEYS, schemas)?;
        let relations: Vec<RelationRow> = query(&mut client, QUERY_RELATIONS, schemas)?;

        Ok(build_catalog(
            relations,
            columns,
            composites,
            domains,
            enums,
            foreign_keys,
            primary_keys,
        ))
    }
}

/// Assembles the flat query rows into a sorted `Catalog`.
fn build_catalog(
    relations: Vec<RelationRow>,
    columns: Vec<ColumnRow>,
    composites: Vec<ColumnRow>,
    domains: Vec<DomainRow>,
    enums: Vec<EnumRow>,
    foreign_keys: Vec<ForeignKeyRow>,
    primary_keys: Vec<PrimaryKeyRow>,
) -> Catalog {
    let mut columns_by_table = columns_by_table(columns);
    let enums_by_schema = enums_by_schema(enums);
    let mut foreign_keys_by_table = foreign_keys_by_table(foreign_keys);
    let mut primary_keys_by_table = primary_keys_by_table(primary_keys);

    let mut schemas: HashMap<String, Schema> = HashMap::new();

    for relation in relations {
        let key = (relation.schema.clone(), relation.name.clone());
        let columns = columns_by_table.remove(&key).unwrap_or_default();

        let schema = schemas
            .entry(relation.schema.clone())
            .or_insert_with(|| Schema::new(relation.schema.clone()));

        // kind: r/p are ordinary/partitioned tables, v/m are views/materialized views.
        match relation.kind.as_str() {
            "r" | "p" => schema.tables.push(Table {
                name: relation.name,
                schema: relation.schema,
                columns,
                primary_key: primary_keys_by_table.remove(&key).unwrap_or_default(),
                foreign_keys: foreign_keys_by_table.remove(&key).unwrap_or_default(),
                comment: relation.comment,
            }),
            "v" | "m" => schema.views.push(View {
                name: relation.name,
                schema: relation.schema,
                columns,
                comment: relation.comment,
            }),
            _ => {}
        }
    }

    for (schema_name, schema_enums) in enums_by_schema {
        schemas
            .entry(schema_name.clone())
            .or_insert_with(|| Schema::new(schema_name))
            .enums = schema_enums;
    }

    for (schema_name, schema_composites) in composites_by_schema(composites) {
        schemas
            .entry(schema_name.clone())
            .or_insert_with(|| Schema::new(schema_name))
            .composites = schema_composites;
    }

    for (schema_name, schema_domains) in domains_by_schema(domains) {
        schemas
            .entry(schema_name.clone())
            .or_insert_with(|| Schema::new(schema_name))
            .domains = schema_domains;
    }

    let mut schemas: Vec<Schema> = schemas.into_values().collect();
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    for schema in &mut schemas {
        schema.tables.sort_by(|a, b| a.name.cmp(&b.name));
        schema.views.sort_by(|a, b| a.name.cmp(&b.name));
        schema.enums.sort_by(|a, b| a.name.cmp(&b.name));
        schema.composites.sort_by(|a, b| a.name.cmp(&b.name));
        schema.domains.sort_by(|a, b| a.name.cmp(&b.name));
        for table in &mut schema.tables {
            table.foreign_keys.sort_by(|a, b| {
                a.columns
                    .cmp(&b.columns)
                    .then(a.ref_table.cmp(&b.ref_table))
            });
        }
    }

    Catalog { schemas }
}

fn query<T: FromRow>(
    client: &mut RefMut<Client>,
    query: &str,
    schemas: &[String],
) -> Result<Vec<T>, SourceError> {
    let rows = client
        .query(query, &[&schemas])
        .map_err(|e| SourceError::Query(e.to_string()))?;

    Ok(rows.iter().map(T::from_row).collect())
}

/// Groups columns by (schema, table) and identifies their shape.
fn columns_by_table(mut columns: Vec<ColumnRow>) -> HashMap<(String, String), Vec<Column>> {
    let mut columns_by_table: HashMap<(String, String), Vec<Column>> = HashMap::new();

    columns.sort_by_key(|column| column.ordinal);

    for column in columns {
        let ty = column_type(&column);
        let entry = Column {
            name: column.name,
            ty,
            nullable: column.nullable,
            comment: column.comment,
        };
        columns_by_table
            .entry((column.schema, column.table_name))
            .or_default()
            .push(entry);
    }

    columns_by_table
}

/// Groups enums by schema and identifies their shape.
fn enums_by_schema(enums: Vec<EnumRow>) -> HashMap<String, Vec<Enum>> {
    let mut enums_by_table: HashMap<String, Vec<Enum>> = HashMap::new();

    for enum_row in enums {
        let entry = Enum {
            name: enum_row.name,
            schema: enum_row.schema.clone(),
            values: enum_row.values,
        };
        enums_by_table
            .entry(enum_row.schema)
            .or_default()
            .push(entry);
    }

    enums_by_table
}

/// Groups composite types by schema and identifies their shape.
fn composites_by_schema(fields: Vec<ColumnRow>) -> HashMap<String, Vec<Composite>> {
    let mut composites_by_schema: HashMap<String, Vec<Composite>> = HashMap::new();

    for ((schema, name), fields) in columns_by_table(fields) {
        composites_by_schema
            .entry(schema.clone())
            .or_default()
            .push(Composite {
                name,
                schema,
                fields,
            });
    }

    composites_by_schema
}

/// Groups domains by schema and identifies their shape.
fn domains_by_schema(domains: Vec<DomainRow>) -> HashMap<String, Vec<Domain>> {
    let mut domains_by_schema: HashMap<String, Vec<Domain>> = HashMap::new();

    for domain in domains {
        let base = resolve_type(&TypeInfo {
            udt: &domain.base_udt,
            typtype: &domain.base_typtype,
            typmod: domain.base_typmod,
            elem_udt: domain.base_elem_udt.as_deref(),
            elem_typtype: domain.base_elem_typtype.as_deref(),
            type_schema: &domain.base_type_schema,
        });
        domains_by_schema
            .entry(domain.schema.clone())
            .or_default()
            .push(Domain {
                name: domain.name,
                schema: domain.schema,
                base,
                not_null: domain.not_null,
                default: domain.default,
            });
    }

    domains_by_schema
}

/// Groups primary-key columns by (schema, table). Each table has at most one primary key.
fn primary_keys_by_table(
    primary_keys: Vec<PrimaryKeyRow>,
) -> HashMap<(String, String), Vec<String>> {
    primary_keys
        .into_iter()
        .map(|primary_key| {
            (
                (primary_key.schema, primary_key.table_name),
                primary_key.columns,
            )
        })
        .collect()
}

/// Groups foreign keys by (schema, table) and identifies their shape.
fn foreign_keys_by_table(
    foreign_keys: Vec<ForeignKeyRow>,
) -> HashMap<(String, String), Vec<ForeignKey>> {
    let mut foreign_keys_by_table: HashMap<(String, String), Vec<ForeignKey>> = HashMap::new();

    for foreign_key in foreign_keys {
        let entry = ForeignKey {
            name: foreign_key.name,
            columns: foreign_key.local_columns,
            ref_table: (foreign_key.ref_schema, foreign_key.ref_table),
            ref_columns: foreign_key.ref_columns,
        };
        foreign_keys_by_table
            .entry((foreign_key.schema, foreign_key.table_name))
            .or_default()
            .push(entry);
    }

    foreign_keys_by_table
}
