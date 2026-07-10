#[derive(Debug)]
pub struct Catalog {
    pub schemas: Vec<Schema>,
}

#[derive(Debug)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub enums: Vec<Enum>,
    pub views: Vec<View>,
    // TODO: composite/domain types
}

#[derive(Debug)]
pub struct Table {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
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

#[derive(Debug)]
pub enum ColumnType {
    Scalar(ScalarType),
    Array(Box<ColumnType>),
    Enum { schema: String, name: String },
    // TODO: composite/domain types
}

/// Backend-neutral scalar types. Each SchemaSource maps its native type names onto these;
/// anything outside the common set falls through to Other(String).
#[derive(Debug)]
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

#[derive(Debug)]
pub struct ForeignKey {
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
