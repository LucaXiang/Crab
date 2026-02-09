import React, { useState, lazy, Suspense } from 'react';
import { HeldOrder, CartItem } from '@/core/domain/types';
import { Clock, List, Settings, ShoppingBag, Percent, Gift, TrendingUp } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { OrderItemsSummary } from '@/screens/Checkout/OrderItemsSummary';
import { CartItemDetailModal } from '@/presentation/components/modals/CartItemDetailModal';
import { QuickAddModal } from '@/presentation/components/modals/QuickAddModal';
import * as orderOps from '@/core/stores/order/commands';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useOrderTimeline } from '@/core/stores/order/useActiveOrdersStore';
import { formatCurrency, Currency } from '@/utils/currency';

// Lazy load TimelineList - only loads when user clicks Timeline tab
const TimelineList = lazy(() =>
  import('@/shared/components/TimelineList').then(module => ({ default: module.TimelineList }))
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
    if (item.is_comped) return; // Comped items are locked
    const index = order.items.findIndex((i) =>
      i.instance_id ? i.instance_id === item.instance_id : i.id === item.id
    );
    if (index !== -1) {
      // Pass server-authoritative data directly — backend handles split logic
      setEditingItem({ item, index });
    }
  }, [order]);

  const handleSaveItem = React.useCallback(async (index: number, updates: Partial<CartItem>, authorizer?: { id: number; name: string }) => {
    const item = order.items[index];
    const instanceId = item.instance_id;

    // Send command to backend - state will be updated via event (Server Authority)
    await orderOps.modifyItem(order.order_id, instanceId, {
      price: updates.price ?? undefined,
      quantity: updates.quantity ?? undefined,
      manual_discount_percent: updates.manual_discount_percent ?? undefined,
      note: updates.note ?? undefined,
      selected_options: updates.selected_options ?? undefined,
      selected_specification: updates.selected_specification ?? undefined,
    }, authorizer);

    setEditingItem(null);
  }, [order]);

  const handleDeleteItem = React.useCallback(async (index: number, authorizer?: { id: number; name: string }) => {
    const item = order.items[index];
    // Server Authority: backend handles paid item protection, no local computation
    await orderOps.removeItem(order.order_id, item.instance_id, 'Removed from payment screen', undefined, authorizer);
    setEditingItem(null);
  }, [order]);

  // Use server-provided financial totals (authoritative)
  const displayOriginalPrice = order.original_total;
  // Combined item+orderRule discount/surcharge (excluding order manual)
  const displayItemDiscount = Currency.sub(order.total_discount, order.order_manual_discount_amount).toNumber();
  const displayItemSurcharge = Currency.sub(order.total_surcharge, order.order_manual_surcharge_amount).toNumber();

  // Split: rule discount/surcharge (item-level + order-level rules)
  const itemRuleDiscount = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_discount_amount, item.quantity)).toNumber(), 0);
  const itemRuleSurcharge = order.items
    .filter(i => !i._removed)
    .reduce((sum, item) => Currency.add(sum, Currency.mul(item.rule_surcharge_amount, item.quantity)).toNumber(), 0);
  const totalRuleDiscount = Currency.add(itemRuleDiscount, order.order_rule_discount_amount).toNumber();
  const totalRuleSurcharge = Currency.add(itemRuleSurcharge, order.order_rule_surcharge_amount).toNumber();
  // Manual item discount = total item discount - rule discount
  const manualItemDiscount = Currency.sub(displayItemDiscount, totalRuleDiscount).toNumber();

  const displayFinalTotal = order.total;
  const displayRemainingAmount = order.remaining_amount;

  // Timeline 现在直接从 store 获取，不需要 useEffect

  return (
    <div className="w-[calc(380px+3.5rem)] bg-white h-full border-r border-gray-200 flex flex-col shadow-xl relative">
      {/* Header */}
      <div className="p-5 border-b border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <h1 className="text-xl font-bold text-gray-800 leading-tight">
              {order.is_retail ? (
                <>
                  {t('checkout.retail_order')}
                  {order.queue_number != null && (
                    <span className="ml-2 text-blue-600">#{String(order.queue_number).padStart(3, '0')}</span>
                  )}
                </>
              ) : (
                <>
                  {t('checkout.table_order')} {order.zone_name}-{order.table_name}
                </>
              )}
            </h1>
            <div className="text-sm text-gray-500 flex items-center gap-2 mt-1">
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
              className="px-4 py-2.5 bg-primary-50 hover:bg-primary-100 rounded-lg text-primary-500 transition-colors flex items-center gap-1.5"
              title={t('pos.quick_add.title')}
            >
              <ShoppingBag size={20} />
              <span className="text-base font-bold">{t('pos.quick_add.title')}</span>
            </button>

            {onManage && !order.is_retail && (
            <button
              onClick={onManage}
              className="p-2.5 bg-gray-100 hover:bg-gray-200 rounded-lg text-gray-600 transition-colors"
            >
              <Settings size={24} />
            </button>
          )}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-gray-200">
        <button
          onClick={() => setActiveTab('ITEMS')}
          className={`flex-1 py-4 text-base font-bold flex justify-center items-center gap-2 transition-colors border-b-2 ${
            activeTab === 'ITEMS'
              ? 'text-primary-500 border-primary-500 bg-primary-50'
              : 'text-gray-500 border-transparent hover:bg-gray-50'
          }`}
        >
          <List size={20} /> {t('checkout.items.unpaid')}
        </button>
        <button
          onClick={() => setActiveTab('TIMELINE')}
          className={`flex-1 py-4 text-base font-bold flex justify-center items-center gap-2 transition-colors border-b-2 ${
            activeTab === 'TIMELINE'
              ? 'text-blue-600 border-blue-600 bg-blue-50'
              : 'text-gray-500 border-transparent hover:bg-gray-50'
          }`}
        >
          <Clock size={20} /> {t('checkout.timeline.title')}
        </button>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto p-4 bg-white">
        {activeTab === 'ITEMS' ? (
          <OrderItemsSummary
            items={order.items.filter(i => !i._removed)}
            onEditItem={handleEditItem}
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
        {/* 1. Original Price */}
        <div className="flex justify-between items-end">
          <span className="text-gray-500 font-medium text-sm">{t('pos.cart.original')}</span>
          <span className="text-sm font-medium text-gray-900">{formatCurrency(displayOriginalPrice)}</span>
        </div>

        {/* 2. Comp reduction (赠送减免) */}
        {order.comp_total_amount > 0 && (
          <div className="flex justify-between items-center">
            <span className="text-emerald-600 font-medium text-sm flex items-center gap-1">
              <Gift size={12} />
              {t('checkout.breakdown.comp')}
            </span>
            <span className="text-sm font-medium text-emerald-600">
              -{formatCurrency(order.comp_total_amount)}
            </span>
          </div>
        )}

        {/* 3. Manual item discount (手动折扣) */}
        {manualItemDiscount > 0 && (
          <div className="flex justify-between items-end">
            <span className="text-orange-500 font-medium text-sm">
              {t('checkout.breakdown.manual_discount')}
            </span>
            <span className="text-sm font-medium text-orange-500">
              -{formatCurrency(manualItemDiscount)}
            </span>
          </div>
        )}

        {/* 4. Rule discount (规则折扣) */}
        {totalRuleDiscount > 0 && (
          <div className="flex justify-between items-end">
            <span className="text-amber-600 font-medium text-sm">
              {t('checkout.breakdown.rule_discount')}
            </span>
            <span className="text-sm font-medium text-amber-600">
              -{formatCurrency(totalRuleDiscount)}
            </span>
          </div>
        )}

        {/* 5. Rule surcharge (规则附加费) */}
        {totalRuleSurcharge > 0 && (
          <div className="flex justify-between items-end">
            <span className="text-purple-500 font-medium text-sm">
              {t('checkout.breakdown.rule_surcharge')}
            </span>
            <span className="text-sm font-medium text-purple-500">
              +{formatCurrency(totalRuleSurcharge)}
            </span>
          </div>
        )}

        {/* 6. Order manual discount (整单手动折扣) */}
        {order.order_manual_discount_amount > 0 && (
          <div className="flex justify-between items-center">
            <span className="text-orange-500 font-medium text-sm flex items-center gap-1">
              <Percent size={12} />
              {t('checkout.breakdown.order_discount')}
              {order.order_manual_discount_percent != null && (
                <span className="text-xs opacity-75">({order.order_manual_discount_percent}%)</span>
              )}
            </span>
            <span className="text-sm font-medium text-orange-500">
              -{formatCurrency(order.order_manual_discount_amount)}
            </span>
          </div>
        )}

        {/* 7. Order manual surcharge (整单手动附加费) */}
        {order.order_manual_surcharge_amount > 0 && (
          <div className="flex justify-between items-center">
            <span className="text-purple-500 font-medium text-sm flex items-center gap-1">
              <TrendingUp size={12} />
              {t('checkout.breakdown.order_surcharge')}
              {order.order_manual_surcharge_percent != null && (
                <span className="text-xs opacity-75">({order.order_manual_surcharge_percent}%)</span>
              )}
            </span>
            <span className="text-sm font-medium text-purple-500">
              +{formatCurrency(order.order_manual_surcharge_amount)}
            </span>
          </div>
        )}

        {/* 8. Final Price (Total) */}
        <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
          <span className="text-gray-800 font-bold text-base">{t('checkout.amount.total')}</span>
          <span className="text-2xl font-bold text-gray-900">{formatCurrency(displayFinalTotal)}</span>
        </div>

        {/* Paid & Remaining */}
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
          readOnlyAttributes={false}
        />
      )}
    </div>
  );
});

OrderSidebar.displayName = 'OrderSidebar';
