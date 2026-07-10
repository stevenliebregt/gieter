use postgres::Row;

pub(crate) trait FromRow {
    fn from_row(row: &Row) -> Self;
}

#[derive(Debug)]
pub(crate) struct RelationRow {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) comment: Option<String>,
}

impl FromRow for RelationRow {
    fn from_row(row: &Row) -> Self {
        RelationRow {
            schema: row.get("schema"),
            name: row.get("name"),
            kind: row.get("kind"),
            comment: row.get("comment"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ColumnRow {
    pub(crate) schema: String,
    pub(crate) table_name: String,
    pub(crate) name: String,
    pub(crate) ordinal: i16,
    pub(crate) nullable: bool,
    pub(crate) typmod: i32,
    pub(crate) udt: String,
    pub(crate) typtype: String,
    pub(crate) elem_udt: Option<String>,
    pub(crate) elem_typtype: Option<String>,
    pub(crate) type_schema: String,
    pub(crate) comment: Option<String>,
}

impl FromRow for ColumnRow {
    fn from_row(row: &Row) -> Self {
        ColumnRow {
            schema: row.get("schema"),
            table_name: row.get("table_name"),
            name: row.get("name"),
            ordinal: row.get("ordinal"),
            nullable: row.get("nullable"),
            typmod: row.get("typmod"),
            udt: row.get("udt"),
            typtype: row.get("typtype"),
            elem_udt: row.get("elem_udt"),
            elem_typtype: row.get("elem_typtype"),
            type_schema: row.get("type_schema"),
            comment: row.get("comment"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct EnumRow {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) values: Vec<String>,
}

impl FromRow for EnumRow {
    fn from_row(row: &Row) -> Self {
        EnumRow {
            schema: row.get("schema"),
            name: row.get("name"),
            values: row.get("values"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ForeignKeyRow {
    pub(crate) schema: String,
    pub(crate) table_name: String,
    pub(crate) name: String,
    pub(crate) local_columns: Vec<String>,
    pub(crate) ref_schema: String,
    pub(crate) ref_table: String,
    pub(crate) ref_columns: Vec<String>,
}

impl FromRow for ForeignKeyRow {
    fn from_row(row: &Row) -> Self {
        ForeignKeyRow {
            schema: row.get("schema"),
            table_name: row.get("table_name"),
            name: row.get("name"),
            local_columns: row.get("local_columns"),
            ref_schema: row.get("ref_schema"),
            ref_table: row.get("ref_table"),
            ref_columns: row.get("ref_columns"),
        }
    }
}
