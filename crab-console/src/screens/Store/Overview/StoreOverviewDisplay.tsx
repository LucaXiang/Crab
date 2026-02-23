import React from 'react';
import {
  BarChart3, DollarSign, ShoppingBag, Users, TrendingUp,
  CreditCard, Clock, XCircle, AlertTriangle, Tag, Award, Receipt,
} from 'lucide-react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell,
} from 'recharts';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/format';
import type { StoreOverview } from '@/core/types/stats';

const PIE_COLORS = ['#ef4444', '#3b82f6', '#22c55e', '#f59e0b', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

interface Props {
  overview: StoreOverview;
  showHeader?: boolean;
}

export const StoreOverviewDisplay: React.FC<Props> = ({ overview, showHeader = true }) => {
  const { t } = useI18n();

  const totalCategorySales = overview.category_sales.reduce((sum, c) => sum + c.revenue, 0);
  const totalPayments = overview.payment_breakdown.reduce((sum, p) => sum + p.amount, 0);

  const trendData = overview.revenue_trend.map(p => ({
    hour: `${p.hour}:00`,
    revenue: p.revenue,
    orders: p.orders,
  }));

  return (
    <div className="space-y-4 md:space-y-6">
      {showHeader && (
        <div className="bg-white rounded-2xl border border-slate-200 px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <BarChart3 className="w-5 h-5 text-primary-500" />
            <span className="font-bold text-slate-900">{t('stats.today')}</span>
          </div>
          <span className="text-sm text-slate-400">{new Date().toLocaleDateString()}</span>
        </div>
      )}

      {/* KPI Row 1 — Primary */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <KpiCard icon={DollarSign} bg="bg-primary-100" color="text-primary-600" value={formatCurrency(overview.revenue)} label={t('stats.total_sales')} accent />
        <KpiCard icon={ShoppingBag} bg="bg-green-100" color="text-green-600" value={String(overview.orders)} label={t('stats.completed_orders')} />
        <KpiCard icon={Users} bg="bg-blue-100" color="text-blue-600" value={String(overview.guests)} label={t('stats.guests')} />
        <KpiCard icon={TrendingUp} bg="bg-purple-100" color="text-purple-600" value={formatCurrency(overview.average_order_value)} label={t('stats.average_order')} />
      </div>

      {/* KPI Row 2 — Secondary */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {overview.payment_breakdown.slice(0, 2).map(pb => (
          <KpiCard key={pb.method} icon={CreditCard} bg="bg-indigo-100" color="text-indigo-600" value={formatCurrency(pb.amount)} label={`${pb.method} (${pb.count})`} />
        ))}
        <KpiCard icon={Users} bg="bg-teal-100" color="text-teal-600" value={formatCurrency(overview.per_guest_spend)} label={t('stats.per_guest')} />
        <KpiCard icon={Clock} bg="bg-amber-100" color="text-amber-600" value={overview.average_dining_minutes > 0 ? `${Math.round(overview.average_dining_minutes)} min` : '-'} label={t('stats.avg_dining_time')} />
      </div>

      {/* KPI Row 3 — Losses & Discounts */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <KpiCard icon={XCircle} bg="bg-red-100" color="text-red-600" value={String(overview.voided_orders)} label={`${t('stats.void_orders')} (${formatCurrency(overview.voided_amount)})`} />
        <KpiCard icon={AlertTriangle} bg="bg-orange-100" color="text-orange-600" value={String(overview.loss_orders)} label={`${t('stats.loss_orders')} (${formatCurrency(overview.loss_amount)})`} />
        <KpiCard icon={Tag} bg="bg-yellow-100" color="text-yellow-600" value={formatCurrency(overview.total_discount)} label={t('stats.total_discount')} />
        <KpiCard icon={Receipt} bg="bg-slate-100" color="text-slate-600" value={formatCurrency(overview.total_tax)} label={t('stats.total_tax')} />
      </div>

      {/* Revenue Trend — recharts Area */}
      {trendData.length > 0 && (
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('stats.revenue_trend')}</h3>
          </div>
          <ResponsiveContainer width="100%" height={220}>
            <AreaChart data={trendData} margin={{ top: 4, right: 4, left: -10, bottom: 0 }}>
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
                formatter={(value: number | undefined, name: string | undefined) => [
                  name === 'revenue' ? formatCurrency(value ?? 0) : (value ?? 0),
                  name === 'revenue' ? t('stats.total_sales') : t('stats.orders_label'),
                ]}
                labelFormatter={l => l}
              />
              <Area type="monotone" dataKey="revenue" stroke="#ef4444" strokeWidth={2} fill="url(#colorRevenue)" />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Two columns: Payment Breakdown Pie + Tax Breakdown */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {overview.payment_breakdown.length > 0 && (
          <div className="bg-white rounded-2xl border border-slate-200 p-6">
            <div className="flex items-center gap-2 mb-4">
              <CreditCard className="w-5 h-5 text-slate-400" />
              <h3 className="font-bold text-slate-900">{t('stats.payment_breakdown')}</h3>
            </div>
            <div className="flex items-center gap-4">
              <div className="w-36 h-36 shrink-0">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={overview.payment_breakdown}
                      dataKey="amount"
                      nameKey="method"
                      cx="50%"
                      cy="50%"
                      innerRadius={30}
                      outerRadius={60}
                      paddingAngle={2}
                    >
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
                        <span className="text-slate-700 truncate">{pb.method}</span>
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
          </div>
        )}

        {overview.tax_breakdown.length > 0 && (
          <div className="bg-white rounded-2xl border border-slate-200 p-6">
            <div className="flex items-center gap-2 mb-4">
              <Receipt className="w-5 h-5 text-slate-400" />
              <h3 className="font-bold text-slate-900">{t('stats.tax_breakdown')}</h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-slate-100">
                    <th className="text-left py-2 text-xs font-medium text-slate-400">{t('stats.tax_rate')}</th>
                    <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.tax_base')}</th>
                    <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.tax_amount')}</th>
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
          </div>
        )}
      </div>

      {/* Tag Sales */}
      {overview.tag_sales.length > 0 && (
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
          <div className="flex items-center gap-2 mb-4">
            <Tag className="w-5 h-5 text-slate-400" />
            <h3 className="font-bold text-slate-900">{t('stats.tag_sales')}</h3>
          </div>
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
        </div>
      )}

      {/* Two columns: Top Products + Category Sales */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {overview.top_products.length > 0 && (
          <div className="bg-white rounded-2xl border border-slate-200 p-6">
            <div className="flex items-center gap-2 mb-4">
              <Award className="w-5 h-5 text-slate-400" />
              <h3 className="font-bold text-slate-900">{t('stats.top_products')}</h3>
            </div>
            <div className="space-y-2">
              {overview.top_products.map((product, i) => {
                const maxQty = overview.top_products[0]?.quantity ?? 1;
                const barPct = (product.quantity / maxQty) * 100;
                return (
                  <div key={i} className="relative">
                    <div className="absolute inset-y-0 left-0 rounded bg-primary-50" style={{ width: `${barPct}%` }} />
                    <div className="relative flex items-center justify-between py-2 px-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="text-xs font-bold text-primary-400 w-5 text-right">{i + 1}</span>
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
          </div>
        )}

        {overview.category_sales.length > 0 && (
          <div className="bg-white rounded-2xl border border-slate-200 p-6">
            <div className="flex items-center gap-2 mb-4">
              <BarChart3 className="w-5 h-5 text-slate-400" />
              <h3 className="font-bold text-slate-900">{t('stats.category_sales')}</h3>
            </div>
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
          </div>
        )}
      </div>
    </div>
  );
};

const KpiCard: React.FC<{
  icon: React.FC<{ className?: string }>;
  bg: string;
  color: string;
  value: string;
  label: string;
  accent?: boolean;
}> = ({ icon: Icon, bg, color, value, label, accent }) => (
  <div className={`bg-white rounded-xl border ${accent ? 'border-primary-200 ring-1 ring-primary-100' : 'border-slate-200'} p-4`}>
    <div className={`w-8 h-8 ${bg} rounded-lg flex items-center justify-center mb-2`}>
      <Icon className={`w-4 h-4 ${color}`} />
    </div>
    <p className={`text-lg font-bold ${accent ? 'text-primary-600' : 'text-slate-900'}`}>{value}</p>
    <p className="text-xs text-slate-400">{label}</p>
  </div>
);
