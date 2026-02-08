/**
 * useRetailOrderRecovery Hook
 *
 * 在 POS 页面启动时检查是否有未完成的零售订单需要恢复。
 * 如果有，自动进入 checkout 页面继续处理。
 */

import { useEffect, useRef } from 'react';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import {
  getPendingRetailOrder,
  clearPendingRetailOrder,
} from '@/core/stores/order/retailOrderTracker';

interface UseRetailOrderRecoveryParams {
  setViewMode: (mode: 'pos' | 'checkout') => void;
  setCurrentOrderKey: (key: string | number | null) => void;
}

/**
 * 检查并恢复未完成的零售订单
 */
export function useRetailOrderRecovery({
  setViewMode,
  setCurrentOrderKey,
}: UseRetailOrderRecoveryParams): void {
  const hasChecked = useRef(false);

  useEffect(() => {
    if (hasChecked.current) return;

    const pending = getPendingRetailOrder();
    if (!pending) {
      hasChecked.current = true;
      return;
    }

    let cancelled = false;

    const checkPendingOrder = async () => {
      // 轮询等待订单出现在 store 中（最多等待 10 秒）
      const maxAttempts = 20;
      const interval = 500;

      for (let attempt = 0; attempt < maxAttempts; attempt++) {
        if (cancelled) return;

        const store = useActiveOrdersStore.getState();
        const order = store.getOrder(pending.orderId);

        if (order) {
          if (order.status === 'ACTIVE' && order.is_retail) {
            useCheckoutStore.getState().setCheckoutOrder(order);
            setCurrentOrderKey(pending.orderId);
            setViewMode('checkout');
            hasChecked.current = true;
            return;
          } else {
            clearPendingRetailOrder();
            hasChecked.current = true;
            return;
          }
        }

        await new Promise((resolve) => setTimeout(resolve, interval));
      }

      clearPendingRetailOrder();
      hasChecked.current = true;
    };

    // 立即开始检查
    checkPendingOrder();

    return () => {
      cancelled = true;
    };
  }, [setViewMode, setCurrentOrderKey]);
}
