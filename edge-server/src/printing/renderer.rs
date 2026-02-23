//! Kitchen ticket renderer
//!
//! Renders KitchenOrder data into ESC/POS format for thermal printers.

use chrono_tz::Tz;
use crab_printer::EscPosBuilder;

use super::types::{KitchenOrder, PrintItemContext};

/// Kitchen ticket renderer
///
/// Renders kitchen orders for thermal printers.
/// Groups items by category and sorts by external_id.
pub struct KitchenTicketRenderer {
    width: usize,
    timezone: Tz,
}

impl KitchenTicketRenderer {
    /// Create a new renderer with specified paper width and timezone
    ///
    /// Common widths:
    /// - 58mm paper: 32 characters
    /// - 80mm paper: 48 characters
    pub fn new(width: usize, timezone: Tz) -> Self {
        Self { width, timezone }
    }

    /// Render a kitchen order to ESC/POS bytes
    pub fn render(&self, order: &KitchenOrder) -> Vec<u8> {
        let mut b = EscPosBuilder::new(self.width);

        // Header: Table name + timestamp
        self.render_header(&mut b, order);

        // Group items by category
        let grouped = self.group_by_category(&order.items);

        // Render each category group
        for (category_name, items) in grouped {
            self.render_category(&mut b, &category_name, &items);
        }

        // Footer
        self.render_footer(&mut b, order);

        b.build()
    }

    /// Render the header section
    fn render_header(&self, b: &mut EscPosBuilder, order: &KitchenOrder) {
        // Table name (large, centered)
        b.center();
        b.double_size();
        b.bold();

        let table_name = order.table_name.as_deref().unwrap_or("外卖");
        b.line(table_name);

        b.bold_off();
        b.reset_size();

        // Timestamp (in configured timezone)
        let timestamp = format_timestamp(order.created_at, self.timezone);
        b.line(&timestamp);

        b.left();
        b.sep_double();
    }

    /// Group items by category, sorted by category_id
    fn group_by_category<'a>(
        &self,
        items: &'a [super::types::KitchenOrderItem],
    ) -> Vec<(String, Vec<&'a PrintItemContext>)> {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<i64, Vec<&PrintItemContext>> = BTreeMap::new();

        for item in items {
            groups
                .entry(item.context.category_id)
                .or_default()
                .push(&item.context);
        }

        // Convert to vec with category names, sort items by external_id
        groups
            .into_values()
            .map(|mut items| {
                // Sort by external_id (nulls last)
                items.sort_by(|a, b| match (a.external_id, b.external_id) {
                    (Some(a_id), Some(b_id)) => a_id.cmp(&b_id),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });

                // Get category name from first item
                let category_name = items
                    .first()
                    .map(|i| i.category_name.clone())
                    .unwrap_or_default();

                (category_name, items)
            })
            .collect()
    }

    /// Render a category section
    fn render_category(
        &self,
        b: &mut EscPosBuilder,
        category_name: &str,
        items: &[&PrintItemContext],
    ) {
        // Category header
        b.bold();
        b.line(&format!("【{}】", category_name));
        b.bold_off();

        // Render each item
        for item in items {
            self.render_item(b, item);
        }

        b.sep_single();
    }

    /// Render a single item
    fn render_item(&self, b: &mut EscPosBuilder, item: &PrintItemContext) {
        // Item line: #001 商品名 (规格) x2
        let mut line = String::new();

        // External ID (product number)
        if let Some(ext_id) = item.external_id {
            line.push_str(&format!("#{:03} ", ext_id));
        } else {
            line.push_str("     "); // 5 spaces padding
        }

        // Kitchen name
        line.push_str(&item.kitchen_name);

        // Spec name (skip empty names)
        if let Some(ref spec) = item.spec_name
            && !spec.is_empty()
        {
            line.push_str(&format!(" ({})", spec));
        }

        // Quantity
        if item.quantity > 1 {
            line.push_str(&format!(" x{}", item.quantity));
        }

        // Index for labels (e.g., "2/5")
        if let Some(ref index) = item.index {
            line.push_str(&format!(" [{}]", index));
        }

        b.double_height();
        b.line(&line);
        b.reset_size();

        // Options (做法)
        for opt in &item.options {
            b.line(&format!("     - {}", opt));
        }

        // Note (备注)
        if let Some(ref note) = item.note
            && !note.is_empty()
        {
            b.bold();
            b.line(&format!("     * {}", note));
            b.bold_off();
        }
    }

    /// Render the footer section
    fn render_footer(&self, b: &mut EscPosBuilder, order: &KitchenOrder) {
        // Reprint indicator
        if order.print_count > 0 {
            b.newline();
            b.center();
            b.bold();
            b.line(&format!("*** 补打 #{} ***", order.print_count));
            b.bold_off();
            b.left();
        }

        // Feed and cut
        b.feed(3);
        b.cut();
    }
}

impl Default for KitchenTicketRenderer {
    fn default() -> Self {
        Self::new(48, chrono_tz::Europe::Madrid)
    }
}

/// Format unix timestamp (millis) to readable string (MM-DD HH:mm:ss) in given timezone
fn format_timestamp(ts: i64, tz: Tz) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts) {
        dt.with_timezone(&tz).format("%m-%d %H:%M:%S").to_string()
    } else {
        "时间未知".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printing::types::KitchenOrderItem;

    fn create_test_order() -> KitchenOrder {
        KitchenOrder {
            id: "evt-1".to_string(),
            order_id: "order-1".to_string(),
            table_name: Some("100桌".to_string()),
            created_at: 1705912335000, // 2024-01-22 14:32:15 UTC (millis)
            items: vec![
                KitchenOrderItem {
                    context: PrintItemContext {
                        category_id: 1,
                        category_name: "热菜".to_string(),
                        product_id: 1,
                        external_id: Some(1),
                        kitchen_name: "宫保鸡丁".to_string(),
                        product_name: "宫保鸡丁".to_string(),
                        spec_name: Some("大".to_string()),
                        quantity: 2,
                        index: None,
                        options: vec!["微辣".to_string()],
                        note: Some("不要花生".to_string()),
                        kitchen_destinations: vec!["kitchen-1".to_string()],
                        label_destinations: vec![],
                    },
                },
                KitchenOrderItem {
                    context: PrintItemContext {
                        category_id: 1,
                        category_name: "热菜".to_string(),
                        product_id: 2,
                        external_id: Some(3),
                        kitchen_name: "红烧肉".to_string(),
                        product_name: "红烧肉".to_string(),
                        spec_name: None,
                        quantity: 1,
                        index: None,
                        options: vec![],
                        note: None,
                        kitchen_destinations: vec!["kitchen-1".to_string()],
                        label_destinations: vec![],
                    },
                },
                KitchenOrderItem {
                    context: PrintItemContext {
                        category_id: 2,
                        category_name: "凉菜".to_string(),
                        product_id: 3,
                        external_id: Some(15),
                        kitchen_name: "凉拌黄瓜".to_string(),
                        product_name: "凉拌黄瓜".to_string(),
                        spec_name: None,
                        quantity: 1,
                        index: None,
                        options: vec![],
                        note: Some("少放蒜".to_string()),
                        kitchen_destinations: vec!["kitchen-1".to_string()],
                        label_destinations: vec![],
                    },
                },
            ],
            print_count: 0,
        }
    }

    #[test]
    fn test_render_kitchen_ticket() {
        let renderer = KitchenTicketRenderer::new(48, chrono_tz::Europe::Madrid);
        let order = create_test_order();

        let data = renderer.render(&order);

        // Should produce non-empty output
        assert!(!data.is_empty());

        // Check for some expected content (GBK encoded, so check raw bytes exist)
        assert!(data.len() > 100);
    }

    #[test]
    fn test_group_by_category() {
        let renderer = KitchenTicketRenderer::new(48, chrono_tz::Europe::Madrid);
        let order = create_test_order();

        let grouped: Vec<_> = renderer
            .group_by_category(&order.items)
            .into_iter()
            .map(|(name, items)| (name, items.len()))
            .collect();

        // Should have 2 categories
        assert_eq!(grouped.len(), 2);
    }
}
