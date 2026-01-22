//! Order snapshot utilities
//!
//! This module provides utilities for order snapshot computation:
//! - `generate_instance_id`: Generate content-addressed instance IDs for items
//! - `input_to_snapshot`: Convert CartItemInput to CartItemSnapshot
//!
//! Note: Event application logic has been moved to the appliers module.
//! Use `EventAction` from `super::appliers` to apply events to snapshots.

use shared::order::CartItemSnapshot;

/// Generate a content-addressed instance_id from item properties
///
/// The instance_id is a hash of the item's properties that affect its identity:
/// - product_id
/// - price
/// - manual_discount_percent
/// - selected_options
/// - selected_specification
/// - surcharge
///
/// Items with the same instance_id can be merged (quantities added together).
pub fn generate_instance_id(
    product_id: &str,
    price: f64,
    manual_discount_percent: Option<f64>,
    options: &Option<Vec<shared::order::ItemOption>>,
    specification: &Option<shared::order::SpecificationInfo>,
    surcharge: Option<f64>,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    hasher.update(product_id.as_bytes());
    hasher.update(price.to_be_bytes());

    if let Some(discount) = manual_discount_percent {
        hasher.update(discount.to_be_bytes());
    }

    if let Some(opts) = options {
        for opt in opts {
            hasher.update(opt.attribute_id.as_bytes());
            hasher.update(opt.option_idx.to_be_bytes());
        }
    }

    if let Some(spec) = specification {
        hasher.update(spec.id.as_bytes());
    }

    if let Some(s) = surcharge {
        hasher.update(s.to_be_bytes());
    }

    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for shorter ID
}

/// Convert CartItemInput to CartItemSnapshot with generated instance_id
pub fn input_to_snapshot(input: &shared::order::CartItemInput) -> CartItemSnapshot {
    let instance_id = generate_instance_id(
        &input.product_id,
        input.price,
        input.manual_discount_percent,
        &input.selected_options,
        &input.selected_specification,
        input.surcharge,
    );

    CartItemSnapshot {
        id: input.product_id.clone(),
        instance_id,
        name: input.name.clone(),
        price: input.price,
        original_price: input.original_price,
        quantity: input.quantity,
        unpaid_quantity: input.quantity, // Initially all unpaid
        selected_options: input.selected_options.clone(),
        selected_specification: input.selected_specification.clone(),
        manual_discount_percent: input.manual_discount_percent,
        rule_discount_amount: None,
        rule_surcharge_amount: None,
        applied_rules: None,
        surcharge: input.surcharge,
        note: input.note.clone(),
        authorizer_id: input.authorizer_id.clone(),
        authorizer_name: input.authorizer_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_instance_id() {
        let id1 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id2 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id3 = generate_instance_id("product-1", 10.0, Some(50.0), &None, &None, None);

        // Same inputs should produce same ID
        assert_eq!(id1, id2);

        // Different inputs should produce different ID
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_generate_instance_id_with_price_difference() {
        let id1 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id2 = generate_instance_id("product-1", 15.0, None, &None, &None, None);

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_instance_id_with_surcharge() {
        let id1 = generate_instance_id("product-1", 10.0, None, &None, &None, None);
        let id2 = generate_instance_id("product-1", 10.0, None, &None, &None, Some(2.0));

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_input_to_snapshot() {
        let input = shared::order::CartItemInput {
            product_id: "product-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0),
            surcharge: None,
            note: Some("Test note".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        };

        let snapshot = input_to_snapshot(&input);

        assert_eq!(snapshot.id, "product-1");
        assert_eq!(snapshot.name, "Test Product");
        assert_eq!(snapshot.price, 10.0);
        assert_eq!(snapshot.quantity, 2);
        assert_eq!(snapshot.unpaid_quantity, 2);
        assert_eq!(snapshot.manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.note, Some("Test note".to_string()));
        assert!(!snapshot.instance_id.is_empty());
    }
}
