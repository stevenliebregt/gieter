use crate::options::Options;
use crate::typemap;
use convert_case::{Case, Casing};
use gieter_core::ir::{Catalog, ColumnType, Table};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Resolved branding for a catalog: which columns carry a brand, and the definition
/// each brand needs. Empty when branding is disabled.
#[derive(Default)]
pub struct Brands {
    columns: HashMap<(String, String, String), String>,
    definitions: BTreeMap<String, Definition>,
}

/// A brand type's underlying TypeScript type and any import that type needs.
pub struct Definition {
    pub base_ts: String,
    pub import: Option<String>,
}

impl Brands {
    pub fn resolve(catalog: &Catalog, options: &Options) -> Self {
        let mut brands = Brands::default();

        if !options.brand.enabled {
            return brands;
        }

        let extra: HashSet<&str> = options.brand.extra.iter().map(String::as_str).collect();

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

                    let mapped = typemap::resolve(scalar, &options.types);

                    brands.columns.insert(
                        (schema.name.clone(), table.name.clone(), column.name.clone()),
                        name.clone(),
                    );

                    brands.definitions.entry(name).or_insert(Definition {
                        base_ts: mapped.ts,
                        import: mapped.import,
                    });
                }
            }
        }

        brands
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// The brand a column carries, if any.
    pub fn brand_of(&self, schema: &str, table: &str, column: &str) -> Option<&str> {
        self.columns
            .get(&(schema.to_string(), table.to_string(), column.to_string()))
            .map(String::as_str)
    }

    /// Brand definitions in name order, for emitting the brand type declarations.
    pub fn declarations(&self) -> impl Iterator<Item = (&str, &Definition)> {
        self.definitions
            .iter()
            .map(|(name, def)| (name.as_str(), def))
    }
}

/// The brand a column earns, by precedence: BRAND= comment, then extra, then a
/// single-column foreign key, then a single-column primary key.
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

/// A `<Table><Column>` brand identifier, dropping the table prefix when the column name
/// already begins with it, so `book.book_id` becomes `BookId`, not `BookBookId`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use gieter_core::ir::{Column, ForeignKey, ScalarType, Schema};

    fn scalar(name: &str, ty: ScalarType, comment: Option<&str>) -> Column {
        Column {
            name: name.into(),
            ty: ColumnType::Scalar(ty),
            nullable: false,
            comment: comment.map(str::to_string),
        }
    }

    fn table(
        name: &str,
        columns: Vec<Column>,
        primary_key: Vec<&str>,
        foreign_keys: Vec<ForeignKey>,
    ) -> Table {
        Table {
            name: name.into(),
            schema: "public".into(),
            columns,
            primary_key: primary_key.into_iter().map(String::from).collect(),
            foreign_keys,
            comment: None,
        }
    }

    fn catalog(tables: Vec<Table>) -> Catalog {
        Catalog {
            schemas: vec![Schema {
                name: "public".into(),
                tables,
                ..Default::default()
            }],
        }
    }

    fn enabled(extra: Vec<&str>) -> Options {
        Options {
            brand: crate::options::BrandOptions {
                enabled: true,
                extra: extra.into_iter().map(String::from).collect(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn disabled_branding_is_empty() {
        let catalog = catalog(vec![table(
            "books",
            vec![scalar("id", ScalarType::Int32, None)],
            vec!["id"],
            vec![],
        )]);
        assert!(Brands::resolve(&catalog, &Options::default()).is_empty());
    }

    #[test]
    fn brands_a_single_column_primary_key_and_a_foreign_key() {
        let catalog = catalog(vec![table(
            "books",
            vec![
                scalar("id", ScalarType::Int32, None),
                scalar("author_id", ScalarType::Int32, None),
            ],
            vec!["id"],
            vec![ForeignKey {
                name: "books_author_fk".into(),
                columns: vec!["author_id".into()],
                ref_table: ("public".into(), "authors".into()),
                ref_columns: vec!["id".into()],
            }],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec![]));

        assert_eq!(brands.brand_of("public", "books", "id"), Some("BooksId"));
        assert_eq!(
            brands.brand_of("public", "books", "author_id"),
            Some("AuthorsId")
        );
    }

    #[test]
    fn a_key_that_is_both_primary_and_foreign_uses_the_referenced_brand() {
        let catalog = catalog(vec![table(
            "user_settings",
            vec![scalar("user_id", ScalarType::Uuid, None)],
            vec!["user_id"],
            vec![ForeignKey {
                name: "user_settings_user_fk".into(),
                columns: vec!["user_id".into()],
                ref_table: ("public".into(), "user".into()),
                ref_columns: vec!["id".into()],
            }],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec![]));

        assert_eq!(
            brands.brand_of("public", "user_settings", "user_id"),
            Some("UserId")
        );
    }

    #[test]
    fn a_table_prefixed_key_is_not_doubled() {
        let catalog = catalog(vec![table(
            "book",
            vec![scalar("book_id", ScalarType::Uuid, None)],
            vec!["book_id"],
            vec![],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec![]));

        assert_eq!(brands.brand_of("public", "book", "book_id"), Some("BookId"));
    }

    #[test]
    fn a_brand_comment_wins_over_the_primary_key() {
        let catalog = catalog(vec![table(
            "books",
            vec![scalar(
                "id",
                ScalarType::Int32,
                Some("the id BRAND=CustomId here"),
            )],
            vec!["id"],
            vec![],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec![]));

        assert_eq!(brands.brand_of("public", "books", "id"), Some("CustomId"));
    }

    #[test]
    fn extra_columns_are_branded_by_table_and_column() {
        let catalog = catalog(vec![table(
            "accounts",
            vec![scalar("owner_ref", ScalarType::Int32, None)],
            vec![],
            vec![],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec!["accounts.owner_ref"]));

        assert_eq!(
            brands.brand_of("public", "accounts", "owner_ref"),
            Some("AccountsOwnerRef")
        );
    }

    #[test]
    fn composite_primary_keys_are_not_branded() {
        let catalog = catalog(vec![table(
            "memberships",
            vec![
                scalar("user_id", ScalarType::Int32, None),
                scalar("group_id", ScalarType::Int32, None),
            ],
            vec!["user_id", "group_id"],
            vec![],
        )]);

        let brands = Brands::resolve(&catalog, &enabled(vec![]));

        assert!(
            brands
                .brand_of("public", "memberships", "user_id")
                .is_none()
        );
    }
}
