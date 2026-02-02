/**
 * OrderDetailMode - 订单详情页面
 *
 * 在 Checkout 页面中展示类似 HistoryOrderDetail 的信息（不含时间线）。
 * 使用活跃订单 HeldOrder (OrderSnapshot) 数据，而非归档数据。
 */

import React, { useState, useCallback } from 'react';
import { HeldOrder, CartItemSnapshot, PaymentRecord } from '@/core/domain/types';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency, Currency } from '@/utils/currency';
import {
  ArrowLeft, Receipt, CreditCard, Coins,
  ChevronDown, ChevronUp, ChevronsDown, ChevronsUp, Ban,
  Gift,
} from 'lucide-react';

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
  ITEM_SPLIT: { label: 'history.payment.split_type.item', bg: 'bg-violet-100', text: 'text-violet-600' },
  AMOUNT_SPLIT: { label: 'history.payment.split_type.amount', bg: 'bg-amber-100', text: 'text-amber-600' },
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

  const visibleItems = order.items.filter(i => !i._removed);

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
  const displayItemDiscount = order.total_discount - order.order_manual_discount_amount;

  // Split: rule vs manual
  const itemRuleDiscount = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_discount_amount ?? 0, item.quantity)).toNumber(), 0);
  const itemRuleSurcharge = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_surcharge_amount ?? 0, item.quantity)).toNumber(), 0);
  const totalRuleDiscount = Currency.add(itemRuleDiscount, order.order_rule_discount_amount ?? 0).toNumber();
  const totalRuleSurcharge = Currency.add(itemRuleSurcharge, order.order_rule_surcharge_amount ?? 0).toNumber();
  const manualItemDiscount = Currency.sub(displayItemDiscount, totalRuleDiscount).toNumber();

  return (
    <div className="h-full flex">
      <OrderSidebar
        order={order}
        totalPaid={totalPaid}
        remaining={remaining}
        onManage={onManageTable}
      />
      <div className="flex-1 flex flex-col bg-gray-50">
        {/* Header */}
        <div className="p-6 bg-white border-b border-gray-200 shadow-sm flex justify-between items-center">
          <h3 className="font-bold text-gray-800 text-xl flex items-center gap-2">
            <Receipt size={24} className="text-gray-600" />
            {t('checkout.order_detail.title')}
          </h3>
          <button
            onClick={onBack}
            className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium flex items-center gap-2 transition-all"
          >
            <ArrowLeft size={20} /> {t('common.action.back')}
          </button>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Order Items Card */}
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center justify-between font-bold text-gray-700">
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
              {visibleItems.map((item, idx) => (
                <OrderItemRow
                  key={item.instance_id || idx}
                  item={item}
                  index={idx}
                  isExpanded={expandedItems.has(idx)}
                  onToggle={toggleItem}
                  t={t}
                />
              ))}
            </div>
            {/* Price Breakdown Footer */}
            <div className="p-5 bg-gray-50 border-t border-gray-200 space-y-2">
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
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
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
  t: (key: string, params?: Record<string, string | number>) => string;
}

const OrderItemRow: React.FC<OrderItemRowProps> = React.memo(({ item, index, isExpanded, onToggle, t }) => {
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const discountAmount = (item.rule_discount_amount ?? 0);
  const surchargeAmount = (item.rule_surcharge_amount ?? 0);
  const hasDiscount = discountAmount > 0 || (item.manual_discount_percent != null && item.manual_discount_percent > 0);
  const hasSurcharge = surchargeAmount > 0;
  const isFullyPaid = (item.unpaid_quantity ?? item.quantity) === 0;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div
        className="p-4 flex justify-between items-center cursor-pointer hover:bg-gray-50 transition-colors select-none"
        onClick={() => onToggle(index)}
      >
        <div className="flex items-center gap-4 flex-1">
          <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0
            ${isFullyPaid ? 'bg-green-100 text-green-600' : 'bg-gray-100 text-gray-500'}
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
                <span className="text-xs bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-1">
                  <Gift size={10} />
                  {t('checkout.comp.badge')}
                </span>
              )}
              {hasDiscount && (
                <span className="text-[0.625rem] font-bold bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded-full">
                  {item.manual_discount_percent ? `-${item.manual_discount_percent}%` : `-${formatCurrency(discountAmount)}`}
                </span>
              )}
              {hasSurcharge && (
                <span className="text-[0.625rem] font-bold bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded-full">
                  +{formatCurrency(surchargeAmount)}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-2">
              <span>{formatCurrency(item.unit_price ?? item.price)}</span>
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
        <div className="font-bold text-gray-800 pl-4">{formatCurrency(item.line_total ?? Currency.mul(item.unit_price ?? item.price, item.quantity).toNumber())}</div>
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
                    {formatCurrency(item.unit_price ?? item.price)} / {t('checkout.amount.unit_price')}
                  </div>
                </div>
                <div className="font-bold text-gray-800 pl-4 shrink-0">
                  {formatCurrency(Currency.mul(item.unit_price ?? item.price, item.quantity).toNumber())}
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
