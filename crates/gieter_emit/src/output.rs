use gieter_core::emitter::EmitError;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Either one file for every kind, or a per-kind split with a `default` fallback.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(try_from = "RawOutput")]
pub enum Output {
    Single(String),
    Split(Split),
}

/// A per-kind file map with an optional `default` for unmapped kinds.
#[derive(Debug, PartialEq)]
pub struct Split {
    default: Option<String>,
    by_kind: BTreeMap<Kind, String>,
}

/// A raw version that can be deserialized from the toml tables, which is used to construct the
/// proper [`Output`].
#[derive(Deserialize)]
#[serde(untagged)]
enum RawOutput {
    Single(String),
    Split(BTreeMap<String, String>),
}

impl TryFrom<RawOutput> for Output {
    type Error = String;

    fn try_from(raw: RawOutput) -> Result<Self, Self::Error> {
        match raw {
            RawOutput::Single(file) => Ok(Output::Single(file)),
            RawOutput::Split(mut map) => {
                let default = map.remove("default");

                let mut by_kind = BTreeMap::new();

                for (key, file) in map {
                    let kind = Kind::from_key(&key).ok_or_else(|| {
                        let options = Kind::ALL.map(|kind| kind.as_str()).join(", ");
                        format!(
                            "unknown output kind '{key}'; expected one of: {options}, or default"
                        )
                    })?;

                    by_kind.insert(kind, file);
                }

                Ok(Output::Split(Split { default, by_kind }))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Brands,
    Enums,
    Composites,
    Domains,
    Tables,
    Views,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Brands,
        Kind::Enums,
        Kind::Composites,
        Kind::Domains,
        Kind::Tables,
        Kind::Views,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Brands => "brands",
            Kind::Enums => "enums",
            Kind::Composites => "composites",
            Kind::Domains => "domains",
            Kind::Tables => "tables",
            Kind::Views => "views",
        }
    }

    fn from_key(key: &str) -> Option<Kind> {
        Some(match key {
            "brands" => Kind::Brands,
            "enums" => Kind::Enums,
            "composites" => Kind::Composites,
            "domains" => Kind::Domains,
            "tables" => Kind::Tables,
            "views" => Kind::Views,
            _ => return None,
        })
    }
}

/// Look up the right file for the passed kind. Falling back to default.
fn resolve(output: &Output, kind: Kind) -> Option<&str> {
    match output {
        Output::Single(file) => Some(file),
        Output::Split(split) => split
            .by_kind
            .get(&kind)
            .or(split.default.as_ref())
            .map(String::as_str),
    }
}

pub fn target_file<'a>(
    output: &'a Output,
    kind: Kind,
    emitter: &str,
) -> Result<&'a str, EmitError> {
    resolve(output, kind).ok_or_else(|| EmitError::Options {
        emitter: emitter.to_string(),
        message: format!(
            "no output file for kind '{}'; map it or add a `default`",
            kind.as_str()
        ),
    })
}

/// Resolves cross-file imports, returns a path when the kind will be written to a different file
/// than the current one.
pub fn import_target<'a>(output: &'a Output, current_file: &str, kind: Kind) -> Option<&'a str> {
    let target = resolve(output, kind)?;
    (target != current_file).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> Output {
        #[derive(Deserialize)]
        struct Wrapper {
            output: Output,
        }
        let wrapper: Wrapper = toml::from_str(toml).expect("parse");
        wrapper.output
    }

    #[test]
    fn output_accepts_a_single_file() {
        assert_eq!(parse(r#"output = "db.ts""#), Output::Single("db.ts".into()));
    }

    #[test]
    fn output_accepts_a_split_map_with_a_default() {
        let output = parse(
            r#"
            [output]
            brands = "shared.ts"
            enums = "shared.ts"
            default = "db.ts"
        "#,
        );
        let Output::Split(split) = &output else {
            panic!("expected a split map");
        };
        assert_eq!(split.by_kind[&Kind::Brands], "shared.ts");
        assert_eq!(split.by_kind[&Kind::Enums], "shared.ts");
        assert_eq!(split.default.as_deref(), Some("db.ts"));
    }

    #[test]
    fn an_unknown_output_kind_is_an_error() {
        let result: Result<Output, _> = toml::from_str(r#"tabels = "x.ts""#);
        assert!(result.is_err(), "unknown kind should be rejected");
    }

    #[test]
    fn target_file_uses_default_when_kind_is_unmapped() {
        let output = parse(
            r#"
            [output]
            enums = "enums.ts"
            default = "db.ts"
        "#,
        );
        assert_eq!(target_file(&output, Kind::Enums, "ts").unwrap(), "enums.ts");
        assert_eq!(target_file(&output, Kind::Tables, "ts").unwrap(), "db.ts");
    }

    #[test]
    fn target_file_errors_without_a_match_or_default() {
        let output = parse(
            r#"
            [output]
            enums = "enums.ts"
        "#,
        );
        let err = target_file(&output, Kind::Tables, "typescript").unwrap_err();
        assert!(matches!(err, EmitError::Options { emitter, .. } if emitter == "typescript"));
    }

    #[test]
    fn target_file_single_returns_the_one_file_for_every_kind() {
        let output = Output::Single("db.ts".into());
        assert_eq!(target_file(&output, Kind::Brands, "ts").unwrap(), "db.ts");
        assert_eq!(target_file(&output, Kind::Views, "ts").unwrap(), "db.ts");
    }

    #[test]
    fn import_target_is_some_only_across_files() {
        let output = parse(
            r#"
            [output]
            enums = "enums.ts"
            default = "db.ts"
        "#,
        );
        // Enum referenced from db.ts -> lands in enums.ts -> cross-file.
        assert_eq!(
            import_target(&output, "db.ts", Kind::Enums),
            Some("enums.ts")
        );
        // Enum referenced from within enums.ts itself -> already in scope.
        assert_eq!(import_target(&output, "enums.ts", Kind::Enums), None);
    }

    #[test]
    fn import_target_single_is_always_in_scope() {
        let output = Output::Single("db.ts".into());
        assert_eq!(import_target(&output, "db.ts", Kind::Enums), None);
    }
}
