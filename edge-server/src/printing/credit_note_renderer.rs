//! Credit note receipt renderer
//!
//! Renders CreditNoteDetail into ESC/POS format for thermal printers.
//! Layout follows the "NOTA DE CRÉDITO" format per design doc.

use chrono_tz::Tz;
use crab_printer::EscPosBuilder;
use shared::models::{CreditNoteDetail, receipt_text};

/// Credit note receipt renderer
pub struct CreditNoteReceiptRenderer {
    width: usize,
    timezone: Tz,
    locale: String,
    currency_symbol: String,
}

impl CreditNoteReceiptRenderer {
    pub fn new(width: usize, timezone: Tz, locale: String, currency_symbol: String) -> Self {
        Self {
            width,
            timezone,
            locale,
            currency_symbol,
        }
    }

    /// Render a credit note receipt to ESC/POS bytes
    pub fn render(&self, detail: &CreditNoteDetail) -> Vec<u8> {
        let txt = receipt_text(&self.locale);
        let mut b = EscPosBuilder::new(self.width);
        let cn = &detail.credit_note;

        // Title
        b.center();
        b.double_size();
        b.bold();
        b.line(txt.credit_note_title);
        b.bold_off();
        b.reset_size();

        b.sep_double();
        b.left();

        // Credit note number + date
        b.line_lr(
            &format!("{} {}", txt.credit_note_num_label, cn.credit_note_number),
            &format_timestamp(cn.created_at, self.timezone),
        );

        // Original receipt reference
        b.line(&format!(
            "{} {}",
            txt.original_receipt_label, cn.original_receipt
        ));

        b.sep_single();

        // Items header
        b.bold();
        self.render_item_header(&mut b, &txt);
        b.bold_off();
        b.sep_single();

        // Items
        for item in &detail.items {
            let qty_str = format!("x{}", item.quantity);
            let amount_str = format!("{:.2}", item.line_credit).replace('.', txt.decimal_separator);
            // Name column = width - qty(5) - amount(10) - spaces(2)
            let name_width = self.width.saturating_sub(17);
            let name = if item.item_name.len() > name_width {
                &item.item_name[..name_width]
            } else {
                &item.item_name
            };
            b.line(&format!(
                "{:<nw$} {:>5} {:>10}",
                name,
                qty_str,
                amount_str,
                nw = name_width
            ));
        }

        b.sep_single();

        // Amounts
        let subtotal_str = format!("{:.2} {}", cn.subtotal_credit, self.currency_symbol)
            .replace('.', txt.decimal_separator);
        b.line_lr(txt.credit_subtotal_label, &subtotal_str);

        let tax_str = format!("{:.2} {}", cn.tax_credit, self.currency_symbol)
            .replace('.', txt.decimal_separator);
        b.line_lr(txt.iva_label, &tax_str);

        b.sep_single();

        // Total (bold, double size)
        b.bold();
        b.double_size();
        let total_str = format!("{:.2} {}", cn.total_credit, self.currency_symbol)
            .replace('.', txt.decimal_separator);
        b.line_lr(txt.total_label, &total_str);
        b.reset_size();
        b.bold_off();

        b.sep_single();

        // Method + reason + operator
        let method_display = match cn.refund_method.as_str() {
            "CASH" => txt.refund_cash,
            "CARD" => txt.refund_card,
            other => other,
        };
        b.line_lr(txt.refund_method_label, method_display);
        b.line(&format!("{} {}", txt.refund_reason_label, cn.reason));

        if let Some(ref authorizer) = cn.authorizer_name {
            b.line(&format!("{} {}", txt.authorizer_label, authorizer));
        }
        b.line(&format!("{} {}", txt.cashier_label, cn.operator_name));

        b.sep_double();

        b.feed(6);
        b.cut();

        b.build()
    }

    fn render_item_header(&self, b: &mut EscPosBuilder, txt: &shared::models::ReceiptText) {
        let name_width = self.width.saturating_sub(17);
        b.line(&format!(
            "{:<nw$} {:>5} {:>10}",
            txt.col_article,
            txt.col_cant,
            txt.col_amount,
            nw = name_width
        ));
    }
}

impl Default for CreditNoteReceiptRenderer {
    fn default() -> Self {
        Self::new(
            48,
            chrono_tz::Europe::Madrid,
            "es-ES".to_string(),
            "EUR".to_string(),
        )
    }
}

/// Format unix timestamp (millis) to readable string in given timezone
fn format_timestamp(ts: i64, tz: Tz) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts) {
        dt.with_timezone(&tz).format("%d/%m/%Y %H:%M").to_string()
    } else {
        "--/--/---- --:--".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::{CreditNote, CreditNoteItem};

    fn test_detail() -> CreditNoteDetail {
        CreditNoteDetail {
            credit_note: CreditNote {
                id: 1,
                credit_note_number: "CN-20260227-0001".to_string(),
                original_order_pk: 100,
                original_receipt: "FAC20260227-0001".to_string(),
                subtotal_credit: 13.22,
                tax_credit: 2.78,
                total_credit: 16.00,
                refund_method: "CASH".to_string(),
                reason: "Calidad del producto".to_string(),
                note: None,
                operator_id: 1,
                operator_name: "María".to_string(),
                authorizer_id: Some(2),
                authorizer_name: Some("Manager".to_string()),
                shift_id: Some(10),
                cloud_synced: false,
                created_at: 1740667500000, // 2025-02-27 14:35 UTC
            },
            items: vec![
                CreditNoteItem {
                    id: 1,
                    credit_note_id: 1,
                    original_instance_id: "inst-1".to_string(),
                    item_name: "Paella".to_string(),
                    quantity: 1,
                    unit_price: 12.50,
                    line_credit: 12.50,
                    tax_rate: 1000,
                    tax_credit: 1.31,
                },
                CreditNoteItem {
                    id: 2,
                    credit_note_id: 1,
                    original_instance_id: "inst-2".to_string(),
                    item_name: "Cerveza".to_string(),
                    quantity: 1,
                    unit_price: 3.50,
                    line_credit: 3.50,
                    tax_rate: 1000,
                    tax_credit: 0.37,
                },
            ],
        }
    }

    #[test]
    fn test_render_credit_note_receipt() {
        let renderer = CreditNoteReceiptRenderer::new(
            48,
            chrono_tz::Europe::Madrid,
            "es-ES".to_string(),
            "EUR".to_string(),
        );
        let data = renderer.render(&test_detail());
        assert!(data.len() > 100);
    }

    #[test]
    fn test_render_58mm() {
        let renderer = CreditNoteReceiptRenderer::new(
            32,
            chrono_tz::Europe::Madrid,
            "es-ES".to_string(),
            "EUR".to_string(),
        );
        let data = renderer.render(&test_detail());
        assert!(data.len() > 100);
    }
}
