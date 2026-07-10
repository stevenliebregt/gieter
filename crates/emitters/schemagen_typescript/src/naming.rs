use crate::options::PropertyCase;
use convert_case::{Case, Casing};

pub fn property_name(name: &str, case: &PropertyCase) -> String {
    match case {
        PropertyCase::Preserve => name.to_string(),
        PropertyCase::Camel => name.to_case(Case::Camel),
        PropertyCase::Snake => name.to_case(Case::Snake),
    }
}

/// Member identifier for an enum value: kept as-is when it is already a valid identifier
/// (so SCREAMING_CASE survives), otherwise PascalCased from a sanitized form.
pub fn enum_member(value: &str) -> String {
    if is_identifier(value) {
        value.to_string()
    } else {
        value.to_case(Case::Pascal)
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_leaves_the_property_name_untouched() {
        assert_eq!(
            property_name("author_id", &PropertyCase::Preserve),
            "author_id"
        );
    }

    #[test]
    fn property_name_camel_and_snake_recase_across_conventions() {
        assert_eq!(property_name("author_id", &PropertyCase::Camel), "authorId");
        assert_eq!(property_name("authorId", &PropertyCase::Snake), "author_id");
    }
}
