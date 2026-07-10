use crate::rows::ColumnRow;
use schemagen_core::ir::{ColumnType, ScalarType};

pub(crate) fn column_type(column: &ColumnRow) -> ColumnType {
    // Check if it is an enum
    if column.typtype == "e" {
        return ColumnType::Enum {
            schema: column.type_schema.clone(),
            name: column.udt.clone(),
        };
    }

    // Check if it is an array, in postgres that is identifiable by the _ prefix
    if column.udt.starts_with('_') {
        return ColumnType::Array(Box::new(element_type(column)));
    }

    // Fall back to normal scalar type
    ColumnType::Scalar(scalar_from(&column.udt, column.typmod))
}

fn element_type(column: &ColumnRow) -> ColumnType {
    let elem_udt = column.elem_udt.as_deref().unwrap_or_default();

    // Check if the array inner type is an enum
    if column.elem_typtype.as_deref() == Some("e") {
        return ColumnType::Enum {
            schema: column.type_schema.clone(),
            name: elem_udt.to_string(),
        };
    }

    // Fall back to normal scalar type
    ColumnType::Scalar(scalar_from(elem_udt, column.typmod))
}

fn scalar_from(udt: &str, typmod: i32) -> ScalarType {
    match udt {
        "bool" => ScalarType::Boolean,
        "int2" => ScalarType::Int16,
        "int4" => ScalarType::Int32,
        "int8" => ScalarType::Int64,
        "float4" => ScalarType::Float32,
        "float8" => ScalarType::Float64,
        "uuid" => ScalarType::Uuid,
        "json" | "jsonb" => ScalarType::Json,
        "bytea" => ScalarType::Bytes,
        "date" => ScalarType::Date,
        "text" => ScalarType::Text { max_len: None },
        "varchar" => ScalarType::Text {
            max_len: decode_typmod(typmod),
        },
        "bpchar" => ScalarType::Char {
            len: decode_typmod(typmod).unwrap_or(1),
        },
        "numeric" => {
            // numeric packs two values: precision in the high 16 bits, scale in the low 16.
            let (precision, scale) = match decode_typmod(typmod) {
                Some(packed) => (Some((packed >> 16) & 0xFFFF), Some(packed & 0xFFFF)),
                None => (None, None),
            };
            ScalarType::Decimal { precision, scale }
        }
        "time" => ScalarType::Time {
            precision: temporal_precision(typmod),
        },
        "timestamp" => ScalarType::Timestamp {
            tz: false,
            precision: temporal_precision(typmod),
        },
        "timestamptz" => ScalarType::Timestamp {
            tz: true,
            precision: temporal_precision(typmod),
        },
        other => ScalarType::Other(other.to_string()),
    }
}

/// Strip the `VARHDRSZ` header offset from a length/numeric typmod.
/// `-1` means the modifier was unspecified (e.g. plain `varchar`).
fn decode_typmod(typmod: i32) -> Option<u32> {
    if typmod == -1 {
        None
    } else {
        Some((typmod - 4) as u32)
    }
}

/// Fractional-seconds precision for time/timestamp types. Stored directly, without
/// the `VARHDRSZ` offset. `-1` means unspecified.
fn temporal_precision(typmod: i32) -> Option<u32> {
    if typmod == -1 {
        None
    } else {
        Some(typmod as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(
        udt: &str,
        typtype: &str,
        elem_udt: Option<&str>,
        elem_typtype: Option<&str>,
        type_schema: &str,
    ) -> ColumnRow {
        ColumnRow {
            schema: "public".into(),
            table_name: "t".into(),
            name: "c".into(),
            ordinal: 1,
            nullable: false,
            typmod: -1,
            udt: udt.into(),
            typtype: typtype.into(),
            elem_udt: elem_udt.map(Into::into),
            elem_typtype: elem_typtype.map(Into::into),
            type_schema: type_schema.into(),
            comment: None,
        }
    }

    #[test]
    fn maps_the_base_scalar_types() {
        assert_eq!(scalar_from("bool", -1), ScalarType::Boolean);
        assert_eq!(scalar_from("int2", -1), ScalarType::Int16);
        assert_eq!(scalar_from("int4", -1), ScalarType::Int32);
        assert_eq!(scalar_from("int8", -1), ScalarType::Int64);
        assert_eq!(scalar_from("float4", -1), ScalarType::Float32);
        assert_eq!(scalar_from("float8", -1), ScalarType::Float64);
        assert_eq!(scalar_from("uuid", -1), ScalarType::Uuid);
        assert_eq!(scalar_from("json", -1), ScalarType::Json);
        assert_eq!(scalar_from("jsonb", -1), ScalarType::Json);
        assert_eq!(scalar_from("bytea", -1), ScalarType::Bytes);
        assert_eq!(scalar_from("date", -1), ScalarType::Date);
        assert_eq!(scalar_from("text", -1), ScalarType::Text { max_len: None });
        assert_eq!(
            scalar_from("time", -1),
            ScalarType::Time { precision: None }
        );
        assert_eq!(
            scalar_from("timestamp", -1),
            ScalarType::Timestamp {
                tz: false,
                precision: None
            }
        );
        assert_eq!(
            scalar_from("timestamptz", -1),
            ScalarType::Timestamp {
                tz: true,
                precision: None
            }
        );
    }

    #[test]
    fn unknown_types_fall_back_to_other() {
        assert_eq!(
            scalar_from("citext", -1),
            ScalarType::Other("citext".into())
        );
    }

    #[test]
    fn numeric_typmod_unpacks_into_precision_and_scale() {
        // numeric(10, 2): (10 << 16 | 2) + 4 = 655366
        assert_eq!(
            scalar_from("numeric", 655366),
            ScalarType::Decimal {
                precision: Some(10),
                scale: Some(2)
            }
        );
        assert_eq!(
            scalar_from("numeric", -1),
            ScalarType::Decimal {
                precision: None,
                scale: None
            }
        );
    }

    #[test]
    fn varchar_and_char_lengths_strip_the_header_offset() {
        // varchar(10): 10 + 4 = 14
        assert_eq!(
            scalar_from("varchar", 14),
            ScalarType::Text { max_len: Some(10) }
        );
        assert_eq!(
            scalar_from("varchar", -1),
            ScalarType::Text { max_len: None }
        );
        // char(1): 1 + 4 = 5; bare char and unspecified both default to 1
        assert_eq!(scalar_from("bpchar", 5), ScalarType::Char { len: 1 });
        assert_eq!(scalar_from("bpchar", -1), ScalarType::Char { len: 1 });
    }

    #[test]
    fn temporal_precision_has_no_header_offset() {
        assert_eq!(
            scalar_from("time", 3),
            ScalarType::Time { precision: Some(3) }
        );
        assert_eq!(
            scalar_from("timestamp", 6),
            ScalarType::Timestamp {
                tz: false,
                precision: Some(6)
            }
        );
        assert_eq!(
            scalar_from("timestamptz", 3),
            ScalarType::Timestamp {
                tz: true,
                precision: Some(3)
            }
        );
    }

    #[test]
    fn classifies_a_plain_scalar_column() {
        assert_eq!(
            column_type(&column("int4", "b", None, None, "pg_catalog")),
            ColumnType::Scalar(ScalarType::Int32)
        );
    }

    #[test]
    fn resolves_an_enum_reference_in_another_schema() {
        assert_eq!(
            column_type(&column("mood", "e", None, None, "auth")),
            ColumnType::Enum {
                schema: "auth".into(),
                name: "mood".into()
            }
        );
    }

    #[test]
    fn classifies_a_scalar_array() {
        assert_eq!(
            column_type(&column("_text", "b", Some("text"), Some("b"), "pg_catalog")),
            ColumnType::Array(Box::new(ColumnType::Scalar(ScalarType::Text {
                max_len: None
            })))
        );
    }

    #[test]
    fn resolves_a_cross_schema_enum_array() {
        assert_eq!(
            column_type(&column("_mood", "b", Some("mood"), Some("e"), "auth")),
            ColumnType::Array(Box::new(ColumnType::Enum {
                schema: "auth".into(),
                name: "mood".into()
            }))
        );
    }
}
