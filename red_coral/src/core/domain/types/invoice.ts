/**
 * Invoice Types (Verifactu)
 *
 * 发票类型定义，与 Rust shared::models::invoice 对齐。
 */

/** Tipo de factura: F2 (simplified) or R5 (rectificativa) */
export type TipoFactura = 'F2' | 'R5';

/** Invoice source type */
export type InvoiceSourceType = 'ORDER' | 'CREDIT_NOTE';

/** AEAT submission status */
export type AeatStatus = 'PENDING' | 'SUBMITTED' | 'ACCEPTED' | 'REJECTED';

/** Invoice entity (matches backend Invoice) */
export interface Invoice {
  id: number;
  invoice_number: string;
  serie: string;
  tipo_factura: TipoFactura;
  source_type: InvoiceSourceType;
  source_pk: number;
  subtotal: number;
  tax: number;
  total: number;
  huella: string;
  prev_huella: string | null;
  fecha_expedicion: string;
  fecha_hora_registro: string;
  nif: string;
  nombre_razon: string;
  factura_rectificada_id: number | null;
  factura_rectificada_num: string | null;
  cloud_synced: boolean;
  aeat_status: AeatStatus;
  created_at: number;
}
