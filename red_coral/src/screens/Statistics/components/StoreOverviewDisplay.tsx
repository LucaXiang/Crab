import React from 'react';
import {
  BarChart3, DollarSign, ShoppingBag, Users, TrendingUp,
  CreditCard, Banknote, Clock, XCircle, AlertTriangle, Tag, Award, Receipt, RotateCcw,
  ArrowUpRight, ArrowDownRight, UtensilsCrossed, Map as MapIcon, Plus, Hash,
} from 'lucide-react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend, Line, ComposedChart,
} from 'recharts';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import type { StoreOverview } from '@/core/domain/types';

const PIE_COLORS = ['#ef4444', '#3b82f6', '#22c55e', '#f59e0b', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

const PAYMENT_I18N_KEYS: Record<string, string> = {
  CASH: 'checkout.method.cash',
  CARD: 'checkout.method.card',
};

const SERVICE_TYPE_I18N_KEYS: Record<string, string> = {
  DINE_IN: 'checkout.order_type.dine_in',
  TAKEOUT: 'checkout.order_type.takeout',
};

interface Props {
  overview: StoreOverview;
  previousOverview?: StoreOverview | null;
  lastWeekOverview?: StoreOverview | null;
  /** Business day cutoff in minutes from midnight for correct hourly trend ordering */
  cutoffMinutes?: number;
}

function pctChange(current: number, previous: number): number | null {
  if (previous === 0) return current > 0 ? null : null;
  return ((current - previous) / Math.abs(previous)) * 100;
}

export const StoreOverviewDisplay: React.FC<Props> = ({ overview, previousOverview, lastWeekOverview, cutoffMinutes = 0 }) => {
  const { t } = useI18n();

  const paymentLabel = (method: string): string => {
    const key = PAYMENT_I18N_KEYS[method.toUpperCase()];
    return key ? t(key) : method;
  };

  const serviceTypeLabel = (type: string): string => {
    const key = SERVICE_TYPE_I18N_KEYS[type.toUpperCase()];
    return key ? t(key) : type;
  };
  const prev = previousOverview ?? null;
  const lastWeek = lastWeekOverview ?? null;

  const totalCategorySales = overview.category_sales.reduce((sum, c) => sum + c.revenue, 0);
  const totalPayments = overview.payment_breakdown.reduce((sum, p) => sum + p.amount, 0);
  const hasDailyTrend = overview.daily_trend.length > 1;

  // Build hourly trend data — ordered by business day (cutoffHour → 23 → 0 → cutoffHour-1)
  const hourlyTrendData = (() => {
    const currentMap = new Map(overview.revenue_trend.map(p => [p.hour, p]));
    const prevMap = prev ? new Map(prev.revenue_trend.map(p => [p.hour, p])) : null;
    const lwMap = lastWeek ? new Map(lastWeek.revenue_trend.map(p => [p.hour, p])) : null;

    const allHours = new Set<number>();
    for (const p of overview.revenue_trend) allHours.add(p.hour);
    if (prev) for (const p of prev.revenue_trend) allHours.add(p.hour);
    if (lastWeek) for (const p of lastWeek.revenue_trend) allHours.add(p.hour);

    if (allHours.size === 0) return [];

    const cutoffHour = Math.floor(cutoffMinutes / 60);
    const toBizOrder = (h: number) => (h - cutoffHour + 24) % 24;
    const sorted = [...allHours].sort((a, b) => toBizOrder(a) - toBizOrder(b));

    const firstBiz = toBizOrder(sorted[0]);
    const lastBiz = toBizOrder(sorted[sorted.length - 1]);
    const hours: number[] = [];
    for (let i = firstBiz; i <= lastBiz; i++) {
      hours.push((cutoffHour + i) % 24);
    }

    return hours.map(h => ({
      hour: `${h}:00`,
      revenue: currentMap.get(h)?.revenue ?? 0,
      orders: currentMap.get(h)?.orders ?? 0,
      ...(prevMap ? { prevRevenue: prevMap.get(h)?.revenue ?? 0 } : {}),
      ...(lwMap ? { lwRevenue: lwMap.get(h)?.revenue ?? 0 } : {}),
    }));
  })();

  const dailyTrendData = overview.daily_trend.map(p => ({
    date: p.date.slice(5),
    revenue: p.revenue,
    orders: p.orders,
  }));

  return (
    <div className="space-y-4 md:space-y-6">
      {/* KPI Row 1 — Primary */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <KpiCard icon={DollarSign} bg="bg-green-100" color="text-green-600" value={formatCurrency(overview.net_revenue)} label={t('statistics.metric.net_revenue')} accent delta={prev ? pctChange(overview.net_revenue, prev.net_revenue) : undefined} />
        <KpiCard icon={ShoppingBag} bg="bg-blue-100" color="text-blue-600" value={String(overview.orders)} label={t('statistics.metric.orders')} delta={prev ? pctChange(overview.orders, prev.orders) : undefined} />
        <KpiCard icon={Users} bg="bg-purple-100" color="text-purple-600" value={String(overview.guests)} label={t('statistics.metric.customers')} delta={prev ? pctChange(overview.guests, prev.guests) : undefined} />
        <KpiCard icon={TrendingUp} bg="bg-orange-100" color="text-orange-600" value={formatCurrency(overview.average_order_value)} label={t('statistics.metric.avg_order_value')} delta={prev ? pctChange(overview.average_order_value, prev.average_order_value) : undefined} />
      </div>

      {/* KPI Row 2 — Secondary */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {overview.payment_breakdown.slice(0, 2).map(pb => {
          const isCash = pb.method.toUpperCase() === 'CASH';
          return (
            <KpiCard key={pb.method} icon={isCash ? Banknote : CreditCard} bg={isCash ? 'bg-emerald-100' : 'bg-indigo-100'} color={isCash ? 'text-emerald-600' : 'text-indigo-600'} value={formatCurrency(pb.amount)} label={`${paymentLabel(pb.method)} (${pb.count})`} />
          );
        })}
        <KpiCard icon={Users} bg="bg-teal-100" color="text-teal-600" value={formatCurrency(overview.per_guest_spend)} label={t('statistics.metric.avg_guest_spend')} delta={prev ? pctChange(overview.per_guest_spend, prev.per_guest_spend) : undefined} />
        <KpiCard icon={Clock} bg="bg-amber-100" color="text-amber-600" value={overview.average_dining_minutes > 0 ? `${Math.round(overview.average_dining_minutes)} min` : '-'} label={t('statistics.metric.avg_dining_time')} />
      </div>

      {/* KPI Row 3 — Losses, Discounts & Refunds */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <KpiCard icon={XCircle} bg="bg-red-100" color="text-red-600" value={String(overview.voided_orders)} label={`${t('statistics.metric.voided_orders')} (${formatCurrency(overview.voided_amount)})`} delta={prev ? pctChange(overview.voided_orders, prev.voided_orders) : undefined} invertDelta />
        <KpiCard icon={AlertTriangle} bg="bg-orange-100" color="text-orange-600" value={String(overview.loss_orders)} label={`${t('statistics.metric.loss_orders')} (${formatCurrency(overview.loss_amount)})`} delta={prev ? pctChange(overview.loss_orders, prev.loss_orders) : undefined} invertDelta />
        <KpiCard icon={RotateCcw} bg="bg-pink-100" color="text-pink-600" value={String(overview.refund_count)} label={`${t('statistics.metric.refunds')} (${formatCurrency(overview.refund_amount)})`} delta={prev ? pctChange(overview.refund_count, prev.refund_count) : undefined} invertDelta />
        <KpiCard icon={XCircle} bg="bg-rose-100" color="text-rose-600" value={String(overview.anulacion_count)} label={`${t('statistics.metric.anulacion')} (${formatCurrency(overview.anulacion_amount)})`} delta={prev ? pctChange(overview.anulacion_count, prev.anulacion_count) : undefined} invertDelta />
      </div>

      {/* KPI Row 4 — Tax, Surcharge, Discount & Items */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <KpiCard icon={DollarSign} bg="bg-green-50" color="text-green-500" value={formatCurrency(overview.revenue)} label={t('statistics.metric.gross_revenue')} delta={prev ? pctChange(overview.revenue, prev.revenue) : undefined} />
        <KpiCard icon={Tag} bg="bg-yellow-100" color="text-yellow-600" value={formatCurrency(overview.total_discount)} label={t('statistics.metric.total_discount')} delta={prev ? pctChange(overview.total_discount, prev.total_discount) : undefined} invertDelta />
        <KpiCard icon={Receipt} bg="bg-slate-100" color="text-slate-600" value={formatCurrency(overview.total_tax)} label={t('statistics.metric.total_tax')} delta={prev ? pctChange(overview.total_tax, prev.total_tax) : undefined} />
        {overview.total_surcharge > 0 && (
          <KpiCard icon={Plus} bg="bg-cyan-100" color="text-cyan-600" value={formatCurrency(overview.total_surcharge)} label={t('statistics.metric.total_surcharge')} delta={prev ? pctChange(overview.total_surcharge, prev.total_surcharge) : undefined} />
        )}
        <KpiCard icon={Hash} bg="bg-violet-100" color="text-violet-600" value={overview.avg_items_per_order > 0 ? overview.avg_items_per_order.toFixed(1) : '-'} label={t('statistics.metric.avg_items_per_order')} delta={prev ? pctChange(overview.avg_items_per_order, prev.avg_items_per_order) : undefined} />
      </div>

      {/* Daily Revenue Trend — for cross-day ranges */}
      {hasDailyTrend && (
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.metric.daily_trend')}</h3>
          </div>
          <ResponsiveContainer width="100%" height={220}>
            <AreaChart data={dailyTrendData} margin={{ top: 4, right: 4, left: -10, bottom: 0 }}>
              <defs>
                <linearGradient id="colorDailyRevenue" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ef4444" stopOpacity={0.2} />
                  <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
              <XAxis dataKey="date" tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} />
              <YAxis tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} tickFormatter={v => formatCurrency(v)} />
              <Tooltip
                contentStyle={{ borderRadius: 12, border: '1px solid #e2e8f0', boxShadow: '0 4px 12px rgba(0,0,0,.08)', fontSize: 13 }}
                formatter={(value: number | undefined, name: string | undefined) => [
                  name === 'revenue' ? formatCurrency(value ?? 0) : (value ?? 0),
                  name === 'revenue' ? t('statistics.metric.revenue') : t('statistics.metric.orders'),
                ]}
              />
              <Area type="monotone" dataKey="revenue" stroke="#ef4444" strokeWidth={2} fill="url(#colorDailyRevenue)" />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Hourly Revenue Trend — with comparison lines */}
      {overview.revenue_trend.length > 0 && (
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.chart.revenue_trend')}</h3>
          </div>
          <ResponsiveContainer width="100%" height={220}>
            <ComposedChart data={hourlyTrendData} margin={{ top: 4, right: 4, left: -10, bottom: 0 }}>
              <defs>
                <linearGradient id="colorRevenue" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ef4444" stopOpacity={0.2} />
                  <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
              <XAxis dataKey="hour" tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} />
              <YAxis tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} tickFormatter={v => formatCurrency(v)} />
              <Tooltip
                contentStyle={{ borderRadius: 12, border: '1px solid #e2e8f0', boxShadow: '0 4px 12px rgba(0,0,0,.08)', fontSize: 13 }}
                formatter={(value: number | undefined, name: string | undefined) => {
                  const labels: Record<string, string> = {
                    revenue: t('statistics.metric.line_today'),
                    prevRevenue: t('statistics.metric.line_yesterday'),
                    lwRevenue: t('statistics.metric.line_last_week'),
                  };
                  return [formatCurrency(value ?? 0), labels[name ?? ''] ?? name];
                }}
              />
              <Legend
                wrapperStyle={{ fontSize: 12 }}
                formatter={(value: string) => {
                  const labels: Record<string, string> = {
                    revenue: t('statistics.metric.line_today'),
                    prevRevenue: t('statistics.metric.line_yesterday'),
                    lwRevenue: t('statistics.metric.line_last_week'),
                  };
                  return labels[value] ?? value;
                }}
              />
              <Area type="monotone" dataKey="revenue" stroke="#ef4444" strokeWidth={2} fill="url(#colorRevenue)" name="revenue" />
              {prev && <Line type="monotone" dataKey="prevRevenue" stroke="#94a3b8" strokeWidth={1.5} strokeDasharray="6 3" dot={false} name="prevRevenue" />}
              {lastWeek && <Line type="monotone" dataKey="lwRevenue" stroke="#3b82f6" strokeWidth={1.5} strokeDasharray="3 3" dot={false} name="lwRevenue" />}
            </ComposedChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Two columns: Payment Breakdown Pie + Tax Breakdown */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <CreditCard className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.metric.payment_breakdown')}</h3>
          </div>
          {overview.payment_breakdown.length > 0 ? (
            <div className="flex flex-col items-center gap-4 md:flex-row md:items-center">
              <div className="w-36 h-36 shrink-0">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie data={overview.payment_breakdown} dataKey="amount" nameKey="method" cx="50%" cy="50%" innerRadius={30} outerRadius={60} paddingAngle={2}>
                      {overview.payment_breakdown.map((_, i) => (
                        <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
                      ))}
                    </Pie>
                  </PieChart>
                </ResponsiveContainer>
              </div>
              <div className="flex-1 space-y-2">
                {overview.payment_breakdown.map((pb, i) => {
                  const pct = totalPayments > 0 ? ((pb.amount / totalPayments) * 100).toFixed(1) : '0';
                  return (
                    <div key={pb.method} className="flex items-center justify-between text-sm">
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="w-2.5 h-2.5 rounded-full shrink-0" style={{ backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                        <span className="text-slate-700 truncate">{paymentLabel(pb.method)}</span>
                      </div>
                      <div className="flex items-center gap-3 shrink-0 ml-2">
                        <span className="text-xs text-slate-400">{pct}%</span>
                        <span className="font-semibold text-slate-900 w-20 text-right">{formatCurrency(pb.amount)}</span>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : <EmptySection />}
        </div>

        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <Receipt className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.metric.tax_breakdown')}</h3>
          </div>
          {overview.tax_breakdown.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-slate-100">
                    <th className="text-left py-2 text-xs font-medium text-slate-400">{t('statistics.metric.tax_rate')}</th>
                    <th className="text-right py-2 text-xs font-medium text-slate-400">{t('statistics.metric.tax_base')}</th>
                    <th className="text-right py-2 text-xs font-medium text-slate-400">{t('statistics.metric.tax_amount')}</th>
                  </tr>
                </thead>
                <tbody>
                  {overview.tax_breakdown.map((tax, i) => (
                    <tr key={i} className="border-b border-slate-50 last:border-0">
                      <td className="py-2 text-slate-700 font-medium">{tax.tax_rate}%</td>
                      <td className="py-2 text-right text-slate-700">{formatCurrency(tax.base_amount)}</td>
                      <td className="py-2 text-right font-semibold text-slate-900">{formatCurrency(tax.tax_amount)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : <EmptySection />}
        </div>
      </div>

      {/* Service Type & Zone Sales */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <UtensilsCrossed className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.metric.service_type')}</h3>
          </div>
          {overview.service_type_breakdown.length > 0 ? (() => {
            const totalSvcOrders = overview.service_type_breakdown.reduce((sum, s) => sum + s.orders, 0);
            return (
              <div className="flex flex-col items-center gap-4 md:flex-row md:items-center">
                <div className="w-36 h-36 shrink-0">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <Pie data={overview.service_type_breakdown} dataKey="orders" nameKey="service_type" cx="50%" cy="50%" innerRadius={30} outerRadius={60} paddingAngle={2}>
                        {overview.service_type_breakdown.map((_, i) => (
                          <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
                        ))}
                      </Pie>
                    </PieChart>
                  </ResponsiveContainer>
                </div>
                <div className="flex-1 space-y-2">
                  {overview.service_type_breakdown.map((st, i) => {
                    const pct = totalSvcOrders > 0 ? ((st.orders / totalSvcOrders) * 100).toFixed(1) : '0';
                    return (
                      <div key={st.service_type} className="flex items-center justify-between text-sm">
                        <div className="flex items-center gap-2 min-w-0">
                          <span className="w-2.5 h-2.5 rounded-full shrink-0" style={{ backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                          <span className="text-slate-700 truncate">{serviceTypeLabel(st.service_type)}</span>
                        </div>
                        <div className="flex items-center gap-3 shrink-0 ml-2">
                          <span className="text-xs text-slate-400">{pct}% · {st.orders}</span>
                          <span className="font-semibold text-slate-900 w-20 text-right">{formatCurrency(st.revenue)}</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })() : <EmptySection />}
        </div>

        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <MapIcon className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.metric.zone_sales')}</h3>
          </div>
          {overview.zone_sales.length > 0 ? (() => {
            const totalZoneRev = overview.zone_sales.reduce((sum, z) => sum + z.revenue, 0);
            return (
              <div className="space-y-3">
                {overview.zone_sales.map((zone, i) => {
                  const pct = totalZoneRev > 0 ? (zone.revenue / totalZoneRev) * 100 : 0;
                  return (
                    <div key={zone.zone_name}>
                      <div className="flex items-center justify-between text-sm mb-1">
                        <div className="flex items-center gap-2">
                          <span className="w-2.5 h-2.5 rounded-full shrink-0" style={{ backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                          <span className="text-slate-700">{zone.zone_name}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-slate-400">{zone.orders} · {zone.guests}p</span>
                          <span className="font-semibold text-slate-900">{formatCurrency(zone.revenue)}</span>
                        </div>
                      </div>
                      <div className="h-2 bg-slate-100 rounded-full overflow-hidden">
                        <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                      </div>
                    </div>
                  );
                })}
              </div>
            );
          })() : <EmptySection />}
        </div>
      </div>

      {/* Refund Method Breakdown */}
      <div className="bg-white rounded-2xl border border-slate-200 p-6">
        <div className="flex items-center gap-2 mb-4">
          <RotateCcw className="w-5 h-5 text-slate-400" />
          <h3 className="font-bold text-slate-900">{t('statistics.metric.refund_methods')}</h3>
        </div>
        {overview.refund_method_breakdown.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-100">
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('statistics.metric.method')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('statistics.metric.count')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('statistics.metric.amount')}</th>
                </tr>
              </thead>
              <tbody>
                {overview.refund_method_breakdown.map((rm, i) => (
                  <tr key={i} className="border-b border-slate-50 last:border-0">
                    <td className="py-2 text-slate-700 font-medium">{paymentLabel(rm.method)}</td>
                    <td className="py-2 text-right text-slate-700">{rm.count}</td>
                    <td className="py-2 text-right font-semibold text-slate-900">{formatCurrency(rm.amount)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : <EmptySection />}
      </div>

      {/* Tag Sales */}
      <div className="bg-white rounded-2xl border border-slate-200 p-6">
        <div className="flex items-center gap-2 mb-4">
          <Tag className="w-5 h-5 text-slate-400" />
          <h3 className="font-bold text-slate-900">{t('statistics.metric.tag_sales')}</h3>
        </div>
        {overview.tag_sales.length > 0 ? (
          <div className="flex flex-wrap gap-3">
            {overview.tag_sales.map((tag, i) => (
              <div key={i} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-100 bg-slate-50">
                <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: tag.color || PIE_COLORS[i % PIE_COLORS.length] }} />
                <span className="text-sm font-medium text-slate-700">{tag.name}</span>
                <span className="text-xs text-slate-400">{tag.quantity}x</span>
                <span className="text-sm font-semibold text-slate-900">{formatCurrency(tag.revenue)}</span>
              </div>
            ))}
          </div>
        ) : <EmptySection />}
      </div>

      {/* Two columns: Top Products + Category Sales */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <Award className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.top_products')}</h3>
          </div>
          {overview.top_products.length > 0 ? (
            <div className="space-y-2">
              {overview.top_products.map((product, i) => {
                const maxQty = overview.top_products[0]?.quantity ?? 1;
                const barPct = (product.quantity / maxQty) * 100;
                return (
                  <div key={i} className="relative">
                    <div className="absolute inset-y-0 left-0 rounded bg-blue-50" style={{ width: `${barPct}%` }} />
                    <div className="relative flex items-center justify-between py-2 px-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="text-xs font-bold text-blue-400 w-5 text-right">{i + 1}</span>
                        <span className="text-sm text-slate-700 truncate">{product.name}</span>
                      </div>
                      <div className="flex items-center gap-3 shrink-0 ml-2">
                        <span className="text-xs text-slate-400">{product.quantity}x</span>
                        <span className="text-sm font-semibold text-slate-900 w-20 text-right">{formatCurrency(product.revenue)}</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : <EmptySection />}
        </div>

        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <BarChart3 className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('statistics.chart.sales_by_category')}</h3>
          </div>
          {overview.category_sales.length > 0 ? (
            <div className="space-y-3">
              {overview.category_sales.map((cat, i) => {
                const pct = totalCategorySales > 0 ? (cat.revenue / totalCategorySales) * 100 : 0;
                return (
                  <div key={i}>
                    <div className="flex items-center justify-between text-sm mb-1">
                      <div className="flex items-center gap-2">
                        <span className="w-2.5 h-2.5 rounded-full shrink-0" style={{ backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                        <span className="text-slate-700">{cat.name}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-slate-400">{pct.toFixed(1)}%</span>
                        <span className="font-semibold text-slate-900">{formatCurrency(cat.revenue)}</span>
                      </div>
                    </div>
                    <div className="h-2 bg-slate-100 rounded-full overflow-hidden">
                      <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, backgroundColor: PIE_COLORS[i % PIE_COLORS.length] }} />
                    </div>
                  </div>
                );
              })}
            </div>
          ) : <EmptySection />}
        </div>
      </div>
    </div>
  );
};

const EmptySection: React.FC = () => (
  <p className="text-sm text-slate-400 py-4 text-center">-</p>
);

const KpiCard: React.FC<{
  icon: React.FC<{ className?: string }>;
  bg: string;
  color: string;
  value: string;
  label: string;
  accent?: boolean;
  delta?: number | null;
  invertDelta?: boolean;
}> = ({ icon: Icon, bg, color, value, label, accent, delta, invertDelta }) => {
  let deltaEl: React.ReactNode = null;
  if (delta !== undefined && delta !== null) {
    const isUp = delta > 0;
    const isGood = invertDelta ? !isUp : isUp;
    const DeltaIcon = isUp ? ArrowUpRight : ArrowDownRight;
    deltaEl = (
      <span className={`inline-flex items-center gap-0.5 text-xs font-medium ${isGood ? 'text-green-600' : 'text-red-500'}`}>
        <DeltaIcon className="w-3 h-3" />
        {Math.abs(delta).toFixed(1)}%
      </span>
    );
  }

  return (
    <div className={`bg-white rounded-xl border ${accent ? 'border-green-200 ring-1 ring-green-100' : 'border-slate-200'} p-4`}>
      <div className="flex items-start justify-between mb-2">
        <div className={`w-8 h-8 ${bg} rounded-lg flex items-center justify-center`}>
          <Icon className={`w-4 h-4 ${color}`} />
        </div>
        {deltaEl}
      </div>
      <p className={`text-lg font-bold ${accent ? 'text-green-600' : 'text-slate-900'}`}>{value}</p>
      <p className="text-xs text-slate-400">{label}</p>
    </div>
  );
};
