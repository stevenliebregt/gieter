use crate::options::TypeOverride;
use gieter_core::ir::ScalarType;
use std::collections::BTreeMap;

/// A resolved TypeScript type for a scalar: the type expression plus an optional import line that
/// must accompany it in any file that uses the type.
#[derive(Debug, PartialEq)]
pub struct Mapped {
    pub ts: String,
    pub import: Option<String>,
}

/// Resolve a scalar to its TypeScript type, letting a config override win over the default.
pub fn resolve(ty: &ScalarType, overrides: &BTreeMap<String, TypeOverride>) -> Mapped {
    if let Some(over) = overrides.get(ty.key()) {
        return Mapped {
            ts: over.ts.clone(),
            import: over.import.clone(),
        };
    }

    Mapped {
        ts: default_ts(ty).to_string(),
        import: None,
    }
}

fn default_ts(ty: &ScalarType) -> &'static str {
    match ty {
        ScalarType::Boolean => "boolean",
        ScalarType::Int16 | ScalarType::Int32 | ScalarType::Int64 => "number",
        ScalarType::Float32 | ScalarType::Float64 => "number",
        ScalarType::Decimal { .. } => "number",
        ScalarType::Char { .. } | ScalarType::Text { .. } => "string",
        ScalarType::Uuid => "string",
        ScalarType::Json => "unknown",
        ScalarType::Bytes => "Uint8Array",
        ScalarType::Date | ScalarType::Time { .. } | ScalarType::Timestamp { .. } => "string",
        ScalarType::Other(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overrides(pairs: &[(&str, &str, Option<&str>)]) -> BTreeMap<String, TypeOverride> {
        pairs
            .iter()
            .map(|(key, ts, import)| {
                (
                    key.to_string(),
                    TypeOverride {
                        ts: ts.to_string(),
                        import: import.map(str::to_string),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn defaults_cover_each_scalar_family() {
        let none = BTreeMap::new();
        assert_eq!(resolve(&ScalarType::Int64, &none).ts, "number");
        assert_eq!(resolve(&ScalarType::Uuid, &none).ts, "string");
        assert_eq!(resolve(&ScalarType::Json, &none).ts, "unknown");
        assert_eq!(resolve(&ScalarType::Bytes, &none).ts, "Uint8Array");
        assert_eq!(
            resolve(
                &ScalarType::Timestamp {
                    tz: true,
                    precision: None
                },
                &none
            )
            .ts,
            "string"
        );
        assert_eq!(
            resolve(&ScalarType::Other("citext".into()), &none).ts,
            "unknown"
        );
        assert!(resolve(&ScalarType::Int64, &none).import.is_none());
    }

    #[test]
    fn type_parameters_do_not_change_the_mapping() {
        let none = BTreeMap::new();
        assert_eq!(
            resolve(
                &ScalarType::Decimal {
                    precision: Some(10),
                    scale: Some(2)
                },
                &none
            )
            .ts,
            "number"
        );
        assert_eq!(
            resolve(&ScalarType::Text { max_len: Some(5) }, &none).ts,
            "string"
        );
    }

    #[test]
    fn an_override_wins_and_carries_its_import() {
        let over = overrides(&[("uuid", "UUID", Some("import type { UUID } from './uuid';"))]);
        let mapped = resolve(&ScalarType::Uuid, &over);
        assert_eq!(mapped.ts, "UUID");
        assert_eq!(
            mapped.import.as_deref(),
            Some("import type { UUID } from './uuid';")
        );
    }

    #[test]
    fn an_unmapped_backend_type_is_overridden_by_its_raw_name() {
        let over = overrides(&[("citext", "CIText", None)]);
        assert_eq!(
            resolve(&ScalarType::Other("citext".into()), &over).ts,
            "CIText"
        );
    }
}
