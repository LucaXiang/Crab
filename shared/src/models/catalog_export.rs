//! Catalog export/import payload — shared between edge-server and crab-cloud.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    Attribute, AttributeBinding, Category, DiningTable, PriceRule, ProductFull, Tag, Zone,
};

/// Catalog export payload — the content of `catalog.json` inside the ZIP.
///
/// Used by both edge-server and crab-cloud for data transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogExport {
    pub version: u32,
    pub exported_at: i64,
    pub tags: Vec<Tag>,
    pub categories: Vec<Category>,
    pub products: Vec<ProductFull>,
    pub attributes: Vec<Attribute>,
    pub attribute_bindings: Vec<AttributeBinding>,
    #[serde(default)]
    pub price_rules: Vec<PriceRule>,
    #[serde(default)]
    pub zones: Vec<Zone>,
    #[serde(default)]
    pub dining_tables: Vec<DiningTable>,
}

/// Validate catalog data integrity before import.
///
/// Checks referential integrity (FK relationships) and basic constraints.
/// Returns `Ok(())` if valid, or `Err(message)` describing the first violation.
pub fn validate_catalog(catalog: &CatalogExport) -> Result<(), String> {
    // Collect known IDs
    let tag_ids: HashSet<i64> = catalog.tags.iter().map(|t| t.id).collect();
    let category_ids: HashSet<i64> = catalog.categories.iter().map(|c| c.id).collect();
    let product_ids: HashSet<i64> = catalog.products.iter().map(|p| p.id).collect();
    let attribute_ids: HashSet<i64> = catalog.attributes.iter().map(|a| a.id).collect();
    let zone_ids: HashSet<i64> = catalog.zones.iter().map(|z| z.id).collect();

    // Check duplicate IDs
    if tag_ids.len() != catalog.tags.len() {
        return Err("Duplicate tag IDs".into());
    }
    if category_ids.len() != catalog.categories.len() {
        return Err("Duplicate category IDs".into());
    }
    if product_ids.len() != catalog.products.len() {
        return Err("Duplicate product IDs".into());
    }
    if attribute_ids.len() != catalog.attributes.len() {
        return Err("Duplicate attribute IDs".into());
    }
    if zone_ids.len() != catalog.zones.len() {
        return Err("Duplicate zone IDs".into());
    }

    // Product → Category FK
    for p in &catalog.products {
        if !category_ids.contains(&p.category_id) {
            return Err(format!(
                "Product '{}' (id={}) references unknown category_id={}",
                p.name, p.id, p.category_id
            ));
        }
        // Product must have at least one spec
        if p.specs.is_empty() {
            return Err(format!("Product '{}' (id={}) has no specs", p.name, p.id));
        }
    }

    // Category → Tag FK
    for c in &catalog.categories {
        for tag_id in &c.tag_ids {
            if !tag_ids.contains(tag_id) {
                return Err(format!(
                    "Category '{}' (id={}) references unknown tag_id={}",
                    c.name, c.id, tag_id
                ));
            }
        }
    }

    // Product → Tag FK
    for p in &catalog.products {
        for tag in &p.tags {
            if !tag_ids.contains(&tag.id) {
                return Err(format!(
                    "Product '{}' (id={}) references unknown tag_id={}",
                    p.name, p.id, tag.id
                ));
            }
        }
    }

    // AttributeBinding → owner FK
    for b in &catalog.attribute_bindings {
        let owner_valid = match b.owner_type.as_str() {
            "product" => product_ids.contains(&b.owner_id),
            "category" => category_ids.contains(&b.owner_id),
            _ => false,
        };
        if !owner_valid {
            return Err(format!(
                "AttributeBinding (id={}) references unknown {}={}",
                b.id, b.owner_type, b.owner_id
            ));
        }
        if !attribute_ids.contains(&b.attribute_id) {
            return Err(format!(
                "AttributeBinding (id={}) references unknown attribute_id={}",
                b.id, b.attribute_id
            ));
        }
    }

    // DiningTable → Zone FK
    for dt in &catalog.dining_tables {
        if !zone_ids.contains(&dt.zone_id) {
            return Err(format!(
                "DiningTable '{}' (id={}) references unknown zone_id={}",
                dt.name, dt.id, dt.zone_id
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_exported_catalog() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test_catalog.json"
        ))
        .unwrap();
        match serde_json::from_str::<CatalogExport>(&json) {
            Ok(c) => {
                println!(
                    "OK: {} products, {} categories",
                    c.products.len(),
                    c.categories.len()
                );
                // Also validate
                validate_catalog(&c).unwrap();
                println!("Validation passed");
            }
            Err(e) => {
                panic!("Deserialization failed: {e}");
            }
        }
    }
}
