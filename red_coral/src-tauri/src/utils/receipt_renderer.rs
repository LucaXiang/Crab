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

        // Header Information Optimization
        b.align_left();

        // Line 1: FACTURA SIMPLIFICADA (Centered or Left) - Let's keep it bold
        b.bold_on();
        b.write_line("FACTURA SIMPLIFICADA");
        b.bold_off();

        // Line 2: Order ID + Date (Right aligned date)
        b.line_lr(
            &format!("Num: {}", self.receipt.order_id),
            &self.receipt.timestamp,
        );

        // Line 3: Mesa / Camarero / Comensales (if available)
        // We assume 'table_name' might contain guest info or we can just format it nicely
        // If we had guest count in receipt data, we would show it.
        // For now, let's show Table on left, and maybe "T: [Table]" if short.

        // Let's try to fit more info:
        // MESA: [Name]           OP: [Admin/User] (If we had op name)
        // Since we only have table_name, let's just make it clear.

        let zone_str = self.receipt.zone_name.as_deref().unwrap_or("");
        let table_full = format!("{} MESA: {}", zone_str, self.receipt.table_name);
        b.line_lr(table_full.trim(), "Terminal: 01");

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

        if let Some(surcharge) = &self.receipt.surcharge {
            b.dash_sep();
            b.align_center();
            b.bold_on();
            b.write_line(&format!("*** {} ***", surcharge.name.to_uppercase()));
            b.bold_off();
            b.align_left();
            let desc = if surcharge.type_ == "percentage" {
                format!("Suplemento ({}%)", surcharge.value)
            } else {
                format!("Suplemento ({:.2} €/ud)", surcharge.value).replace('.', ",")
            };
            if surcharge.amount.abs() < 0.001 {
                b.write(&desc);
                b.write(" ");
                b.bold_on();
                b.write("(EXENTO)");
                b.bold_off();
                b.write("\n");
            } else {
                b.write(&desc);
                b.write("\n");
            }
            b.dash_sep();
        }

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
            // Print selected options (if any)
            if let Some(options) = &item.selected_options {
                if !options.is_empty() {
                    let mut groups: Vec<(String, Vec<String>, f64)> = Vec::new();
                    for option in options {
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

            if let Some(dp) = item.discount_percent {
                if dp > 0.0 {
                    b.bold_on();
                    let before = item.original_price.unwrap_or(item.price);
                    let before_str = format!("{:.2} €", before).replace('.', ",");
                    // Layout: Indent(3) + Desc(20) + Space(6) + PVP(8) + Space(1) + Total(10)
                    // We want "ANTES 10,00 €" to align with PVP column (width 8)
                    // The "ANTES " part can spill left into the space/desc area if needed

                    let discount_text = format!("> DESC -{}%", dp.round() as i32);

                    // Layout: Indent(3) + Discount Text + Spacing + "ANTES" + Price
                    // We want the Price to align with the PVP column.
                    // PVP column: Start index 29, Width 8. Ends at index 36 (length 37).

                    let pvp_col_end_len = 37;
                    let before_width = get_gbk_width(&before_str);
                    let antes_str = " ANTES ";
                    let antes_width = get_gbk_width(antes_str);

                    let mut line = String::new();
                    line.push_str("   "); // Indent 3
                    line.push_str(&discount_text);

                    let current_len = get_gbk_width(&line);
                    let total_right_width = before_width + antes_width;

                    // Calculate required padding
                    // We want: current_len + padding + total_right_width = pvp_col_end_len
                    // padding = pvp_col_end_len - total_right_width - current_len

                    if pvp_col_end_len > total_right_width + current_len {
                        let padding = pvp_col_end_len - total_right_width - current_len;
                        line.push_str(&" ".repeat(padding));
                        line.push_str(antes_str);
                        line.push_str(&before_str);
                    } else {
                        // Fallback: just append with single space
                        line.push_str(antes_str);
                        line.push_str(&before_str);
                    }

                    b.write_line(&line);
                    b.bold_off();
                }
            }
        }

        b.eq_sep();
        // Group items by tax rate
        let mut tax_groups: std::collections::HashMap<i32, (f64, f64)> =
            std::collections::HashMap::new();

        // If items have tax_rate, use it. Otherwise use default 0.10
        let default_tax = 0.10;

        for item in &self.receipt.items {
            let rate = item.tax_rate.unwrap_or(default_tax);
            let rate_key = (rate * 100.0).round() as i32;

            let entry = tax_groups.entry(rate_key).or_insert((0.0, 0.0));
            // Calculate base for this item
            // item.total is tax inclusive
            let item_base = item.total / (1.0 + rate);
            let item_tax = item.total - item_base;

            entry.0 += item_base;
            entry.1 += item_tax;
        }

        b.align_left();
        // Layout: Margin(7) + Base(14) + Rate(6) + Tax(14) = 41 chars (centered in 48)
        // Layout: Left(20) + IVA(8) + Space(1) + Base(8) + Space(1) + Cuota(10) = 48 chars
        // Base aligns with PVP (cols 29-36), Cuota aligns with Importe (cols 38-47)
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

        // Layout: Label(23) + Space(1) + IVA(4) + Space(1) + Base(8) + Space(1) + Cuota(10) = 48
        // Base aligns with PVP (cols 29-36), Cuota aligns with Importe (cols 38-47)

        let left_header = pad_to_gbk_width(&left_text, 23, false);

        // Calculate total savings
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

        let h_iva = pad_to_gbk_width("IVA", 4, true);
        let h_base = pad_to_gbk_width("BASE IMP", 8, true);
        let h_cuota = pad_to_gbk_width("CUOTA", 10, true);

        // Actually, let's be precise with spaces
        b.write_line(&format!("{} {} {} {}", left_header, h_iva, h_base, h_cuota));

        // Sort by tax rate for consistent output
        let mut sorted_rates: Vec<_> = tax_groups.keys().cloned().collect();
        sorted_rates.sort();

        let mut total_base = 0.0;
        let mut total_tax = 0.0;

        for (i, rate_key) in sorted_rates.iter().enumerate() {
            // SAFETY: rate_key comes from tax_groups.keys() — lookup always succeeds
            let (base_amount, tax_amount) = tax_groups.get(rate_key).expect("rate_key from keys()");
            total_base += base_amount;
            total_tax += tax_amount;

            let base_str = format!("{:.2} €", base_amount).replace('.', ",");
            let rate_str = format!("{}%", rate_key);
            let tax_str = format!("{:.2} €", tax_amount).replace('.', ",");

            let col1 = pad_to_gbk_width(&rate_str, 4, true);
            let col2 = pad_to_gbk_width(&base_str, 8, true);
            let col3 = pad_to_gbk_width(&tax_str, 10, true);

            // Show savings in the first row if any
            let left_content = if i == 0 && total_savings > 0.005 {
                format!("AHORRO: -{:.2} €", total_savings).replace('.', ",")
            } else {
                "".to_string()
            };
            let left_col = pad_to_gbk_width(&left_content, 23, false);

            b.write_line(&format!("{} {} {} {}", left_col, col1, col2, col3));
        }

        // Render Subtotals
        // Padding is 23 + 1 + 4 + 1 = 29 spaces
        // Separator width: 8 + 1 + 10 = 19
        let sub_padding = " ".repeat(29);
        let sub_sep = format!("{}{}", sub_padding, "-".repeat(19));
        b.write_line(&sub_sep);

        let total_base_str = format!("{:.2} €", total_base).replace('.', ",");
        let total_tax_str = format!("{:.2} €", total_tax).replace('.', ",");
        // No rate column in subtotal
        let col_t2 = pad_to_gbk_width(&total_base_str, 8, true);
        let col_t3 = pad_to_gbk_width(&total_tax_str, 10, true);
        b.write_line(&format!("{}{} {}", sub_padding, col_t2, col_t3));

        b.write("\n");
        b.underscore_sep();
        b.write("\n");

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

        let qr_payload = self
            .receipt
            .qr_data
            .as_deref()
            .unwrap_or("https://verifactu.example/qr?ref=TEST-001");
        b.align_center();
        b.write("\x1D\x28\x6B\x04\x00\x31\x41\x31\x00");
        b.write("\x1D\x28\x6B\x03\x00\x31\x43\x06");
        b.write("\x1D\x28\x6B\x03\x00\x31\x45\x31");
        let data_bytes = qr_payload.as_bytes();
        let p_l = (data_bytes.len() + 3) as u8;
        let mut store_cmd = String::new();
        store_cmd.push('\x1D');
        store_cmd.push('(');
        store_cmd.push('k');
        store_cmd.push(char::from(p_l));
        store_cmd.push('\x00');
        store_cmd.push('\x31');
        store_cmd.push('\x50');
        store_cmd.push('\x30');
        b.write(&store_cmd);
        b.write(qr_payload);
        b.write("\x1D\x28\x6B\x03\x00\x31\x51\x30");
        b.write("\n\n");
        b.write("\n\n\n");
        b.write("\x1D\x56\x00");
        b.finalize()
    }
}
