/**
 * CheckoutScreen - 结账界面（使用新的支付流程）
 *
 * 职责：
 * 1. 显示订单信息和商品列表
 * 2. 提供订单折扣和商品编辑功能
 * 3. 集成新的 PaymentFlow 组件处理支付
 * 4. 处理订单作废
 */

import React, { useState, useCallback, useEffect } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { useConfirm } from '@/hooks/useConfirm';
import { PaymentFlow } from './payment/PaymentFlow';
import { useOrderCommands } from '@/core/stores/order';
import { VoidReasonModal } from './VoidReasonModal';
import { SupervisorAuthModal } from '@/presentation/components/auth/SupervisorAuthModal';
import { usePermission } from '@/hooks/usePermission';
import { Permission } from '@/core/domain/types';
import { useCheckoutActions } from '@/core/stores/order/useCheckoutStore';

interface CheckoutScreenProps {
  order: HeldOrder;
  onCancel: () => void;
  onComplete: () => void;
  onVoid?: (reason?: string) => void;
  onUpdateOrder?: (order: HeldOrder) => void;
  onManageTable?: () => void;
}

export const CheckoutScreen: React.FC<CheckoutScreenProps> = ({
  order,
  onCancel,
  onComplete,
  onVoid: propOnVoid,
  onUpdateOrder: propOnUpdateOrder,
  onManageTable,
}) => {
  const { t } = useI18n();
  const { dialogProps } = useConfirm();
  const { voidOrder } = useOrderCommands();
  const { hasPermission } = usePermission();
  const { setCheckoutOrder } = useCheckoutActions();
  
  const [isVoidModalOpen, setIsVoidModalOpen] = useState(false);
  const [isSupervisorModalOpen, setIsSupervisorModalOpen] = useState(false);
  const [pendingVoidReason, setPendingVoidReason] = useState<string | null>(null);

  // Local state for modified order (for discounts and item edits)
  const [localOrder, setLocalOrder] = useState<HeldOrder>(order);

  // Keep localOrder in sync when the incoming order prop changes (e.g., after merge/move)
  useEffect(() => {
    setLocalOrder(order);
  }, [order]);

  const handleUpdateOrder = useCallback((updatedOrder: HeldOrder) => {
    // Preserve is_retail flag if it exists in current localOrder but missing in updatedOrder
    // (because EventStore doesn't persist is_retail for Retail orders)
    const preservedOrder = {
        ...updatedOrder,
        is_retail: localOrder.is_retail || updatedOrder.is_retail
    };

    setLocalOrder(preservedOrder);
    setCheckoutOrder(preservedOrder); // Sync to global store to ensure persistence and other observers update
    
    if (propOnUpdateOrder) {
      propOnUpdateOrder(preservedOrder);
    }

    // 不再在 ACTIVE 阶段持久化到后端，仅更新本地状态
  }, [propOnUpdateOrder, localOrder.is_retail, setCheckoutOrder]);

  /**
   * 处理支付完成
   */
  const handlePaymentComplete = useCallback(() => {
    onComplete();
  }, []);

  /**
   * 打开作废订单模态框
   */
  const handleVoidClick = useCallback(() => {
    setIsVoidModalOpen(true);
  }, []);

  /**
   * 执行作废操作
   */
  const processVoid = useCallback(async (reason: string) => {
    if (propOnVoid) {
      propOnVoid(reason);
      setIsVoidModalOpen(false);
    } else {
      const orderKey = order.key || order.order_id || String(order.table_id || '');

      // Execute async command
      const response = await voidOrder(orderKey, reason);

      if (!response.success) {
        // Display error to user
        console.error('Void order failed:', response.error);
        // TODO: Add toast notification
        setIsVoidModalOpen(false);
        return;
      }

      setIsVoidModalOpen(false);
      onCancel();
    }
  }, [propOnVoid, voidOrder, order, onCancel]);

  /**
   * 确认作废订单
   */
  const handleVoidConfirm = useCallback((reason: string) => {
    // If user cannot void orders, require supervisor auth
    if (!hasPermission(Permission.VOID_ORDER)) {
      setPendingVoidReason(reason);
      setIsVoidModalOpen(false); // Close void modal first
      setIsSupervisorModalOpen(true);
    } else {
      processVoid(reason);
    }
  }, [hasPermission, processVoid]);

  const handleSupervisorSuccess = useCallback(() => {
    if (pendingVoidReason) {
      processVoid(pendingVoidReason);
      setPendingVoidReason(null);
    }
    setIsSupervisorModalOpen(false);
  }, [pendingVoidReason, processVoid]);

  return (
    <>
      <div className="h-full flex flex-col bg-gray-50 relative">
        {/* Payment Flow */}
        <div className="flex-1 overflow-hidden">
          <PaymentFlow 
            order={localOrder} 
            onComplete={handlePaymentComplete} 
            onCancel={onCancel}
            onUpdateOrder={handleUpdateOrder}
            onVoid={handleVoidClick}
            onManageTable={onManageTable}
          />
        </div>
      </div>

      <VoidReasonModal 
        isOpen={isVoidModalOpen}
        onClose={() => setIsVoidModalOpen(false)}
        onConfirm={handleVoidConfirm}
      />

      <SupervisorAuthModal
        isOpen={isSupervisorModalOpen}
        onClose={() => {
          setIsSupervisorModalOpen(false);
          setPendingVoidReason(null);
        }}
        onSuccess={handleSupervisorSuccess}
        requiredPermission={Permission.VOID_ORDER}
        actionDescription={t('checkout.void.authRequired')}
      />

      {/* Confirm Dialog */}
      {dialogProps && dialogProps.isOpen && (
        <div className="fixed inset-0 z-[70] bg-black/50 flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full p-6">
            <h3 className="text-xl font-bold text-gray-800 mb-2">{dialogProps.title}</h3>
            <p className="text-gray-600 mb-6">{dialogProps.description}</p>
            <div className="flex gap-3">
              <button
                onClick={dialogProps.onCancel}
                className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold hover:bg-gray-300 transition-colors"
              >
                {dialogProps.cancelText || t('common.action.cancel')}
              </button>
              <button
                onClick={dialogProps.onConfirm}
                className={`flex-1 px-4 py-3 text-white rounded-xl font-bold transition-colors ${
                  dialogProps.variant === 'danger'
                    ? 'bg-red-500 hover:bg-red-600'
                    : 'bg-blue-500 hover:bg-blue-600'
                }`}
              >
                {dialogProps.confirmText || t('common.action.confirm')}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
