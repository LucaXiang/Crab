/**
 * OrderDetailMode - 订单详情页面
 *
 * 在 Checkout 页面中展示类似 HistoryOrderDetail 的信息（不含时间线）。
 * 使用活跃订单 HeldOrder (OrderSnapshot) 数据，而非归档数据。
 */

import React, { useState, useCallback, useMemo } from 'react';
import { HeldOrder, CartItemSnapshot, PaymentRecord } from '@/core/domain/types';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { useI18n } from '@/hooks/useI18n';
import { useProductStore, useCategoryStore } from '@/core/stores/resources';
import { formatCurrency, Currency } from '@/utils/currency';
import { CATEGORY_ACCENT, buildCategoryColorMap } from '@/utils/categoryColors';
import {
  ArrowLeft, Receipt, CreditCard, Coins,
  ChevronDown, ChevronUp, ChevronsDown, ChevronsUp, Ban,
  Gift, Stamp,
} from 'lucide-react';
import { calculateItemSink } from '@/utils/itemSorting';

interface OrderDetailModeProps {
  order: HeldOrder;
  totalPaid: number;
  remaining: number;
  onBack: () => void;
  onManageTable?: () => void;
}

// =============================================================================
// Split type config (same as HistoryDetail)
// =============================================================================

const SPLIT_TYPE_CONFIG: Record<string, { label: string; bg: string; text: string }> = {
  ITEM_SPLIT: { label: 'history.payment.split_type.item', bg: 'bg-indigo-100', text: 'text-indigo-600' },
  AMOUNT_SPLIT: { label: 'history.payment.split_type.amount', bg: 'bg-cyan-100', text: 'text-cyan-600' },
  AA_SPLIT: { label: 'history.payment.split_type.aa', bg: 'bg-cyan-100', text: 'text-cyan-600' },
};

// =============================================================================
// Main Component
// =============================================================================

export const OrderDetailMode: React.FC<OrderDetailModeProps> = ({
  order,
  totalPaid,
  remaining,
  onBack,
  onManageTable,
}) => {
  const { t } = useI18n();
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set());
  const products = useProductStore((s) => s.items);
  const categories = useCategoryStore((s) => s.items);

  const visibleItems = useMemo(() => {
    const items = order.items.filter(i => !i._removed);
    const categoryMap = new Map(categories.map(c => [c.id, c]));
    const productMap = new Map(products.map(p => [p.id, p]));

    return [...items].sort((a, b) => {
      // Sort by category sort_order
      const catA = productMap.get(a.id)?.category_id;
      const catB = productMap.get(b.id)?.category_id;
      const sortA = catA ? (categoryMap.get(catA)?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      const sortB = catB ? (categoryMap.get(catB)?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      if (sortA !== sortB) return sortA - sortB;

      // Fully-paid & comped items sink to end of each category (paid → comped)
      const sinkA = calculateItemSink(a);
      const sinkB = calculateItemSink(b);
      if (sinkA !== sinkB) return sinkA - sinkB;

      // Then by external_id
      const extA = productMap.get(a.id)?.external_id ?? Number.MAX_SAFE_INTEGER;
      const extB = productMap.get(b.id)?.external_id ?? Number.MAX_SAFE_INTEGER;
      if (extA !== extB) return extA - extB;

      return a.name.localeCompare(b.name);
    });
  }, [order.items, products, categories]);

  const colorMap = useMemo(() => buildCategoryColorMap(categories), [categories]);
  const productMap = useMemo(() => new Map(products.map(p => [p.id, p])), [products]);

  const toggleItem = useCallback((idx: number) => {
    setExpandedItems(prev => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx);
      else next.add(idx);
      return next;
    });
  }, []);

  const toggleAll = () => {
    if (expandedItems.size === visibleItems.length) {
      setExpandedItems(new Set());
    } else {
      setExpandedItems(new Set(visibleItems.map((_, i) => i)));
    }
  };

  // Price breakdown (same logic as OrderSidebar)
  const displayItemDiscount = Currency.sub(order.total_discount, order.order_manual_discount_amount).toNumber();

  // Split: rule vs manual
  const itemRuleDiscount = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_discount_amount, item.quantity)).toNumber(), 0);
  const itemRuleSurcharge = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_surcharge_amount, item.quantity)).toNumber(), 0);
  const totalRuleDiscount = Currency.add(itemRuleDiscount, order.order_rule_discount_amount).toNumber();
  const totalRuleSurcharge = Currency.add(itemRuleSurcharge, order.order_rule_surcharge_amount).toNumber();
  const manualItemDiscount = Currency.sub(displayItemDiscount, totalRuleDiscount).toNumber();

  return (
    <div className="h-full flex bg-gray-50/50 backdrop-blur-xl">
      <OrderSidebar
        order={order}
        totalPaid={totalPaid}
        remaining={remaining}
        onManage={onManageTable}
      />
      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        {/* Background Decor */}
        <div className="absolute top-[-20%] left-[-10%] w-[600px] h-[600px] bg-orange-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />
        <div className="absolute bottom-[-20%] right-[-10%] w-[600px] h-[600px] bg-amber-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />

        {/* Header */}
        <div className="p-6 bg-white/80 backdrop-blur-md border-b border-gray-200/50 shadow-sm flex justify-between items-center z-10 shrink-0">
          <h3 className="font-bold text-gray-800 text-2xl flex items-center gap-3">
            <div className="p-2 bg-amber-500 rounded-xl text-white shadow-lg shadow-amber-500/30">
              <Receipt size={24} />
            </div>
            {t('checkout.order_detail.title')}
          </h3>
          <button
            onClick={onBack}
            className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm"
          >
            <ArrowLeft size={20} /> {t('common.action.back')}
          </button>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6 z-10">
          {/* Order Items Card */}
          <div className="bg-white/80 backdrop-blur-sm rounded-2xl shadow-sm border border-gray-200/60 overflow-hidden">
            <div className="p-4 border-b border-gray-100/80 bg-gray-50/80 flex items-center justify-between font-bold text-gray-700">
              <div className="flex items-center gap-2">
                <Receipt size={18} />
                <span>{t('history.info.order_items')}</span>
              </div>
              <button
                onClick={toggleAll}
                title={expandedItems.size === visibleItems.length ? t('common.action.collapse_all') : t('common.action.expand_all')}
                className="p-1.5 text-gray-500 hover:text-gray-700 transition-colors rounded hover:bg-gray-200"
              >
                {expandedItems.size === visibleItems.length ? <ChevronsUp size={18} /> : <ChevronsDown size={18} />}
              </button>
            </div>
            <div className="divide-y divide-gray-100">
              {visibleItems.map((item, idx) => {
                const catId = String(productMap.get(item.id)?.category_id ?? 'uncategorized');
                const colorIdx = colorMap.get(catId) ?? 0;
                return (
                  <OrderItemRow
                    key={item.instance_id || idx}
                    item={item}
                    index={idx}
                    isExpanded={expandedItems.has(idx)}
                    onToggle={toggleItem}
                    accentColor={CATEGORY_ACCENT[colorIdx]}
                    t={t}
                  />
                );
              })}
            </div>
            {/* Price Breakdown Footer */}
            <div className="p-5 bg-gray-50/80 border-t border-gray-200/60 space-y-2">
              {order.comp_total_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-emerald-600">{t('checkout.breakdown.comp')}</span>
                  <span className="text-emerald-600">-{formatCurrency(order.comp_total_amount)}</span>
                </div>
              )}
              {manualItemDiscount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-orange-500">{t('checkout.breakdown.manual_discount')}</span>
                  <span className="text-orange-500">-{formatCurrency(manualItemDiscount)}</span>
                </div>
              )}
              {totalRuleDiscount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-amber-600">{t('checkout.breakdown.rule_discount')}</span>
                  <span className="text-amber-600">-{formatCurrency(totalRuleDiscount)}</span>
                </div>
              )}
              {totalRuleSurcharge > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-purple-500">{t('checkout.breakdown.rule_surcharge')}</span>
                  <span className="text-purple-500">+{formatCurrency(totalRuleSurcharge)}</span>
                </div>
              )}
              {order.order_manual_discount_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-orange-500">{t('checkout.breakdown.order_discount')}</span>
                  <span className="text-orange-500">-{formatCurrency(order.order_manual_discount_amount)}</span>
                </div>
              )}
              {order.order_manual_surcharge_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-purple-500">{t('checkout.breakdown.order_surcharge')}</span>
                  <span className="text-purple-500">+{formatCurrency(order.order_manual_surcharge_amount)}</span>
                </div>
              )}
              <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
                <span className="text-gray-800 font-bold">{t('checkout.amount.total')}</span>
                <span className="text-xl font-bold text-primary-500">{formatCurrency(order.total)}</span>
              </div>
            </div>
          </div>

          {/* Payment Records Card */}
          <div className="bg-white/80 backdrop-blur-sm rounded-2xl shadow-sm border border-gray-200/60 overflow-hidden">
            <div className="p-4 border-b border-gray-100/80 bg-gray-50/80 flex items-center gap-2 font-bold text-gray-700">
              <CreditCard size={18} />
              <span>{t('history.payment.details')}</span>
            </div>
            <div className="divide-y divide-gray-100">
              {order.payments.length === 0 ? (
                <div className="p-4 text-center text-gray-400 text-sm">{t('history.payment.no_payments')}</div>
              ) : (
                order.payments.map((payment, idx) => (
                  <PaymentRow key={payment.payment_id || idx} payment={payment} aaTotalShares={order.aa_total_shares} t={t} />
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// =============================================================================
// Order Item Row
// =============================================================================

interface OrderItemRowProps {
  item: CartItemSnapshot;
  index: number;
  isExpanded: boolean;
  onToggle: (index: number) => void;
  accentColor?: string;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const OrderItemRow: React.FC<OrderItemRowProps> = React.memo(({ item, index, isExpanded, onToggle, accentColor, t }) => {
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const totalRuleDiscount = item.rule_discount_amount;
  const totalRuleSurcharge = item.rule_surcharge_amount;
  const discountPercent = item.manual_discount_percent || 0;
  const isFullyPaid = item.unpaid_quantity === 0;
  const isPartiallyPaid = !isFullyPaid && item.unpaid_quantity < item.quantity;

  return (
    <div>
      <div
        className={`p-4 flex justify-between items-center cursor-pointer transition-colors select-none ${
          item.is_comped ? 'bg-emerald-50/60' : 'hover:bg-gray-50/50'
        }`}
        onClick={() => onToggle(index)}
      >
        <div className="flex items-center gap-3 flex-1">
          <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: accentColor || '#d1d5db' }} />
          <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0
            ${item.is_comped ? 'bg-emerald-100 text-emerald-600' : isFullyPaid ? 'bg-green-100 text-green-600' : isPartiallyPaid ? 'bg-amber-100 text-amber-600' : 'bg-gray-100 text-gray-500'}
          `}>
            x{item.quantity}
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
              <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200">
                #{item.instance_id.slice(-5)}
              </span>
              <span>{item.name}</span>
              {item.selected_specification?.name && (
                <span className="text-xs text-gray-500">({item.selected_specification.name})</span>
              )}
              {item.is_comped && (
                item.instance_id.startsWith('stamp_reward::') ? (
                  <span className="text-xs bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded flex items-center gap-1">
                    <Stamp size={10} />
                    {t('checkout.stamp_reward')}
                  </span>
                ) : (
                  <span className="text-xs bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-1">
                    <Gift size={10} />
                    {t('checkout.comp.badge')}
                  </span>
                )
              )}
              {discountPercent > 0 && (
                <span className="text-[0.625rem] font-bold bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded-full">
                  -{discountPercent}%
                </span>
              )}
              {totalRuleDiscount > 0 && (
                <span className="text-[0.625rem] font-bold bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded-full">
                  -{formatCurrency(totalRuleDiscount)}
                </span>
              )}
              {totalRuleSurcharge > 0 && (
                <span className="text-[0.625rem] font-bold bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded-full">
                  +{formatCurrency(totalRuleSurcharge)}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-2">
              <span>{formatCurrency(item.unit_price)}</span>
              <span>/ {t('checkout.amount.unit_price')}</span>
              {hasOptions && (
                <span className="flex items-center gap-1 ml-2 text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded-md">
                  {isExpanded ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                  {t('common.label.details')}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="font-bold text-gray-800 pl-4">{formatCurrency(item.line_total)}</div>
      </div>

      {isExpanded && hasOptions && (() => {
        const grouped = new Map<string, typeof item.selected_options>();
        for (const opt of item.selected_options!) {
          const key = opt.attribute_name;
          if (!grouped.has(key)) grouped.set(key, []);
          grouped.get(key)!.push(opt);
        }
        return (
          <div className="px-16 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
            <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-1 shadow-sm">
              {[...grouped.entries()].map(([attrName, opts]) => (
                <div key={attrName} className="text-sm">
                  <span className="text-gray-500 font-medium">{attrName}: </span>
                  <span className="text-gray-800">
                    {opts!.map((opt, i) => (
                      <React.Fragment key={i}>
                        {i > 0 && ', '}
                        {opt.option_name}
                        {opt.price_modifier != null && opt.price_modifier !== 0 && (
                          <span className={`text-xs font-bold ml-0.5 ${opt.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                            {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                          </span>
                        )}
                      </React.Fragment>
                    ))}
                  </span>
                </div>
              ))}
            </div>
          </div>
        );
      })()}
    </div>
  );
});

OrderItemRow.displayName = 'OrderItemRow';

// =============================================================================
// Payment Row
// =============================================================================

interface PaymentRowProps {
  payment: PaymentRecord;
  aaTotalShares?: number | null;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const PaymentRow: React.FC<PaymentRowProps> = React.memo(({ payment, aaTotalShares, t }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const isCash = /cash/i.test(payment.method);
  const hasItems = payment.split_items && payment.split_items.length > 0;
  const effectiveSplitType = payment.split_type ?? (hasItems ? 'ITEM_SPLIT' : null);
  const splitConfig = effectiveSplitType ? SPLIT_TYPE_CONFIG[effectiveSplitType] ?? null : null;

  const iconBg = isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600';
  const IconComponent = isCash ? Coins : CreditCard;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div
        className={`p-4 flex justify-between items-center ${hasItems ? 'cursor-pointer hover:bg-gray-50' : ''}`}
        onClick={() => hasItems && setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-full ${iconBg}`}>
            <IconComponent size={16} />
          </div>
          <div>
            <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
              {isCash ? t('checkout.method.cash') : payment.method}
              {splitConfig && (
                <span className={`text-[0.625rem] font-bold px-1.5 py-0.5 rounded ${splitConfig.bg} ${splitConfig.text}`}>
                  {t(splitConfig.label)}
                </span>
              )}
              {payment.payment_id && (
                <span className="text-[0.625rem] text-emerald-600 bg-emerald-100 font-bold font-mono px-1.5 py-0.5 rounded">
                  #{payment.payment_id.slice(-5)}
                </span>
              )}
              {payment.cancelled && (
                <span className="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded font-bold flex items-center gap-1">
                  <Ban size={10} /> {t('common.status.cancelled')}
                </span>
              )}
              {hasItems && (
                <span className="text-gray-400">
                  {isExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-2">
              <span>{new Date(payment.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
              {effectiveSplitType === 'AA_SPLIT' && payment.aa_shares && aaTotalShares && (
                <span className="text-cyan-600 font-medium">
                  {payment.aa_shares}/{aaTotalShares} {t('history.payment.aa_shares_unit')}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <div className={`font-bold ${payment.cancelled ? 'text-gray-400 line-through' : 'text-gray-800'}`}>
            {formatCurrency(payment.amount)}
          </div>
          {payment.cancel_reason && (
            <div className="text-xs text-red-500 mt-1">{payment.cancel_reason}</div>
          )}
        </div>
      </div>

      {isExpanded && hasItems && (
        <div className="px-14 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
          <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-2 shadow-sm">
            {payment.split_items!.map((item, idx) => (
              <div key={idx} className="flex items-center gap-3 text-sm">
                <div className="w-7 h-7 rounded flex items-center justify-center font-bold text-xs shrink-0 bg-green-100 text-green-600">
                  x{item.quantity}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
                    <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200">
                      #{item.instance_id.slice(-5)}
                    </span>
                    <span>{item.name}</span>
                  </div>
                  <div className="text-xs text-gray-400">
                    {formatCurrency(item.unit_price)} / {t('checkout.amount.unit_price')}
                  </div>
                </div>
                <div className="font-bold text-gray-800 pl-4 shrink-0">
                  {formatCurrency(Currency.mul(item.unit_price, item.quantity).toNumber())}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

PaymentRow.displayName = 'PaymentRow';
