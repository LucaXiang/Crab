export interface PaymentBreakdown {
  method: string;
  amount: number;
  count: number;
}

export interface RevenueTrendPoint {
  hour: number;
  revenue: number;
  orders: number;
}

export interface TopProduct {
  name: string;
  quantity: number;
  revenue: number;
}

export interface CategorySale {
  name: string;
  revenue: number;
}

export interface TaxBreakdownStat {
  tax_rate: number;
  base_amount: number;
  tax_amount: number;
}

export interface TagSale {
  name: string;
  color: string | null;
  revenue: number;
  quantity: number;
}

export interface RefundMethodBreakdown {
  method: string;
  amount: number;
  count: number;
}

export interface DailyTrendPoint {
  date: string;
  revenue: number;
  orders: number;
}

export interface StoreOverview {
  revenue: number;
  net_revenue: number;
  orders: number;
  guests: number;
  average_order_value: number;
  per_guest_spend: number;
  average_dining_minutes: number;
  total_tax: number;
  total_discount: number;
  voided_orders: number;
  voided_amount: number;
  loss_orders: number;
  loss_amount: number;
  anulacion_count: number;
  anulacion_amount: number;
  refund_count: number;
  refund_amount: number;
  revenue_trend: RevenueTrendPoint[];
  tax_breakdown: TaxBreakdownStat[];
  payment_breakdown: PaymentBreakdown[];
  top_products: TopProduct[];
  category_sales: CategorySale[];
  tag_sales: TagSale[];
  refund_method_breakdown: RefundMethodBreakdown[];
  daily_trend: DailyTrendPoint[];
  service_type_breakdown: ServiceTypeEntry[];
  zone_sales: ZoneSaleEntry[];
  total_surcharge: number;
  avg_items_per_order: number;
}

export interface ServiceTypeEntry {
  service_type: string;
  revenue: number;
  orders: number;
}

export interface ZoneSaleEntry {
  zone_name: string;
  revenue: number;
  orders: number;
  guests: number;
}

export interface DailyReportEntry {
  id: number;
  business_date: string;
  net_revenue: number;
  total_orders: number;
  refund_amount: number;
  refund_count: number;
  auto_generated: boolean;
  updated_at: number;
}

export interface ShiftBreakdown {
  id: number;
  shift_source_id: number;
  operator_id: number;
  operator_name: string;
  status: string;
  start_time: number;
  end_time: number | null;
  starting_cash: number;
  expected_cash: number;
  actual_cash: number | null;
  cash_variance: number | null;
  abnormal_close: boolean;
  completed_orders: number;
  void_orders: number;
  total_sales: number;
  total_paid: number;
  void_amount: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
}

export interface DailyReportDetail {
  id: number;
  business_date: string;
  net_revenue: number;
  total_orders: number;
  refund_amount: number;
  refund_count: number;
  auto_generated: boolean;
  generated_at: number | null;
  generated_by_id: number | null;
  generated_by_name: string | null;
  note: string | null;
  shift_breakdowns: ShiftBreakdown[];
}

export interface RedFlagsSummary {
  item_removals: number;
  item_comps: number;
  order_voids: number;
  order_discounts: number;
  price_modifications: number;
}

export interface OperatorRedFlags {
  operator_id: number | null;
  operator_name: string | null;
  item_removals: number;
  item_comps: number;
  order_voids: number;
  order_discounts: number;
  price_modifications: number;
  total_flags: number;
}

export interface RedFlagsResponse {
  summary: RedFlagsSummary;
  operator_breakdown: OperatorRedFlags[];
}

export interface ShiftEntry {
  source_id: number;
  operator_id: number;
  operator_name: string;
  status: string;
  start_time: number;
  end_time: number | null;
  starting_cash: number;
  expected_cash: number;
  actual_cash: number | null;
  cash_variance: number | null;
  abnormal_close: boolean;
  note: string | null;
}
