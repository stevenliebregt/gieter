use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Catalog {
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub enums: Vec<Enum>,
    pub views: Vec<View>,
    pub composites: Vec<Composite>,
    pub domains: Vec<Domain>,
}

impl Schema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables: vec![],
            enums: vec![],
            views: vec![],
            composites: vec![],
            domains: vec![],
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Table {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct View {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Column {
    pub name: String,
    pub ty: ColumnType,
    pub nullable: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ColumnType {
    Scalar(ScalarType),
    Array(Box<ColumnType>),
    Enum { schema: String, name: String },
    Composite { schema: String, name: String },
    Domain { schema: String, name: String },
}

/// Backend-neutral scalar types. Each Source maps its native type names onto these;
/// anything outside the common set falls through to Other(String).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ScalarType {
    Boolean,
    Int16,   // smallint / int2
    Int32,   // int / int4
    Int64,   // bigint / int8
    Float32, // real / float4
    Float64, // double precision / float8
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    }, // numeric / decimal
    Char {
        len: u32,
    }, // char(n) / bpchar, fixed-length
    Text {
        max_len: Option<u32>,
    }, // varchar(n) / text
    Uuid,
    Json,  // json + jsonb
    Bytes, // bytea
    Date,
    Time {
        precision: Option<u32>,
    },
    Timestamp {
        tz: bool,
        precision: Option<u32>,
    }, // timestamptz when tz is true
    Other(String), // native type name for anything off the list
}

impl ScalarType {
    /// A stable, language-neutral key naming this scalar family. Shared by emitters to use in
    /// configs. Other(name) returns the plain backend type.
    pub fn key(&self) -> &str {
        match self {
            ScalarType::Boolean => "boolean",
            ScalarType::Int16 => "int16",
            ScalarType::Int32 => "int32",
            ScalarType::Int64 => "int64",
            ScalarType::Float32 => "float32",
            ScalarType::Float64 => "float64",
            ScalarType::Decimal { .. } => "decimal",
            ScalarType::Char { .. } => "char",
            ScalarType::Text { .. } => "text",
            ScalarType::Uuid => "uuid",
            ScalarType::Json => "json",
            ScalarType::Bytes => "bytes",
            ScalarType::Date => "date",
            ScalarType::Time { .. } => "time",
            ScalarType::Timestamp { .. } => "timestamp",
            ScalarType::Other(name) => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: (String, String), // schema name, table name
    pub ref_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Enum {
    pub name: String,
    pub schema: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Composite {
    pub name: String,
    pub schema: String,
    pub fields: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Domain {
    pub name: String,
    pub schema: String,
    pub base: ColumnType,
    pub not_null: bool,
    pub default: Option<String>,
    // TODO: checks, engine specific?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Catalog {
        Catalog {
            schemas: vec![Schema {
                name: "public".into(),
                tables: vec![Table {
                    name: "users".into(),
                    schema: "public".into(),
                    columns: vec![Column {
                        name: "tags".into(),
                        ty: ColumnType::Array(Box::new(ColumnType::Scalar(ScalarType::Text {
                            max_len: None,
                        }))),
                        nullable: true,
                        comment: Some("free text tags".into()),
                    }],
                    primary_key: vec!["id".into()],
                    foreign_keys: vec![],
                    comment: None,
                }],
                enums: vec![Enum {
                    name: "mood".into(),
                    schema: "public".into(),
                    values: vec!["happy".into(), "sad".into()],
                }],
                views: vec![],
                composites: vec![],
                domains: vec![],
            }],
        }
    }

    #[test]
    fn catalog_round_trips_through_json() {
        let catalog = sample();
        let json = serde_json::to_string(&catalog).unwrap();
        let back: Catalog = serde_json::from_str(&json).unwrap();
        assert_eq!(catalog, back);
    }

    #[test]
    fn column_type_is_adjacently_tagged() {
        let ty = ColumnType::Scalar(ScalarType::Text { max_len: None });
        let value = serde_json::to_value(&ty).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "kind": "scalar",
                "value": { "kind": "text", "value": { "max_len": null } }
            })
        );
    }
}
