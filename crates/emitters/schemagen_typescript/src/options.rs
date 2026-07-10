use crate::TypescriptEmitter;
use schemagen_core::emitter::{EmitError, Emitter};
use serde::Deserialize;
use std::collections::BTreeMap;

// TODO: some of these options might be shared with other languages, and could live in some base options
#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Options {
    pub type_style: TypeStyle,
    pub enum_style: EnumStyle,
    pub property_case: PropertyCase,
    pub null_style: NullStyle,
    pub comments: bool,
    pub key_comments: bool,
    pub indent: String,
    pub brand: BrandOptions,
    pub types: BTreeMap<String, TypeOverride>,
    pub output: Output,
}

impl Options {
    pub fn from_table(options: &toml::Table) -> Result<Self, EmitError> {
        toml::Value::Table(options.clone())
            .try_into()
            .map_err(|error: toml::de::Error| EmitError::Options {
                emitter: TypescriptEmitter.name().into(),
                message: error.to_string(),
            })
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            type_style: TypeStyle::default(),
            enum_style: EnumStyle::default(),
            property_case: PropertyCase::default(),
            null_style: NullStyle::default(),
            comments: true,
            key_comments: true,
            indent: "  ".into(),
            brand: BrandOptions::default(),
            types: BTreeMap::new(),
            output: Output::default(),
        }
    }
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeStyle {
    #[default]
    Interface,
    Type,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnumStyle {
    #[default]
    Union,
    Enum,
    Const,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyCase {
    #[default]
    Preserve,
    Camel,
    Snake,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullStyle {
    #[default]
    Union,
    Optional,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct BrandOptions {
    pub enabled: bool,
    pub extra: Vec<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct TypeOverride {
    pub ts: String,
    #[serde(default)]
    pub import: Option<String>,
}

/// Either one file for every kind, or a map of kind -> filename (kinds sharing a
/// filename combine, `default` catches unmapped kinds).
#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Output {
    Single(String),
    Split(BTreeMap<String, String>),
}

impl Default for Output {
    fn default() -> Self {
        Output::Single("index.ts".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Options {
        let table: toml::Table = toml::from_str(source).unwrap();
        Options::from_table(&table).unwrap()
    }

    #[test]
    fn empty_options_use_defaults() {
        let options = parse("");
        assert_eq!(options, Options::default());
        assert_eq!(options.property_case, PropertyCase::Preserve);
        assert!(options.comments);
        assert!(!options.brand.enabled);
        assert_eq!(options.output, Output::Single("index.ts".into()));
    }

    #[test]
    fn parses_the_rendering_knobs() {
        let options = parse(
            r#"
            type_style = "type"
            enum_style = "const"
            property_case = "camel"
            null_style = "optional"
            comments = false
            key_comments = false
        "#,
        );
        assert_eq!(options.type_style, TypeStyle::Type);
        assert_eq!(options.enum_style, EnumStyle::Const);
        assert_eq!(options.property_case, PropertyCase::Camel);
        assert_eq!(options.null_style, NullStyle::Optional);
        assert!(!options.comments);
        assert!(!options.key_comments);
    }

    #[test]
    fn parses_brand_and_type_overrides() {
        let options = parse(
            r#"
            [brand]
            enabled = true
            extra = ["accounts.owner_ref"]

            [types.uuid]
            ts = "UUID"
            import = "import type { UUID } from './uuid';"
        "#,
        );
        assert!(options.brand.enabled);
        assert_eq!(options.brand.extra, vec!["accounts.owner_ref".to_string()]);
        let uuid = &options.types["uuid"];
        assert_eq!(uuid.ts, "UUID");
        assert_eq!(
            uuid.import.as_deref(),
            Some("import type { UUID } from './uuid';")
        );
    }

    #[test]
    fn output_accepts_a_single_file_or_a_split_map() {
        assert_eq!(
            parse(r#"output = "db.ts""#).output,
            Output::Single("db.ts".into())
        );

        let output = parse(
            r#"
            [output]
            brands = "shared.ts"
            enums = "shared.ts"
            default = "db.ts"
        "#,
        )
        .output;
        let Output::Split(map) = output else {
            panic!("expected a split map");
        };
        assert_eq!(map["brands"], "shared.ts");
        assert_eq!(map["enums"], "shared.ts");
        assert_eq!(map["default"], "db.ts");
    }

    #[test]
    fn an_unknown_enum_value_is_an_options_error() {
        let table: toml::Table = toml::from_str(r#"type_style = "klass""#).unwrap();
        assert!(Options::from_table(&table).is_err());
    }

    #[test]
    fn the_committed_example_configs_parse() {
        for example in ["simple", "complex"] {
            let path = format!(
                "{}/examples/{example}/schemagen.toml",
                env!("CARGO_MANIFEST_DIR")
            );
            let source = std::fs::read_to_string(&path).unwrap();
            let config = schemagen_core::config::Config::from_toml_str(&source).unwrap();
            for emitter in &config.emitters {
                if emitter.ty == "typescript" {
                    Options::from_table(&emitter.options).unwrap();
                }
            }
        }
    }
}
