//! Default catalog templates — pushed to edge on first connect
//!
//! Converts the former SQLite seed data into a CatalogSnapshot that can be
//! sent via FullSync RPC to newly activated edge servers.

use shared::cloud::catalog::{
    AttributeSnapshotItem, CatalogSnapshot, CategorySnapshotItem, ProductSnapshotItem,
    SnapshotBinding,
};
use shared::models::attribute::{AttributeCreate, AttributeOptionInput};
use shared::models::category::CategoryCreate;
use shared::models::product::{ProductCreate, ProductSpecInput};

/// Build the default Spanish restaurant catalog snapshot.
///
/// Categories (index):
///   0: Tapas y Raciones
///   1: Platos Principales
///   2: Postres
///   3: Cafés
///   4: Bebidas
///
/// Attributes (index):
///   0: Punto (meat doneness)
///   1: Complementos (drink extras)
///   2: Tipo de Leche (milk type)
///   3: Extras (tapa extras)
pub fn default_snapshot() -> CatalogSnapshot {
    CatalogSnapshot {
        tags: vec![],
        categories: categories(),
        products: products(),
        attributes: attributes(),
    }
}

fn spec(name: &str, price: f64) -> ProductSpecInput {
    ProductSpecInput {
        name: name.into(),
        price,
        display_order: 0,
        is_default: true,
        is_active: true,
        receipt_name: None,
        is_root: true,
    }
}

fn extra_spec(name: &str, price: f64, order: i32) -> ProductSpecInput {
    ProductSpecInput {
        name: name.into(),
        price,
        display_order: order,
        is_default: false,
        is_active: true,
        receipt_name: None,
        is_root: false,
    }
}

fn product(
    name: &str,
    cat_idx: usize,
    sort: i32,
    tax: i32,
    specs: Vec<ProductSpecInput>,
) -> ProductSnapshotItem {
    ProductSnapshotItem {
        category_index: cat_idx,
        data: ProductCreate {
            name: name.into(),
            image: None,
            category_id: 0, // filled by executor from category_index
            sort_order: Some(sort),
            tax_rate: Some(tax),
            receipt_name: None,
            kitchen_print_name: None,
            is_kitchen_print_enabled: None,
            is_label_print_enabled: None,
            external_id: None,
            tags: None,
            specs,
        },
        attribute_bindings: vec![],
    }
}

fn categories() -> Vec<CategorySnapshotItem> {
    let cat = |name: &str, sort: i32, kitchen: bool| CategorySnapshotItem {
        data: CategoryCreate {
            name: name.into(),
            sort_order: Some(sort),
            kitchen_print_destinations: vec![],
            label_print_destinations: vec![],
            is_kitchen_print_enabled: Some(kitchen),
            is_label_print_enabled: Some(false),
            is_virtual: None,
            tag_ids: vec![],
            match_mode: None,
            is_display: None,
        },
        attribute_bindings: vec![],
    };

    let mut cats = vec![
        cat("Tapas y Raciones", 1, true),   // 0
        cat("Platos Principales", 2, true), // 1
        cat("Postres", 3, true),            // 2
        cat("Cafés", 4, false),             // 3
        cat("Bebidas", 5, false),           // 4
    ];

    // Complementos → Bebidas (category 4, attr 1)
    cats[4].attribute_bindings.push(SnapshotBinding {
        attribute_index: 1,
        is_required: false,
        display_order: 1,
        default_option_ids: None,
    });

    // Extras → Tapas y Raciones (category 0, attr 3)
    cats[0].attribute_bindings.push(SnapshotBinding {
        attribute_index: 3,
        is_required: false,
        display_order: 2,
        default_option_ids: None,
    });

    cats
}

fn products() -> Vec<ProductSnapshotItem> {
    let mut prods = vec![
        // Tapas y Raciones (cat 0)
        product("Patatas Bravas", 0, 1, 10, vec![spec("", 4.50)]),
        product("Tortilla Española", 0, 2, 10, vec![spec("", 5.50)]),
        product("Jamón Ibérico", 0, 3, 10, vec![spec("", 14.00)]),
        product("Croquetas Caseras", 0, 4, 10, vec![spec("", 6.50)]),
        product("Gambas al Ajillo", 0, 5, 10, vec![spec("", 9.50)]),
        product("Pimientos de Padrón", 0, 6, 10, vec![spec("", 5.50)]),
        // Platos Principales (cat 1)
        product("Paella Valenciana", 1, 1, 10, vec![spec("", 14.50)]),
        product("Solomillo a la Plancha", 1, 2, 10, vec![spec("", 18.50)]), // idx 7
        product("Merluza a la Vasca", 1, 3, 10, vec![spec("", 15.50)]),
        product("Secreto Ibérico", 1, 4, 10, vec![spec("", 16.00)]), // idx 9
        // Postres (cat 2)
        product("Crema Catalana", 2, 1, 10, vec![spec("", 5.50)]),
        product("Tarta de Santiago", 2, 2, 10, vec![spec("", 5.00)]),
        product("Churros con Chocolate", 2, 3, 10, vec![spec("", 4.50)]),
        // Cafés (cat 3)
        product("Café Solo", 3, 1, 10, vec![spec("", 1.30)]),
        product("Café con Leche", 3, 2, 10, vec![spec("", 1.60)]), // idx 14
        product("Cortado", 3, 3, 10, vec![spec("", 1.40)]),        // idx 15
        // Bebidas (cat 4) — multi-spec items
        product(
            "Agua Mineral",
            4,
            1,
            10,
            vec![spec("0.5L", 1.50), extra_spec("1L", 2.50, 1)],
        ),
        product("Refresco", 4, 2, 10, vec![spec("", 2.50)]),
        product("Zumo Natural", 4, 3, 10, vec![spec("", 3.50)]),
        product(
            "Caña",
            4,
            4,
            21,
            vec![spec("", 2.00), extra_spec("Jarra", 5.00, 1)],
        ),
        product(
            "Copa de Vino Tinto",
            4,
            5,
            21,
            vec![spec("Copa", 3.00), extra_spec("Botella", 15.00, 1)],
        ),
        product(
            "Copa de Vino Blanco",
            4,
            6,
            21,
            vec![spec("Copa", 3.00), extra_spec("Botella", 14.00, 1)],
        ),
    ];

    // Punto → Solomillo (idx 7)
    prods[7].attribute_bindings.push(SnapshotBinding {
        attribute_index: 0,
        is_required: true,
        display_order: 1,
        default_option_ids: None,
    });

    // Punto → Secreto Ibérico (idx 9)
    prods[9].attribute_bindings.push(SnapshotBinding {
        attribute_index: 0,
        is_required: true,
        display_order: 1,
        default_option_ids: None,
    });

    // Tipo de Leche → Café con Leche (idx 14)
    prods[14].attribute_bindings.push(SnapshotBinding {
        attribute_index: 2,
        is_required: false,
        display_order: 1,
        default_option_ids: None,
    });

    // Tipo de Leche → Cortado (idx 15)
    prods[15].attribute_bindings.push(SnapshotBinding {
        attribute_index: 2,
        is_required: false,
        display_order: 1,
        default_option_ids: None,
    });

    prods
}

fn attributes() -> Vec<AttributeSnapshotItem> {
    let opt = |name: &str, price: f64, order: i32| AttributeOptionInput {
        name: name.into(),
        price_modifier: price,
        display_order: order,
        receipt_name: None,
        kitchen_print_name: None,
        enable_quantity: false,
        max_quantity: None,
    };

    vec![
        // 0: Punto de carne (single select)
        AttributeSnapshotItem {
            data: AttributeCreate {
                name: "Punto".into(),
                is_multi_select: Some(false),
                max_selections: None,
                default_option_ids: None,
                display_order: Some(1),
                show_on_receipt: Some(true),
                receipt_name: None,
                show_on_kitchen_print: Some(true),
                kitchen_print_name: None,
                options: Some(vec![
                    opt("Poco hecho", 0.0, 1),
                    opt("Al punto", 0.0, 2),
                    opt("Muy hecho", 0.0, 3),
                ]),
            },
        },
        // 1: Complementos de bebida (multi select)
        AttributeSnapshotItem {
            data: AttributeCreate {
                name: "Complementos".into(),
                is_multi_select: Some(true),
                max_selections: None,
                default_option_ids: None,
                display_order: Some(2),
                show_on_receipt: Some(true),
                receipt_name: None,
                show_on_kitchen_print: Some(true),
                kitchen_print_name: None,
                options: Some(vec![
                    opt("Con hielo", 0.10, 1),
                    opt("Con limón", 0.10, 2),
                    opt("Con aceituna", 0.10, 3),
                ]),
            },
        },
        // 2: Tipo de Leche (single select)
        AttributeSnapshotItem {
            data: AttributeCreate {
                name: "Tipo de Leche".into(),
                is_multi_select: Some(false),
                max_selections: None,
                default_option_ids: None,
                display_order: Some(3),
                show_on_receipt: Some(false),
                receipt_name: None,
                show_on_kitchen_print: Some(true),
                kitchen_print_name: None,
                options: Some(vec![
                    opt("Normal", 0.0, 1),
                    opt("Desnatada", 0.0, 2),
                    opt("Avena", 0.30, 3),
                ]),
            },
        },
        // 3: Extras de tapas (multi select)
        AttributeSnapshotItem {
            data: AttributeCreate {
                name: "Extras".into(),
                is_multi_select: Some(true),
                max_selections: None,
                default_option_ids: None,
                display_order: Some(4),
                show_on_receipt: Some(true),
                receipt_name: None,
                show_on_kitchen_print: Some(true),
                kitchen_print_name: None,
                options: Some(vec![
                    opt("Pan", 0.50, 1),
                    opt("Alioli", 0.50, 2),
                    opt("Queso extra", 0.80, 3),
                ]),
            },
        },
    ]
}
