#[derive(Debug)]
pub struct Catalog {
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
pub struct Table {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub comment: Option<String>,
}

#[derive(Debug)]
pub struct View {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub comment: Option<String>,
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub ty: ColumnType,
    pub nullable: bool,
    pub comment: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ColumnType {
    Scalar(ScalarType),
    Array(Box<ColumnType>),
    Enum { schema: String, name: String },
    Composite { schema: String, name: String },
    Domain { schema: String, name: String },
}

/// Backend-neutral scalar types. Each Source maps its native type names onto these;
/// anything outside the common set falls through to Other(String).
#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: (String, String), // schema name, table name
    pub ref_columns: Vec<String>,
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub schema: String,
    pub values: Vec<String>,
}

#[derive(Debug)]
pub struct Composite {
    pub name: String,
    pub schema: String,
    pub fields: Vec<Column>,
}

#[derive(Debug)]
pub struct Domain {
    pub name: String,
    pub schema: String,
    pub base: ColumnType,
    pub not_null: bool,
    pub default: Option<String>,
    // TODO: checks, engine specific?
}
