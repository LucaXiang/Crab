import React, { useState, lazy, Suspense } from 'react';
import { HeldOrder, CartItem } from '@/core/domain/types';
import { OrderEventType, ItemModifiedEvent, ItemRemovedEvent } from '@/core/domain/events';
import { Clock, List, Settings, ShoppingBag } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { OrderItemsSummary } from '@/screens/Checkout/OrderItemsSummary';
import { CartItemDetailModal } from '@/presentation/components/modals/CartItemDetailModal';
import { QuickAddModal } from '@/presentation/components/modals/QuickAddModal';
import { recalculateOrderTotal, convertEventToTimelineEvent, mergeItemsIntoList } from '@/core/services/order/eventReducer';
import { v4 as uuidv4 } from 'uuid';
import * as orderOps from '@/core/stores/order/useOrderOperations';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { Currency } from '@/utils/currency';
import { formatCurrency } from '@/utils/currency';
import { calculateDiscountAmount, calculateOptionsModifier } from '@/utils/pricing';

// Lazy load TimelineList - only loads when user clicks Timeline tab
const TimelineList = lazy(() =>
  import('@/presentation/components/shared/TimelineList').then(module => ({ default: module.TimelineList }))
);

interface OrderSidebarProps {
  order: HeldOrder;
  totalPaid: number;
  remaining: number;
  onUpdateOrder?: (order: HeldOrder) => void;
  onManage?: () => void;
}

type Tab = 'ITEMS' | 'TIMELINE';

export const OrderSidebar = React.memo<OrderSidebarProps>(({ order, totalPaid, remaining, onUpdateOrder, onManage }) => {
  const { t } = useI18n();
  const { user: currentUser } = useAuthStore();
  const [activeTab, setActiveTab] = useState<Tab>('ITEMS');
  const [editingItem, setEditingItem] = useState<{ item: CartItem; index: number } | null>(null);
  const [showQuickAdd, setShowQuickAdd] = useState(false);
  const [lazyTimeline, setLazyTimeline] = useState<any[] | null>(null);

  const handleEditItem = React.useCallback((item: CartItem) => {
    // Find index of item in order.items
    const index = order.items.findIndex((i) => 
      i.instanceId ? i.instanceId === item.instanceId : i.id === item.id
    );
    if (index !== -1) {
      setEditingItem({ item, index });
    }
  }, [order]);

  const handleSaveItem = React.useCallback(async (index: number, updates: Partial<CartItem>, _options?: { userId?: string }) => {
    const item = order.items[index];
    const instanceId = item.instanceId || `item-${index}`;

    // Send command to backend - state will be updated via event
    await orderOps.modifyItem(order.key, instanceId, {
      price: updates.price,
      quantity: updates.quantity,
      discountPercent: updates.discountPercent,
      surcharge: updates.surcharge,
      note: updates.note,
    });

    // Optimistic update for immediate UI feedback
    if (onUpdateOrder) {
      const newItems = [...order.items];
      newItems[index] = { ...newItems[index], ...updates };

      // Create ItemModifiedEvent for local display
      const event: ItemModifiedEvent = {
        id: uuidv4(),
        type: OrderEventType.ITEM_MODIFIED,
        timestamp: Date.now(),
        data: {
          instanceId: instanceId,
          itemName: item.name,
          externalId: item.externalId ? String(item.externalId) : undefined,
          changes: updates
        }
      };

      // Convert to timeline event
      const timelineEvent = convertEventToTimelineEvent(event);

      const updatedOrder = recalculateOrderTotal({
        ...order,
        items: newItems,
        timeline: [...(order.timeline || []), timelineEvent]
      });

      onUpdateOrder(updatedOrder);
    }
    setEditingItem(null);
  }, [order, onUpdateOrder]);

  const handleDeleteItem = React.useCallback(async (index: number, _options?: { userId?: string }) => {
    const item = order.items[index];
    const instanceId = item.instanceId || `item-${index}`;
    const paidQty = order.paidItemQuantities?.[instanceId] || 0;

    // Case 1: Partially paid item - Remove unpaid portion
    if (paidQty > 0 && paidQty < item.quantity) {
      const qtyToRemove = item.quantity - paidQty;

      // Send command to backend - state will be updated via event
      await orderOps.removeItem(order.key, instanceId, 'Removed unpaid portion', qtyToRemove);

      // Optimistic update for immediate UI feedback
      if (onUpdateOrder) {
        const newItems = order.items.map((it, idx) => {
          if (idx === index) {
            return { ...it, quantity: paidQty };
          }
          return it;
        });

        const event: ItemRemovedEvent = {
          id: uuidv4(),
          type: OrderEventType.ITEM_REMOVED,
          timestamp: Date.now(),
          data: {
            instanceId: instanceId,
            itemName: item.name,
            externalId: item.externalId ? String(item.externalId) : undefined,
            quantity: qtyToRemove,
            reason: 'Removed unpaid portion'
          }
        };

        const timelineEvent = convertEventToTimelineEvent(event);

        const updatedOrder = recalculateOrderTotal({
          ...order,
          items: newItems,
          timeline: [...(order.timeline || []), timelineEvent]
        });

        onUpdateOrder(updatedOrder);
      }
      return;
    }

    // Case 2: Fully paid or Unpaid - Remove (Soft Delete)
    await orderOps.removeItem(order.key, instanceId, 'Removed from payment screen');

    // Optimistic update for immediate UI feedback
    if (onUpdateOrder) {
      const newItems = order.items.map((it, idx) => {
        if (idx === index) {
          return { ...it, _removed: true };
        }
        return it;
      });

      const event: ItemRemovedEvent = {
        id: uuidv4(),
        type: OrderEventType.ITEM_REMOVED,
        timestamp: Date.now(),
        data: {
          instanceId: instanceId,
          itemName: item.name,
          externalId: item.externalId ? String(item.externalId) : undefined,
          reason: 'Removed from payment screen'
        }
      };

      const timelineEvent = convertEventToTimelineEvent(event);

      const updatedOrder = recalculateOrderTotal({
        ...order,
        items: newItems,
        timeline: [...(order.timeline || []), timelineEvent]
      });

      onUpdateOrder(updatedOrder);
    }
    setEditingItem(null);
  }, [order, onUpdateOrder]);

  const {
    displayOriginalPrice,
    displayTotalDiscount,
    displayTotalSurcharge,
    displayFinalTotal
  } = React.useMemo(() => {
    const { totalOriginalPrice, totalItemDiscount, totalItemSurcharge } = order.items
      .filter(item => !item._removed)
      .reduce(
        (acc, item) => {
          const quantity = item.quantity;
          const optionsModifier = calculateOptionsModifier(item.selectedOptions).toNumber();
          const basePrice = (item.originalPrice ?? item.price) + optionsModifier;

          const unitDiscount = calculateDiscountAmount(basePrice, item.discountPercent || 0).toNumber();
          const unitSurcharge = item.surcharge || 0;

          return {
            totalOriginalPrice: Currency.add(acc.totalOriginalPrice, Currency.mul(basePrice, quantity)).toNumber(),
            totalItemDiscount: Currency.add(acc.totalItemDiscount, Currency.mul(unitDiscount, quantity)).toNumber(),
            totalItemSurcharge: Currency.add(acc.totalItemSurcharge, Currency.mul(unitSurcharge, quantity)).toNumber(),
          };
        },
        { totalOriginalPrice: 0, totalItemDiscount: 0, totalItemSurcharge: 0 }
      );

    const displayOriginalPrice = totalOriginalPrice;
    const displayTotalDiscount = totalItemDiscount;
    const displayTotalSurcharge = totalItemSurcharge;
    const displayFinalTotal = order.total;

    return {
      displayOriginalPrice,
      displayTotalDiscount,
      displayTotalSurcharge,
      displayFinalTotal,
    };
  }, [order.items, order.total]);

  // Use backend-provided unpaidQuantity for each item
  const unpaidItems = React.useMemo(() => {
    return order.items
      .filter(item => !item._removed && (item.unpaidQuantity ?? item.quantity) > 0)
      .map(item => ({
        ...item,
        quantity: item.unpaidQuantity ?? item.quantity, // Use unpaid quantity for display
      }));
  }, [order.items]);

  React.useEffect(() => {
    if (activeTab !== 'TIMELINE') return;
    // Timeline events come from server via order.timeline
    setLazyTimeline(order.timeline || []);
  }, [activeTab, order.timeline]);

  return (
    <div className="w-[400px] bg-white h-full border-r border-gray-200 flex flex-col shadow-xl relative">
      {/* Header */}
      <div className="p-4 border-b border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <h1 className="text-lg font-bold text-gray-800 leading-tight">
              {t('checkout.tableOrder')} {order.zoneName }-{order.tableName}
            </h1>
            <div className="text-xs text-gray-500 flex items-center gap-2 mt-1">
              <span>
                {order.guestCount} {t('table.guests')}
              </span>
              <span className="w-1 h-1 rounded-full bg-gray-300" />
              <span>{new Date(order.startTime).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowQuickAdd(true)}
              className="px-3 py-2 bg-red-50 hover:bg-red-100 rounded-lg text-red-500 transition-colors flex items-center gap-1.5"
              title={t('pos.quickAdd.title')}
            >
              <ShoppingBag size={18} />
              <span className="text-sm font-bold">{t('pos.quickAdd.title')}</span>
            </button>

            {onManage && !order.isRetail && (
            <button
              onClick={onManage}
              className="p-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-gray-600 transition-colors"
            >
              <Settings size={20} />
            </button>
          )}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-gray-200">
        <button
          onClick={() => setActiveTab('ITEMS')}
          className={`flex-1 py-3 text-sm font-bold flex justify-center items-center gap-2 transition-colors border-b-2 ${
            activeTab === 'ITEMS'
              ? 'text-[#FF5E5E] border-[#FF5E5E] bg-red-50'
              : 'text-gray-500 border-transparent hover:bg-gray-50'
          }`}
        >
          <List size={16} /> {t('checkout.items.unpaid')}
        </button>
        <button
          onClick={() => setActiveTab('TIMELINE')}
          className={`flex-1 py-3 text-sm font-bold flex justify-center items-center gap-2 transition-colors border-b-2 ${
            activeTab === 'TIMELINE'
              ? 'text-blue-600 border-blue-600 bg-blue-50'
              : 'text-gray-500 border-transparent hover:bg-gray-50'
          }`}
        >
          <Clock size={16} /> {t('checkout.timeline.title')}
        </button>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto p-4 bg-white">
        {activeTab === 'ITEMS' ? (
          <OrderItemsSummary
            items={order.items.filter(i => !i._removed)}
            unpaidItems={unpaidItems}
            mode="SELECT"
            selectedQuantities={{}}
            onUpdateSelectedQty={() => {}}
            onEditItem={handleEditItem}
            t={t}
            paidItemQuantities={order.paidItemQuantities}
          />
        ) : (
          <Suspense fallback={
            <div className="flex items-center justify-center h-32">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            </div>
          }>
            <TimelineList events={lazyTimeline || []} />
          </Suspense>
        )}
      </div>

      {/* Footer */}
      <div className="p-5 bg-gray-50 border-t border-gray-200 space-y-2">
        {/* 1. Original Price (Subtotal) */}
        <div className="flex justify-between items-end">
          <span className="text-gray-500 font-medium text-sm">{t('pos.cart.original')}</span>
          <span className="text-sm font-medium text-gray-900">{formatCurrency(displayOriginalPrice)}</span>
        </div>

        {/* 2. Total Discount (If any) */}
        {displayTotalDiscount > 0 && (
          <div className="flex justify-between items-end">
            <span className="text-orange-500 font-medium text-sm">
              {t('checkout.cart.discount')}
            </span>
            <span className="text-sm font-medium text-orange-500">
                -{formatCurrency(displayTotalDiscount)}
              </span>
          </div>
        )}

        {/* 3. Total Surcharge (If any, from Price Rules) */}
        {displayTotalSurcharge > 0 && (
          <div className="flex justify-between items-end">
            <span className="text-purple-500 font-medium text-sm">
              {t('pos.cart.surcharge')}
            </span>
            <span className="text-sm font-medium text-purple-500">
	              +{formatCurrency(displayTotalSurcharge)}
	            </span>
          </div>
        )}

        {/* 4. Final Price (Total) */}
        <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
          <span className="text-gray-800 font-bold text-base">{t('checkout.amount.total')}</span>
          <span className="text-2xl font-bold text-gray-900">{formatCurrency(displayFinalTotal)}</span>
        </div>

        {/* Paid & Remaining (If partial payment exists) */}
        {totalPaid > 0 && (
          <div className="pt-2 border-t border-gray-200 space-y-1">
	            <div className="flex justify-between items-end">
	              <span className="text-blue-600 font-medium text-xs">{t('checkout.amount.paid')}</span>
	              <span className="text-sm text-blue-600 font-semibold">{formatCurrency(totalPaid)}</span>
	            </div>
	            <div className="flex justify-between items-end">
	              <span className="text-red-600 font-medium text-xs">{t('checkout.amount.remaining')}</span>
	              <span className="text-xl font-bold text-red-600">{formatCurrency(remaining)}</span>
	            </div>
          </div>
        )}
      </div>

      {/* Quick Add Modal */}
      {showQuickAdd && (
        <QuickAddModal
          onClose={() => setShowQuickAdd(false)}
          onConfirm={async (items) => {
            // Send command to backend - state will be updated via event
            await orderOps.addItems(order.key, items);

            // Optimistic update for immediate UI feedback
            if (onUpdateOrder) {
               const event: any = {
                   id: uuidv4(),
                   type: OrderEventType.ITEMS_ADDED,
                   timestamp: Date.now(),
                   data: { items }
               };

               const newItems = mergeItemsIntoList(order.items, items);
               const timelineEvent = convertEventToTimelineEvent(event);

               const optimOrder = recalculateOrderTotal({
                   ...order,
                   items: newItems,
                   isPrePayment: false,
                   timeline: [...(order.timeline || []), timelineEvent]
               });

               onUpdateOrder(optimOrder);
            }

            setShowQuickAdd(false);
          }}
        />
      )}

      {/* Item Edit Modal */}
      {editingItem && (
        <CartItemDetailModal
          item={editingItem.item}
          onClose={() => setEditingItem(null)}
          onUpdate={(instanceId, updates, options) => {
            handleSaveItem(editingItem.index, updates, options);
          }}
          onRemove={(instanceId, options) => {
            handleDeleteItem(editingItem.index, options);
          }}
          readOnlyAttributes={true}
        />
      )}
    </div>
  );
});

OrderSidebar.displayName = 'OrderSidebar';
