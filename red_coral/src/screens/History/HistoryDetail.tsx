import React, { useState, useEffect, useCallback } from 'react';
import { HeldOrder, Permission } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { TimelineList, NoteTag } from '@/presentation/components/shared/TimelineList';
import { calculateItemFinalPrice, calculateItemTotal, calculateOptionsModifier } from '@/utils/pricing';
import { formatCurrency } from '@/utils/currency';
import { Receipt, Calendar, Printer, CreditCard, Coins, Clock, ChevronDown, ChevronUp, ChevronsDown, ChevronsUp, Trash2 } from 'lucide-react';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';

import { groupOptionsByAttribute } from '@/utils/formatting';

interface HistoryDetailProps {
  order?: HeldOrder;
  onReprint: () => void;
}

export const HistoryDetail: React.FC<HistoryDetailProps> = ({ order, onReprint }) => {
  const { t } = useI18n();
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set());

  useEffect(() => {
    setExpandedItems(new Set());
  }, [order]);

  const toggleItem = useCallback((idx: number) => {
    setExpandedItems((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
      }
      return next;
    });
  }, []);

  const toggleAll = () => {
    if (!order) return;
    if (expandedItems.size === order.items.length) {
      setExpandedItems(new Set());
    } else {
      setExpandedItems(new Set(order.items.map((_, i) => i)));
    }
  };

  if (!order) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-gray-300">
        <Receipt size={64} className="mb-4 opacity-50" />
        <p>{t('history.info.selectOrder')}</p>
      </div>
    );
  }

  const isVoid = order.status === 'VOID';
  const isMerged = order.status === 'MERGED';
  
  const allocatedPaidMap = React.useMemo(() => {
    const map = new Map<string, number>();
    if (!order.paidItemQuantities) return map;

    // Group items by originalInstanceId (or instanceId if no original)
    const groups = new Map<string, typeof order.items>();
    order.items.forEach(item => {
        const key = item.originalInstanceId || item.instanceId;
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key)!.push(item);
    });

    // Distribute paid quantities
    groups.forEach((items, key) => {
        let availablePaid = order.paidItemQuantities![key] || 0;
        
        // Sort items: Active first, then Removed
        // This ensures that if we have 3 items (1 paid), and delete 2,
        // the 1 active item gets the paid status.
        const sortedItems = [...items].sort((a, b) => {
            if (a._removed === b._removed) return 0;
            return a._removed ? 1 : -1;
        });

        sortedItems.forEach(item => {
            const allocated = Math.min(item.quantity, availablePaid);
            map.set(item.instanceId, allocated);
            availablePaid -= allocated;
        });
    });

    return map;
  }, [order.items, order.paidItemQuantities]);

  const { activeItems, removedItems } = React.useMemo(() => {
    const active: typeof order.items = [];
    const removed: typeof order.items = [];
    order.items.forEach(item => {
      if (item._removed) {
        removed.push(item);
      } else {
        active.push(item);
      }
    });
    return { activeItems: active, removedItems: removed };
  }, [order.items]);

  const paymentEvents = React.useMemo(
    () =>
      order.timeline.filter(
        (e) =>
          e.type === 'PAYMENT' ||
          e.type === 'PAYMENT_ADDED' ||
          e.type === 'ORDER_SPLIT'
      ),
    [order.timeline]
  );

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="bg-white rounded-2xl p-6 shadow-sm border border-gray-200 flex justify-between items-start">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <h1 className={`text-2xl font-bold ${isVoid || isMerged ? 'text-gray-500 line-through' : 'text-gray-900'}`}>
              {order.receiptNumber || order.tableName }
            </h1>
            {isVoid && (
              <span className="px-2 py-1 bg-red-100 text-red-700 text-xs font-bold rounded uppercase">
                {t('history.status.voided')}
              </span>
            )}
            {isMerged && (
              <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-bold rounded uppercase">
                {t('history.status.merged')}
              </span>
            )}
            <ProtectedGate permission={Permission.REPRINT_RECEIPT}>
              <button
                onClick={onReprint}
                className="flex items-center gap-1.5 px-3 py-1 bg-white border border-gray-300 rounded-lg shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-900 transition-colors"
              >
                <Printer size={16} />
                <span>{t('history.action.reprint')}</span>
              </button>
            </ProtectedGate>
          </div>
          <div className="flex gap-4 text-sm text-gray-500">
            {order.tableName !== 'RETAIL' && (
              <div className="flex items-center gap-1.5 font-medium text-gray-700">
                <span>{t('history.info.table')}: {order.tableName}</span>
              </div>
            )}
            <div className="flex items-center gap-1.5">
              <Calendar size={16} />
              <span>{new Date(order.startTime).toLocaleDateString()}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Clock size={16} />
              <span>{new Date(order.startTime).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })} - {order.endTime ? new Date(order.endTime).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false }) : t('common.na')}</span>
            </div>
          </div>
        </div>
          <div className="text-right">
          <div className="text-sm text-gray-500 uppercase font-bold tracking-wider mb-1">{t('history.info.totalAmount')}</div>
          <div className={`text-3xl font-bold ${isVoid || isMerged ? 'text-gray-400 line-through' : 'text-[#FF5E5E]'}`}>{formatCurrency(order.total)}</div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center justify-between font-bold text-gray-700">
              <div className="flex items-center gap-2">
                <Receipt size={18} />
                <span>{t('history.info.orderItems')}</span>
              </div>
              <button
                onClick={toggleAll}
                title={expandedItems.size === order.items.length ? t('common.collapseAll') : t('common.expandAll')}
                className="p-1.5 text-gray-500 hover:text-gray-700 transition-colors rounded hover:bg-gray-200"
              >
                {expandedItems.size === order.items.length ? <ChevronsUp size={18} /> : <ChevronsDown size={18} />}
              </button>
            </div>
            <div className="divide-y divide-gray-100">
              {activeItems.map((item) => {
                const idx = order.items.indexOf(item);
                return (
                  <OrderItemRow
                    key={item.instanceId || `${item.id}-${idx}`}
                    item={item}
                    index={idx}
                    isExpanded={expandedItems.has(idx)}
                    onToggle={toggleItem}
                    order={order}
                    t={t}
                    allocatedPaidQty={allocatedPaidMap.get(item.instanceId)}
                  />
                );
              })}
              {removedItems.length > 0 && (
                <>
                  <div className="p-3 bg-red-50 text-red-800 text-xs font-bold uppercase tracking-wider border-y border-red-100 flex items-center gap-2">
                    <Trash2 size={14} />
                    {t('history.info.removedItems')}
                  </div>
                  {removedItems.map((item) => {
                    const idx = order.items.indexOf(item);
                    return (
                      <OrderItemRow
                        key={item.instanceId || `${item.id}-${idx}`}
                        item={item}
                        index={idx}
                        isExpanded={expandedItems.has(idx)}
                        onToggle={toggleItem}
                        order={order}
                        t={t}
                        allocatedPaidQty={allocatedPaidMap.get(item.instanceId)}
                      />
                    );
                  })}
                </>
              )}
            </div>
            <div className="p-5 bg-gray-50 border-t border-gray-200 space-y-2">
              {!order.surchargeExempt && order.surcharge && order.surcharge.total > 0 && (
                <div className="flex justify-between items-end">
                  <span className="text-purple-500 font-medium text-sm">{order.surcharge.name || 'Surcharge'}</span>
                  <span className="text-sm font-medium text-purple-500">+{formatCurrency(order.surcharge.total)}</span>
                </div>
              )}
              <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
                <span className="text-gray-800 font-bold">{t('checkout.amount.total')}</span>
                <span className="text-xl font-bold text-[#FF5E5E]">{formatCurrency(order.total)}</span>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
              <CreditCard size={18} />
              <span>{t('history.payment.details')}</span>
            </div>
            <div className="divide-y divide-gray-100">
              {paymentEvents.length === 0 ? (
                <div className="p-4 text-center text-gray-400 text-sm">{t('history.payment.noPayments')}</div>
              ) : (
                paymentEvents.map((event, idx) => (
                  <PaymentEventRow key={event.id || `${event.type}-${event.timestamp}-${idx}`} event={event} t={t} />
                ))
              )}
            </div>
          </div>
        </div>
        
        <div className="lg:col-span-1 bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden flex flex-col h-fit">
          <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
            <Clock size={18} />
            <span>{t('checkout.timeline.label')}</span>
          </div>
          <div className="p-6">
            <TimelineList events={order.timeline} />
          </div>
        </div>
      </div>
    </div>
  );
};

interface OrderItemRowProps {
  item: HeldOrder['items'][number];
  index: number;
  isExpanded: boolean;
  onToggle: (index: number) => void;
  order: HeldOrder;
  t: (key: string, params?: Record<string, string | number>) => string;
  allocatedPaidQty?: number;
}

const OrderItemRow: React.FC<OrderItemRowProps> = React.memo(
  ({ item, index, isExpanded, onToggle, order, t, allocatedPaidQty }) => {
    const discountPercent = item.discountPercent || 0;
    const optionsModifier = calculateOptionsModifier(item.selectedOptions).toNumber();
    const baseUnitPrice = (item.originalPrice ?? item.price) + optionsModifier;
    const finalUnitPrice = calculateItemFinalPrice(item).toNumber();
    const lineTotal = calculateItemTotal(item).toNumber();
    const hasDiscount = discountPercent > 0 || baseUnitPrice !== finalUnitPrice;
    const orderLevelSurchargeActive = !order.surchargeExempt && !!order.surcharge && order.surcharge.amount > 0;
    const itemSurcharge = order.surchargeExempt ? 0 : (orderLevelSurchargeActive ? 0 : (item.surcharge || 0));
    const hasAttributes = item.selectedOptions && item.selectedOptions.length > 0;
    const paidQty = allocatedPaidQty !== undefined ? allocatedPaidQty : (order.paidItemQuantities?.[item.instanceId] || 0);
    const isFullyPaid = paidQty >= item.quantity;
    const isRemoved = item._removed;

    return (
      <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''} ${isRemoved ? 'opacity-60 grayscale' : ''}`}>
        <div
          className={`p-4 flex justify-between items-center cursor-pointer hover:bg-gray-50 transition-colors select-none`}
          onClick={() => onToggle(index)}
        >
          <div className="flex items-center gap-4 flex-1">
            <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0 relative
              ${isFullyPaid ? 'bg-green-100 text-green-600' : 'bg-gray-100 text-gray-500'}
            `}>
              {isRemoved ? <span className="line-through">x{item.quantity}</span> : `x${item.quantity}`}
              {paidQty > 0 && !isFullyPaid && (
                <div className="absolute -top-2 -right-2 bg-green-500 text-white text-[10px] px-1 rounded-full shadow-sm">
                  {paidQty}
                </div>
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
                {/* User requested to use InstanceID instead of ExternalID
                {item.externalId && (
                  <span className="text-[10px] text-white bg-gray-900/85 font-bold font-mono px-1.5 py-0.5 rounded backdrop-blur-[1px]">
                    {item.externalId}
                  </span>
                )} */}
                <span>{item.name}</span>
                <span className="text-[10px] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded">
                  #{(item.originalInstanceId || item.instanceId).slice(-5)}
                </span>
                {item.discountPercent ? (
                  <span className="text-[10px] font-bold bg-red-100 text-red-600 px-1.5 py-0.5 rounded-full">-{item.discountPercent}%</span>
                ) : null}
                {!order.surchargeExempt && itemSurcharge ? (
                  <span className="text-[10px] font-bold bg-purple-100 text-purple-600 px-1.5 py-0.5 rounded-full">+{formatCurrency(itemSurcharge)}</span>
                ) : null}
              </div>
              <div className="text-xs text-gray-400 flex items-center gap-2">
                {hasDiscount ? (
                  <>
                    <span className="line-through">{formatCurrency(baseUnitPrice)}</span>
                    <span>{formatCurrency(finalUnitPrice)}</span>
                  </>
                ) : (
                  <span>{formatCurrency(finalUnitPrice)}</span>
                )}
                <span>/ {t('checkout.amount.unitPrice')}</span>
                {hasAttributes && (
                  <span className="flex items-center gap-1 ml-2 text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded-md">
                    {isExpanded ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                    {t('common.details')}
                  </span>
                )}
              </div>
            </div>
          </div>
          <div className="font-bold text-gray-800 pl-4">{formatCurrency(lineTotal)}</div>
        </div>

        {isExpanded && hasAttributes && (
          <div className="px-16 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
            <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-1 shadow-sm">
              {item.selectedOptions && groupOptionsByAttribute(item.selectedOptions).map((group, idx) => (
                <div key={idx} className="flex justify-between items-center text-sm">
                  <div className="flex items-center gap-2">
                    <span className="text-gray-500 font-medium">{group.attributeName}:</span>
                    <span className="text-gray-800">{group.optionNames.join(', ')}</span>
                  </div>
                  {group.totalPrice !== 0 && (
                    <span className={`text-xs font-bold ${group.totalPrice > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                      {group.totalPrice > 0 ? '+' : ''}{formatCurrency(group.totalPrice)}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  }
);

OrderItemRow.displayName = 'OrderItemRow';

export interface PaymentEventRowProps {
  event: HeldOrder['timeline'][number];
  t: (key: string, params?: Record<string, string | number>) => string;
}

export const PaymentEventRow: React.FC<PaymentEventRowProps> = React.memo(({ event, t }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const data: any = (event as any).data || {};

  let methodRaw = '';
  let amountNum = 0;
  let note: string | undefined = undefined;

  if (event.type === 'ORDER_SPLIT') {
    methodRaw = data.paymentMethod || '';
    amountNum = data.splitAmount || 0;
    note = t('timeline.event.splitBill');
  } else {
    const payment = data.payment || data;
    methodRaw = payment?.method || '';
    amountNum = typeof payment?.amount === 'number' ? payment.amount : (() => {
      const m = (event.summary || '').match(/€([0-9.]+)/);
      return m ? parseFloat(m[1]) : 0;
    })();
    note = payment?.note || event.data?.note || undefined;
  }

  const isCash = /cash/i.test(methodRaw);
  const isCard = /card|visa|master/i.test(methodRaw);
  const hasItems = event.type === 'ORDER_SPLIT' && data.items && Array.isArray(data.items) && data.items.length > 0;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div 
        className={`p-4 flex justify-between items-center ${hasItems ? 'cursor-pointer hover:bg-gray-50' : ''}`}
        onClick={() => hasItems && setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-full ${isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600'}`}>
            {isCash ? <Coins size={16} /> : <CreditCard size={16} />}
          </div>
          <div>
            <div className="font-medium text-gray-800 flex items-center gap-2">
              {isCash ? t('checkout.method.cash') : isCard ? t('checkout.method.card') : methodRaw || 'VISA'}
              {hasItems && (
                <span className="text-gray-400">
                  {isExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400">
              {new Date(event.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <div className="font-bold text-gray-800">{formatCurrency(amountNum)}</div>
          {note && (
            <div className="mt-1">
              <NoteTag text={note} />
            </div>
          )}
        </div>
      </div>
      
      {isExpanded && hasItems && (
        <div className="px-14 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
          <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-2 shadow-sm">
            {data.items.map((item: any, idx: number) => (
              <div key={idx} className="flex justify-between items-start text-sm">
                <div className="flex flex-col">
                  <span className="text-gray-800 font-medium">
                    {item.name} <span className="text-gray-500">x{item.quantity}</span>
                    {item.instanceId && (
                      <span className="ml-1.5 text-[10px] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded">
                        #{item.instanceId.slice(-5)}
                      </span>
                    )}
                  </span>
                  {item.selectedOptions && item.selectedOptions.length > 0 && (
                    <span className="text-xs text-gray-500 pl-2 border-l-2 border-gray-100 mt-1">
                      {item.selectedOptions.map((o: any) => o.optionName).join(', ')}
                    </span>
                  )}
                </div>
                <div className="font-medium text-gray-800">
                  {formatCurrency((item.price || 0) * item.quantity)}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

PaymentEventRow.displayName = 'PaymentEventRow';
