use convert_case::{Case, Casing};
use gieter_core::ir::{Catalog, ColumnType, ScalarType, Table};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct BrandConfig {
    pub enabled: bool,
    pub extra: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Brands {
    columns: HashMap<(String, String, String), String>,
    definitions: BTreeMap<String, ScalarType>,
}

impl Brands {
    pub fn resolve(catalog: &Catalog, config: &BrandConfig) -> Self {
        let mut brands = Brands::default();

        if !config.enabled {
            return brands;
        }

        let extra: HashSet<&str> = config.extra.iter().map(String::as_str).collect();

        for schema in &catalog.schemas {
            for table in &schema.tables {
                for column in &table.columns {
                    // Only scalar columns are branded, arrays and enums are not.
                    let ColumnType::Scalar(scalar) = &column.ty else {
                        continue;
                    };

                    let Some(name) =
                        brand_name(table, &column.name, column.comment.as_deref(), &extra)
                    else {
                        continue;
                    };

                    brands.columns.insert(
                        (schema.name.clone(), table.name.clone(), column.name.clone()),
                        name.clone(),
                    );

                    brands.definitions.entry(name).or_insert(scalar.clone());
                }
            }
        }

        brands
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Retrieve the brand of a column, if it has one
    pub fn brand_of(&self, schema: &str, table: &str, column: &str) -> Option<&str> {
        self.columns
            .get(&(schema.to_string(), table.to_string(), column.to_string()))
            .map(String::as_str)
    }

    /// Brand definitions in name order: `(brand name, underlying scalar)`.
    pub fn declarations(&self) -> impl Iterator<Item = (&str, &ScalarType)> {
        self.definitions
            .iter()
            .map(|(name, scalar)| (name.as_str(), scalar))
    }
}

/// The brand a column gets, by precedence: BRAND= comment, then extra, then a single-column
/// foreign key, then a single-column primary key.
fn brand_name(
    table: &Table,
    column: &str,
    comment: Option<&str>,
    extra: &HashSet<&str>,
) -> Option<String> {
    if let Some(name) = comment.and_then(parse_brand_comment) {
        return Some(name);
    }

    if extra.contains(format!("{}.{}", table.name, column).as_str()) {
        return Some(brand_identifier(&table.name, column));
    }

    // A single-column FK brands from the referenced column, matching the referenced table's
    // brand. This beats the primary key so a column that is both (a 1:1 extension table's key)
    // points at the parent rather than creating its own brand.
    if let Some(foreign_key) = table
        .foreign_keys
        .iter()
        .find(|fk| fk.columns.len() == 1 && fk.columns[0] == column)
        && let Some(ref_column) = foreign_key.ref_columns.first()
    {
        return Some(brand_identifier(&foreign_key.ref_table.1, ref_column));
    }

    if table.primary_key.len() == 1 && table.primary_key[0] == column {
        return Some(brand_identifier(&table.name, column));
    }

    None
}

fn brand_identifier(table: &str, column: &str) -> String {
    let table = table.to_case(Case::Pascal);
    let column = column.to_case(Case::Pascal);
    if column.starts_with(&table) {
        column
    } else {
        format!("{table}{column}")
    }
}

fn parse_brand_comment(comment: &str) -> Option<String> {
    comment
        .split_whitespace()
        .find_map(|token| token.strip_prefix("BRAND="))
        .map(str::to_string)
}
