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

export interface StoreOverview {
  revenue: number;
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
  revenue_trend: RevenueTrendPoint[];
  tax_breakdown: TaxBreakdownStat[];
  payment_breakdown: PaymentBreakdown[];
  top_products: TopProduct[];
  category_sales: CategorySale[];
  tag_sales: TagSale[];
}

export interface DailyReportEntry {
  id: number;
  business_date: string;
  total_orders: number;
  completed_orders: number;
  void_orders: number;
  total_sales: number;
  total_paid: number;
  total_unpaid: number;
  void_amount: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
  updated_at: number;
}

export interface ShiftBreakdown {
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
  total_orders: number;
  completed_orders: number;
  void_orders: number;
  total_sales: number;
  total_paid: number;
  void_amount: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
}

export interface TaxBreakdownDetail {
  tax_rate: number;
  net_amount: number;
  tax_amount: number;
  gross_amount: number;
  order_count: number;
}

export interface PaymentBreakdownDetail {
  method: string;
  amount: number;
  count: number;
}

export interface DailyReportDetail {
  id: number;
  business_date: string;
  total_orders: number;
  completed_orders: number;
  void_orders: number;
  total_sales: number;
  total_paid: number;
  total_unpaid: number;
  void_amount: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
  generated_at: number | null;
  generated_by_id: number | null;
  generated_by_name: string | null;
  note: string | null;
  tax_breakdowns: TaxBreakdownDetail[];
  payment_breakdowns: PaymentBreakdownDetail[];
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
