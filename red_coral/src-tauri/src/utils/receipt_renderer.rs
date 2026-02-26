use crate::api::ReceiptData;
use crate::utils::escpos_text::{get_gbk_width, pad_to_gbk_width, EscPosTextBuilder};

pub struct ReceiptRenderer<'a> {
    receipt: &'a ReceiptData,
    width: usize,
}

impl<'a> ReceiptRenderer<'a> {
    pub fn new(receipt: &'a ReceiptData, width: usize) -> Self {
        Self { receipt, width }
    }

    pub fn render(&self) -> String {
        let mut b = EscPosTextBuilder::new(self.width);
        b.align_center();
        if self.receipt.void_reason.is_some() {
            b.align_center();
            b.size_double();
            b.bold_on();
            b.write_line("*** ANULADO ***");
            b.bold_off();
            b.size_reset();
            b.write("\n");
        } else if self.receipt.pre_payment {
            b.align_center();
            b.size_double();
            b.bold_on();
            b.write_line("*** CUENTA ***");
            b.bold_off();
            b.size_reset();
            b.write("\n");
        } else if self.receipt.reprint {
            b.align_center();
            b.size_double();
            b.bold_on();
            b.write_line("*** REIMPRESION ***");
            b.bold_off();
            b.size_reset();
            b.write("\n");
        }
        if let Some(info) = &self.receipt.store_info {
            b.size_double();
            b.write_line(&info.name.to_string());
            b.size_reset();
            b.write_line(&info.address.to_string());
            b.write_line(&format!("CIF: {}", info.nif));
            if let Some(phone) = &info.phone {
                b.write_line(&format!("Tel: {}", phone));
            }
            if let Some(email) = &info.email {
                b.write_line(&format!("Email: {}", email));
            }
            if let Some(website) = &info.website {
                b.write_line(&website.to_string());
            }
            b.write("\n");
        }

        // Header
        b.align_left();
        b.bold_on();
        b.write_line("FACTURA SIMPLIFICADA");
        b.bold_off();

        b.line_lr(
            &format!("Num: {}", self.receipt.order_id),
            &self.receipt.timestamp,
        );

        if let Some(qn) = self.receipt.queue_number {
            let pedido_str = format!("PEDIDO: #{:03}", qn);
            b.line_lr(&pedido_str, "Terminal: 01");
        } else {
            let zone_str = self.receipt.zone_name.as_deref().unwrap_or("");
            let table_full = format!("{} MESA: {}", zone_str, self.receipt.table_name);
            b.line_lr(table_full.trim(), "Terminal: 01");
        }

        let guest_str = format!("Pers: {}", self.receipt.guest_count.unwrap_or(0));
        let opened_str = format!(
            "Apertura: {}",
            self.receipt.opened_at.as_deref().unwrap_or("")
        );
        b.line_lr(&guest_str, &opened_str);
        if let Some(checkout_time) = &self.receipt.checkout_time {
            b.line_lr("", &format!("Cierre:   {}", checkout_time));
        }
        if let Some(reason) = &self.receipt.void_reason {
            b.bold_on();
            b.write_line(&format!("ANULADO:  {}", reason));
            b.bold_off();
        }

        b.write("\n");

        // ── Items ──
        b.align_left();
        let h_uds = "UDS";
        let h_desc_padded = format!("{:<24}", "DESCRIPCION");
        let h_pvp = format!("{:>8}", "PVP");
        let h_importe = format!("{:>10}", "IMPORTE");
        b.write_line(&format!(
            "{} {} {} {}",
            h_uds, h_desc_padded, h_pvp, h_importe
        ));
        b.eq_sep();

        for item in &self.receipt.items {
            let qty_str = pad_to_gbk_width(&item.quantity.to_string(), 3, true);
            let name_str = pad_to_gbk_width(&item.name, 24, false);
            let price_str =
                pad_to_gbk_width(&format!("{:.2} €", item.price).replace('.', ","), 8, true);
            let total_str =
                pad_to_gbk_width(&format!("{:.2} €", item.total).replace('.', ","), 10, true);
            b.write_line(&format!(
                "{} {} {} {}",
                qty_str, name_str, price_str, total_str
            ));

            // Specification name
            if let Some(ref spec_name) = item.spec_name {
                if !spec_name.is_empty() {
                    b.write_line(&format!("   > {}", spec_name));
                }
            }

            // Selected options
            if let Some(options) = &item.selected_options {
                if !options.is_empty() {
                    let mut groups: Vec<(String, Vec<String>, f64)> = Vec::new();
                    for option in options {
                        if !option.show_on_receipt {
                            continue;
                        }
                        let attr_name = &option.attribute_name;
                        let display_name = option
                            .receipt_name
                            .as_deref()
                            .unwrap_or(&option.option_name)
                            .to_string();
                        let price = option.price_modifier;

                        if let Some(group) = groups.iter_mut().find(|g| &g.0 == attr_name) {
                            group.1.push(display_name);
                            group.2 += price;
                        } else {
                            groups.push((attr_name.clone(), vec![display_name], price));
                        }
                    }

                    for (attr_name, opt_names, total_price) in groups {
                        let opts_str = opt_names.join(",");
                        let option_line = if total_price.abs() < 0.001 {
                            format!("   > {}: {}", attr_name, opts_str)
                        } else if total_price > 0.0 {
                            format!("   > {}: {} (+{:.2} €)", attr_name, opts_str, total_price)
                                .replace('.', ",")
                        } else {
                            format!("   > {}: {} ({:.2} €)", attr_name, opts_str, total_price)
                                .replace('.', ",")
                        };
                        b.write_line(&option_line);
                    }
                }
            }

            // Manual discount sub-line
            if let Some(dp) = item.discount_percent {
                if dp > 0.0 {
                    b.bold_on();
                    let before = item.original_price.unwrap_or(item.price);
                    let before_str = format!("{:.2} €", before).replace('.', ",");

                    let discount_text = format!("> DESC -{}%", dp.round() as i32);

                    let pvp_col_end_len = 37;
                    let before_width = get_gbk_width(&before_str);
                    let antes_str = " ANTES ";
                    let antes_width = get_gbk_width(antes_str);

                    let mut line = String::new();
                    line.push_str("   ");
                    line.push_str(&discount_text);

                    let current_len = get_gbk_width(&line);
                    let total_right_width = before_width + antes_width;

                    if pvp_col_end_len > total_right_width + current_len {
                        let padding = pvp_col_end_len - total_right_width - current_len;
                        line.push_str(&" ".repeat(padding));
                        line.push_str(antes_str);
                        line.push_str(&before_str);
                    } else {
                        line.push_str(antes_str);
                        line.push_str(&before_str);
                    }

                    b.write_line(&line);
                    b.bold_off();
                }
            }
        }

        b.eq_sep();

        // ── Subtotal (items sum, before order-level adjustments) ──
        let items_subtotal: f64 = self.receipt.items.iter().map(|i| i.total).sum();

        // Check if there are any order-level adjustments
        let has_rule_adjustments = !self.receipt.rule_adjustments.is_empty();
        let has_manual_discount = self.receipt.discount.is_some();
        let has_manual_surcharge = self.receipt.surcharge.is_some();
        let has_adjustments = has_rule_adjustments || has_manual_discount || has_manual_surcharge;

        if has_adjustments {
            // Show subtotal line
            let subtotal_str = format!("{:.2} €", items_subtotal).replace('.', ",");
            b.line_lr("SUBTOTAL", &subtotal_str);
            b.dash_sep();
        }

        // ── Rule adjustments (整单级价格规则) ──
        for rule in &self.receipt.rule_adjustments {
            let (sign, prefix) = if rule.rule_type == "DISCOUNT" {
                ("-", "")
            } else {
                ("+", "+")
            };

            let desc = if rule.adjustment_type == "PERCENTAGE" {
                format!("{} ({}{}%)", rule.name, prefix, rule.value)
            } else {
                format!("{} ({}{:.2} €)", rule.name, prefix, rule.value).replace('.', ",")
            };

            let amount_str = format!("{}{:.2} €", sign, rule.amount).replace('.', ",");
            b.write_line(&format!(
                "{:<36}{:>10}",
                pad_to_gbk_width(&desc, 36, false),
                amount_str
            ));
        }

        // ── Manual order discount (整单手动折扣) ──
        if let Some(discount) = &self.receipt.discount {
            let desc = if discount.type_ == "percentage" {
                format!("{} (-{}%)", discount.name, discount.value)
            } else {
                format!("{} (-{:.2} €)", discount.name, discount.value).replace('.', ",")
            };
            let amount_str = format!("-{:.2} €", discount.amount).replace('.', ",");
            b.write_line(&format!(
                "{:<36}{:>10}",
                pad_to_gbk_width(&desc, 36, false),
                amount_str
            ));
        }

        // ── Manual order surcharge (整单手动附加费) ──
        if let Some(surcharge) = &self.receipt.surcharge {
            let desc = if surcharge.type_ == "percentage" {
                format!("{} (+{}%)", surcharge.name, surcharge.value)
            } else {
                format!("{} (+{:.2} €)", surcharge.name, surcharge.value).replace('.', ",")
            };
            let amount_str = format!("+{:.2} €", surcharge.amount).replace('.', ",");
            b.write_line(&format!(
                "{:<36}{:>10}",
                pad_to_gbk_width(&desc, 36, false),
                amount_str
            ));
        }

        if has_adjustments {
            b.dash_sep();
        }

        // ── Tax breakdown ──
        // Apportion order-level adjustments proportionally across tax groups
        // so that BASE IMP + CUOTA = TOTAL (fiscal compliance)
        let adjustment_ratio = if items_subtotal.abs() > 0.001 {
            self.receipt.total_amount / items_subtotal
        } else {
            1.0
        };

        let mut tax_groups: std::collections::HashMap<i32, (f64, f64)> =
            std::collections::HashMap::new();
        let default_tax = 0.10;

        for item in &self.receipt.items {
            let rate = item.tax_rate.unwrap_or(default_tax);
            let rate_key = (rate * 100.0).round() as i32;
            let entry = tax_groups.entry(rate_key).or_insert((0.0, 0.0));
            let adjusted_total = item.total * adjustment_ratio;
            let item_base = adjusted_total / (1.0 + rate);
            let item_tax = adjusted_total - item_base;
            entry.0 += item_base;
            entry.1 += item_tax;
        }

        b.align_left();
        let total_qty: f64 = self
            .receipt
            .items
            .iter()
            .map(|item| item.quantity as f64)
            .sum();
        let qty_display = if (total_qty.fract()).abs() < 1e-6 {
            format!("{:.0}", total_qty)
        } else {
            format!("{:.2}", total_qty)
        };
        let left_text = format!("Total Uds: {}", qty_display);
        let left_header = pad_to_gbk_width(&left_text, 23, false);

        // Calculate total savings (manual discounts + rule discounts)
        let mut total_savings = 0.0;
        for item in &self.receipt.items {
            if let Some(dp) = item.discount_percent {
                if dp > 0.0 {
                    let original = item.original_price.unwrap_or(item.price);
                    if original > item.price {
                        total_savings += (original - item.price) * item.quantity as f64;
                    }
                }
            }
        }
        // Add rule discount savings
        for rule in &self.receipt.rule_adjustments {
            if rule.rule_type == "DISCOUNT" {
                total_savings += rule.amount;
            }
        }
        // Add manual order discount
        if let Some(discount) = &self.receipt.discount {
            total_savings += discount.amount;
        }

        let h_iva = pad_to_gbk_width("IVA", 4, true);
        let h_base = pad_to_gbk_width("BASE IMP", 8, true);
        let h_cuota = pad_to_gbk_width("CUOTA", 10, true);
        b.write_line(&format!("{} {} {} {}", left_header, h_iva, h_base, h_cuota));

        let mut sorted_rates: Vec<_> = tax_groups.keys().cloned().collect();
        sorted_rates.sort();

        let mut total_base = 0.0;
        let mut total_tax = 0.0;

        for (i, rate_key) in sorted_rates.iter().enumerate() {
            // SAFETY: rate_key comes from tax_groups.keys() — HashMap::get with own key always succeeds
            let (base_amount, tax_amount) = tax_groups
                .get(rate_key)
                .expect("HashMap::get with key from own keys() is infallible");
            total_base += base_amount;
            total_tax += tax_amount;

            let base_str = format!("{:.2} €", base_amount).replace('.', ",");
            let rate_str = format!("{}%", rate_key);
            let tax_str = format!("{:.2} €", tax_amount).replace('.', ",");

            let col1 = pad_to_gbk_width(&rate_str, 4, true);
            let col2 = pad_to_gbk_width(&base_str, 8, true);
            let col3 = pad_to_gbk_width(&tax_str, 10, true);

            let left_content = if i == 0 && total_savings > 0.005 {
                format!("AHORRO: -{:.2} €", total_savings).replace('.', ",")
            } else {
                "".to_string()
            };
            let left_col = pad_to_gbk_width(&left_content, 23, false);

            b.write_line(&format!("{} {} {} {}", left_col, col1, col2, col3));
        }

        // Subtotals
        let sub_padding = " ".repeat(29);
        let sub_sep = format!("{}{}", sub_padding, "-".repeat(19));
        b.write_line(&sub_sep);

        let total_base_str = format!("{:.2} €", total_base).replace('.', ",");
        let total_tax_str = format!("{:.2} €", total_tax).replace('.', ",");
        let col_t2 = pad_to_gbk_width(&total_base_str, 8, true);
        let col_t3 = pad_to_gbk_width(&total_tax_str, 10, true);
        b.write_line(&format!("{}{} {}", sub_padding, col_t2, col_t3));

        b.write("\n");
        b.underscore_sep();
        b.write("\n");

        // ── TOTAL ──
        b.size_double();
        b.bold_on();
        let total_val = format!("{:.2} €", self.receipt.total_amount).replace('.', ",");
        let total_label = "TOTAL";
        let max_dw = 24;
        let lw = get_gbk_width(total_label);
        let vw = get_gbk_width(&total_val);
        if lw + vw < max_dw {
            let spaces = max_dw - lw - vw;
            b.write(total_label);
            b.write(&" ".repeat(spaces));
            b.write(&total_val);
        } else {
            b.write(total_label);
            b.write(" ");
            b.write(&total_val);
        }
        b.write("\n");
        b.bold_off();
        b.size_reset();

        b.write("\n\n");
        b.align_center();
        b.write_line("IVA INCLUIDO");
        b.bold_on();
        b.write_line("*** GRACIAS POR SU VISITA ***");
        b.bold_off();
        b.eq_sep();

        b.write("\n\n\n\n\n");
        b.write("\x1D\x56\x00");
        b.finalize()
    }
}
