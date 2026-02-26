//! Kitchen ticket renderer
//!
//! Renders KitchenOrder data into ESC/POS format for thermal printers.

use chrono_tz::Tz;
use crab_printer::EscPosBuilder;

use super::types::{KitchenOrder, PrintItemContext};

/// Kitchen ticket renderer
///
/// Clean, minimal layout focused on what kitchen staff needs:
/// table/queue → time → items (qty first) → total count
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

        // Group items by category
        let grouped = self.group_by_category(&order.items);

        // Pre-calculate totals for header
        let total_kinds: usize = grouped.iter().map(|(_, items)| items.len()).sum();
        let total_qty: i32 = grouped
            .iter()
            .flat_map(|(_, items)| items.iter())
            .map(|item| item.quantity)
            .sum();

        self.render_header(&mut b, order, total_kinds, total_qty);

        // Single category → flat list, multiple → show category headers
        let show_categories = grouped.len() > 1;

        for (category_name, items) in &grouped {
            if show_categories {
                let cat_qty: i32 = items.iter().map(|i| i.quantity).sum();
                b.sep_single();
                b.bold();
                b.double_size();
                b.line(&format!("{} ({})", category_name, cat_qty));
                b.reset_size();
                b.bold_off();
            }

            for item in items {
                self.render_item(&mut b, item);
            }
        }

        self.render_footer(&mut b, order);

        b.build()
    }

    /// Header: table/queue (big) + zone|order_id + count|timestamp
    fn render_header(
        &self,
        b: &mut EscPosBuilder,
        order: &KitchenOrder,
        total_kinds: usize,
        total_qty: i32,
    ) {
        // Line 1: table/queue name (centered, double size, bold)
        b.center();
        b.double_size();
        b.bold();

        let title = if let Some(ref name) = order.table_name {
            name.clone()
        } else if let Some(n) = order.queue_number {
            format!("#{:03}", n)
        } else {
            "Para llevar".to_string()
        };
        b.line(&title);

        b.bold_off();
        b.reset_size();
        b.left();

        // Line 2: zone | receipt_number (+ service tag)
        let zone = order.zone_name.as_deref().unwrap_or("");
        let right = if order.is_retail && order.queue_number.is_none() {
            format!("{} [LLEVAR]", order.receipt_number)
        } else {
            order.receipt_number.clone()
        };
        b.line_lr(zone, &right);

        // Line 3: total count | timestamp
        let count_str = if total_kinds as i32 == total_qty {
            format!("{} uds", total_qty)
        } else {
            format!("{} uds ({} items)", total_qty, total_kinds)
        };
        let timestamp = format_timestamp(order.created_at, self.timezone);
        b.bold();
        b.line_lr(&count_str, &timestamp);
        b.bold_off();

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

                let category_name = items
                    .first()
                    .map(|i| i.category_name.clone())
                    .unwrap_or_default();

                (category_name, items)
            })
            .collect()
    }

    /// Column layout: QTY(4) + EXT_ID(5) + NAME(rest)
    /// Sub-lines (spec, options, note) indent to align under NAME column.
    const COL_QTY: usize = 4; // "  2x"
    const COL_EID: usize = 5; // " 0001"

    /// Render a single item with fixed-column layout
    fn render_item(&self, b: &mut EscPosBuilder, item: &PrintItemContext) {
        // Main line: "  2x 0001 Espresso"
        let qty_col = format!(
            "{:>width$}",
            format!("{}x", item.quantity),
            width = Self::COL_QTY
        );
        let eid_col = if let Some(ext_id) = item.external_id {
            format!("{:>width$}", ext_id, width = Self::COL_EID)
        } else {
            " ".repeat(Self::COL_EID)
        };

        let mut name = item.kitchen_name.clone();
        if let Some(ref index) = item.index {
            name.push_str(&format!(" [{}]", index));
        }

        // Item name — normal size
        b.line(&format!("{}{} {}", qty_col, eid_col, name));

        let prefix = " ".repeat(Self::COL_QTY);

        // Spec (规格) — bold
        if let Some(ref spec) = item.spec_name
            && !spec.is_empty()
        {
            b.bold();
            b.line(&format!("{} > SPEC: {}", prefix, spec));
            b.bold_off();
        }

        // Options (属性: 选项1, 选项2) — bold
        if !item.options.is_empty() {
            b.bold();
            for opt in &item.options {
                b.line(&format!("{} > {}", prefix, opt));
            }
            b.bold_off();
        }

        // Note (备注) — bold
        if let Some(ref note) = item.note
            && !note.is_empty()
        {
            b.bold();
            b.line(&format!("{} ** {} **", prefix, note));
            b.bold_off();
        }
    }

    /// Footer: reprint indicator
    fn render_footer(&self, b: &mut EscPosBuilder, order: &KitchenOrder) {
        b.sep_double();

        // Reprint indicator
        if order.print_count > 0 {
            b.newline();
            b.center();
            b.bold();
            b.line(&format!("** REIMPRESION #{} **", order.print_count));
            b.bold_off();
            b.left();
        }

        b.feed(2);
        b.cut();
    }
}

impl Default for KitchenTicketRenderer {
    fn default() -> Self {
        Self::new(48, chrono_tz::Europe::Madrid)
    }
}

/// Format unix timestamp (millis) to readable string in given timezone
fn format_timestamp(ts: i64, tz: Tz) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts) {
        dt.with_timezone(&tz)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    } else {
        "--:--".to_string()
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
            receipt_number: "FAC202401220001".to_string(),
            table_name: Some("B1".to_string()),
            zone_name: Some("Barra".to_string()),
            queue_number: None,
            is_retail: false,
            created_at: 1705912335000,
            items: vec![
                KitchenOrderItem {
                    context: PrintItemContext {
                        category_id: 1,
                        category_name: "Bebidas".to_string(),
                        product_id: 1,
                        external_id: Some(1),
                        kitchen_name: "Espresso".to_string(),
                        product_name: "Espresso".to_string(),
                        spec_name: None,
                        quantity: 1,
                        index: None,
                        options: vec![],
                        label_options: vec![],
                        note: None,
                        kitchen_destinations: vec!["kitchen-1".to_string()],
                        label_destinations: vec![],
                    },
                },
                KitchenOrderItem {
                    context: PrintItemContext {
                        category_id: 1,
                        category_name: "Bebidas".to_string(),
                        product_id: 2,
                        external_id: Some(2),
                        kitchen_name: "Matcha Latte".to_string(),
                        product_name: "Matcha Latte".to_string(),
                        spec_name: Some("Grande".to_string()),
                        quantity: 2,
                        index: None,
                        options: vec!["Azúcar: Sin azúcar".to_string()],
                        label_options: vec![],
                        note: Some("Extra caliente".to_string()),
                        kitchen_destinations: vec!["kitchen-1".to_string()],
                        label_destinations: vec![],
                    },
                },
            ],
            print_count: 0,
        }
    }

    fn create_multi_category_order() -> KitchenOrder {
        KitchenOrder {
            id: "evt-2".to_string(),
            order_id: "order-2".to_string(),
            receipt_number: "FAC202401220002".to_string(),
            table_name: Some("100桌".to_string()),
            zone_name: Some("大厅".to_string()),
            queue_number: None,
            is_retail: false,
            created_at: 1705912335000,
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
                        options: vec!["辣度: 微辣".to_string()],
                        label_options: vec!["微辣".to_string()],
                        note: Some("不要花生".to_string()),
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
                        label_options: vec![],
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
        assert!(data.len() > 100);
    }

    #[test]
    fn test_single_category_no_header() {
        let renderer = KitchenTicketRenderer::new(48, chrono_tz::Europe::Madrid);
        let order = create_test_order();
        // All items in same category — should NOT have category header
        let grouped = renderer.group_by_category(&order.items);
        assert_eq!(grouped.len(), 1);
    }

    #[test]
    fn test_multi_category_has_headers() {
        let renderer = KitchenTicketRenderer::new(48, chrono_tz::Europe::Madrid);
        let order = create_multi_category_order();
        let grouped = renderer.group_by_category(&order.items);
        assert_eq!(grouped.len(), 2);
    }

    #[test]
    fn test_reprint_indicator() {
        let renderer = KitchenTicketRenderer::new(48, chrono_tz::Europe::Madrid);
        let mut order = create_test_order();
        order.print_count = 2;
        let data = renderer.render(&order);
        // Should produce non-empty output (reprint adds more bytes)
        assert!(data.len() > 100);
    }
}
