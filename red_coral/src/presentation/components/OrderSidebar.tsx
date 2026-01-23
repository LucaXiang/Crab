import React, { useState, lazy, Suspense } from 'react';
import { HeldOrder, CartItem } from '@/core/domain/types';
import { Clock, List, Settings, ShoppingBag } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { OrderItemsSummary } from '@/screens/Checkout/OrderItemsSummary';
import { CartItemDetailModal } from '@/presentation/components/modals/CartItemDetailModal';
import { QuickAddModal } from '@/presentation/components/modals/QuickAddModal';
import * as orderOps from '@/core/stores/order/useOrderOperations';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useOrderTimeline } from '@/core/stores/order/useActiveOrdersStore';
import { formatCurrency } from '@/utils/currency';

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
  
  // 直接从 store 获取 timeline（不依赖 order.timeline）
  const timeline = useOrderTimeline(order.order_id);

  const handleEditItem = React.useCallback((item: CartItem) => {
    // Find index of item in order.items
    const index = order.items.findIndex((i) => 
      i.instance_id ? i.instance_id === item.instance_id : i.id === item.id
    );
    if (index !== -1) {
      setEditingItem({ item, index });
    }
  }, [order]);

  const handleSaveItem = React.useCallback(async (index: number, updates: Partial<CartItem>, _options?: { userId?: string }) => {
    const item = order.items[index];
    const instanceId = item.instance_id;

    // Send command to backend - state will be updated via event (Server Authority)
    await orderOps.modifyItem(order.order_id, instanceId, {
      price: updates.price,
      quantity: updates.quantity,
      manual_discount_percent: updates.manual_discount_percent,
      surcharge: updates.surcharge,
      note: updates.note,
    });

    setEditingItem(null);
  }, [order]);

  const handleDeleteItem = React.useCallback(async (index: number, _options?: { userId?: string }) => {
    const item = order.items[index];
    const instanceId = item.instance_id;
    const paidQty = order.paid_item_quantities?.[instanceId] || 0;

    // Case 1: Partially paid item - Remove unpaid portion
    if (paidQty > 0 && paidQty < item.quantity) {
      const qtyToRemove = item.quantity - paidQty;

      // Send command to backend - state will be updated via event (Server Authority)
      await orderOps.removeItem(order.order_id, instanceId, 'Removed unpaid portion', qtyToRemove);
      return;
    }

    // Case 2: Fully paid or Unpaid - Remove (Soft Delete)
    // Send command to backend - state will be updated via event (Server Authority)
    await orderOps.removeItem(order.order_id, instanceId, 'Removed from payment screen');

    setEditingItem(null);
  }, [order]);

  // Use server-provided financial totals (authoritative)
  const displayOriginalPrice = order.original_total;
  const displayTotalDiscount = order.total_discount;
  const displayTotalSurcharge = order.total_surcharge;
  const displayFinalTotal = order.total;
  const displayRemainingAmount = order.remaining_amount;

  // Use backend-provided unpaidQuantity for each item
  const unpaidItems = React.useMemo(() => {
    return order.items
      .filter(item => !item._removed && (item.unpaid_quantity ?? item.quantity) > 0)
      .map(item => ({
        ...item,
        quantity: item.unpaid_quantity ?? item.quantity, // Use unpaid quantity for display
      }));
  }, [order.items]);

  // Timeline 现在直接从 store 获取，不需要 useEffect

  return (
    <div className="w-[400px] bg-white h-full border-r border-gray-200 flex flex-col shadow-xl relative">
      {/* Header */}
      <div className="p-4 border-b border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <h1 className="text-lg font-bold text-gray-800 leading-tight">
              {t('checkout.table_order')} {order.zone_name }-{order.table_name}
            </h1>
            <div className="text-xs text-gray-500 flex items-center gap-2 mt-1">
              <span>
                {order.guest_count} {t('table.guests')}
              </span>
              <span className="w-1 h-1 rounded-full bg-gray-300" />
              <span>{new Date(order.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowQuickAdd(true)}
              className="px-3 py-2 bg-red-50 hover:bg-red-100 rounded-lg text-red-500 transition-colors flex items-center gap-1.5"
              title={t('pos.quick_add.title')}
            >
              <ShoppingBag size={18} />
              <span className="text-sm font-bold">{t('pos.quick_add.title')}</span>
            </button>

            {onManage && !order.is_retail && (
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
            paid_item_quantities={order.paid_item_quantities}
          />
        ) : (
          <Suspense fallback={
            <div className="flex items-center justify-center h-32">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            </div>
          }>
            <TimelineList events={timeline} />
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
        {order.paid_amount > 0 && (
          <div className="pt-2 border-t border-gray-200 space-y-1">
	            <div className="flex justify-between items-end">
	              <span className="text-blue-600 font-medium text-xs">{t('checkout.amount.paid')}</span>
	              <span className="text-sm text-blue-600 font-semibold">{formatCurrency(order.paid_amount)}</span>
	            </div>
	            <div className="flex justify-between items-end">
	              <span className="text-red-600 font-medium text-xs">{t('checkout.amount.remaining')}</span>
	              <span className="text-xl font-bold text-red-600">{formatCurrency(displayRemainingAmount)}</span>
	            </div>
          </div>
        )}
      </div>

      {/* Quick Add Modal */}
      {showQuickAdd && (
        <QuickAddModal
          onClose={() => setShowQuickAdd(false)}
          onConfirm={async (items) => {
            // Send command to backend - state will be updated via event (Server Authority)
            await orderOps.addItems(order.order_id, items);
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
