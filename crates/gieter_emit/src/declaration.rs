use crate::brand::Brands;
use crate::output::Kind;
use gieter_core::ir::{Catalog, Composite, Domain, Enum, ScalarType, Table, View};

pub enum Declaration<'a> {
    Brand(&'a str, &'a ScalarType),
    Enum(&'a Enum),
    Composite(&'a Composite),
    Domain(&'a Domain),
    Table(&'a Table),
    View(&'a View),
}

pub fn declarations<'a>(
    catalog: &'a Catalog,
    brands: &'a Brands,
) -> impl Iterator<Item = (Kind, Declaration<'a>)> {
    let mut declarations = vec![];

    // Brands go first, other items may depend on it
    for (name, scalar) in brands.declarations() {
        declarations.push((Kind::Brands, Declaration::Brand(name, scalar)));
    }

    // Now the other types, in an order that some items may depend on others
    // enums > composites > domains > tables > views
    for schema in &catalog.schemas {
        for item in &schema.enums {
            declarations.push((Kind::Enums, Declaration::Enum(item)));
        }

        for item in &schema.composites {
            declarations.push((Kind::Composites, Declaration::Composite(item)));
        }

        for item in &schema.domains {
            declarations.push((Kind::Domains, Declaration::Domain(item)));
        }

        for item in &schema.tables {
            declarations.push((Kind::Tables, Declaration::Table(item)));
        }

        for item in &schema.views {
            declarations.push((Kind::Views, Declaration::View(item)));
        }
    }

    declarations.into_iter()
}
