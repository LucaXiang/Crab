/**
 * Order Commands Hook
 *
 * Hook for sending order commands to the server.
 * This hook does NOT hold any state - it's purely for command execution.
 *
 * Architecture:
 * - All methods return Promise<CommandResponse>
 * - Commands are sent via Tauri invoke
 * - ACK/NACK responses indicate success/failure
 * - State updates come via separate event broadcasts (handled by useActiveOrdersStore)
 *
 * Usage:
 * const { addItems, completeOrder } = useOrderCommands();
 * const response = await addItems(orderId, items);
 * if (!response.success) {
 *   toast.error(response.error?.message);
 * }
 */

import { useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { createCommand } from './commandUtils';
import type {
  OrderCommand,
  CommandResponse,
  CartItemInput,
  ItemChanges,
  SplitItem,
  PaymentMethod,
  ServiceType,
} from '@/core/domain/types/orderEvent';

// ============================================================================
// Types
// ============================================================================

export interface OpenTableParams {
  table_id?: string;
  table_name?: string;
  zone_id?: string;
  zone_name?: string;
  guest_count?: number;
  is_retail: boolean;
}

export interface PaymentInput {
  method: PaymentMethod;
  amount: number;
  tendered?: number;
  note?: string;
}

/**
 * Send command to backend
 *
 * IMPORTANT: This function checks command lock before execution.
 * If the system is syncing or disconnected, it will return an error
 * immediately without sending the command to prevent dirty data.
 */
async function sendCommand(command: OrderCommand): Promise<CommandResponse> {
  // Import here to avoid circular dependency
  const { checkCommandLock } = await import('@/core/hooks/useCommandLock');

  // Check command lock before execution
  const lockCheck = checkCommandLock();
  if (!lockCheck.canExecute) {
    console.warn(`[Command] Blocked: ${command.payload.type} - ${lockCheck.error}`);
    return {
      command_id: command.command_id,
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: lockCheck.error ?? 'System is busy, please wait',
      },
    };
  }

  try {
    // The backend now returns ApiResponse<CommandResponse>
    // invokeApi handles the unwrapping and error throwing
    const response = await invokeApi<CommandResponse>('order_execute_command', {
      command,
    });
    return response;
  } catch (error: unknown) {
    // Convert invoke errors to CommandResponse format
    console.error('[Command] Failed:', error);
    return {
      command_id: command.command_id,
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: error instanceof Error ? error.message : 'Command execution failed',
      },
    };
  }
}

// ============================================================================
// Hook Implementation
// ============================================================================

export function useOrderCommands() {
  // ==================== Order Lifecycle ====================

  /**
   * Open a new table/order
   */
  const openTable = useCallback(
    async (params: OpenTableParams): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'OPEN_TABLE',
        table_id: params.table_id || null,
        table_name: params.table_name || null,
        zone_id: params.zone_id || null,
        zone_name: params.zone_name || null,
        guest_count: params.guest_count ?? 1,
        is_retail: params.is_retail,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Complete an order (checkout)
   * Note: receipt_number is server-generated at OpenTable, no need to pass
   */
  const completeOrder = useCallback(
    async (orderId: string, serviceType?: ServiceType | null): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'COMPLETE_ORDER',
        order_id: orderId,
        service_type: serviceType ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /** 作废订单选项 */
  interface VoidOrderOptions {
    voidType?: 'CANCELLED' | 'LOSS_SETTLED';
    lossReason?: 'CUSTOMER_FLED' | 'CUSTOMER_INSOLVENT' | 'OTHER';
    lossAmount?: number;
    note?: string;
    authorizerId?: string | null;
    authorizerName?: string | null;
  }

  /**
   * Void an order
   */
  const voidOrder = useCallback(
    async (orderId: string, options?: VoidOrderOptions): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'VOID_ORDER',
        order_id: orderId,
        void_type: options?.voidType ?? 'CANCELLED',
        loss_reason: options?.lossReason ?? null,
        loss_amount: options?.lossAmount ?? null,
        note: options?.note ?? null,
        authorizer_id: options?.authorizerId ?? null,
        authorizer_name: options?.authorizerName ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Item Operations ====================

  /**
   * Add items to an order
   */
  const addItems = useCallback(
    async (orderId: string, items: CartItemInput[]): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'ADD_ITEMS',
        order_id: orderId,
        items,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Modify an item in the order
   */
  const modifyItem = useCallback(
    async (
      orderId: string,
      instanceId: string,
      changes: ItemChanges,
      authorizer?: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'MODIFY_ITEM',
        order_id: orderId,
        instance_id: instanceId,
        changes,
        authorizer_id: authorizer?.id ?? null,
        authorizer_name: authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Remove an item from the order
   * @param quantity - If provided, removes only this quantity; otherwise removes all
   */
  const removeItem = useCallback(
    async (
      orderId: string,
      instanceId: string,
      quantity?: number,
      reason?: string,
      authorizer?: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'REMOVE_ITEM',
        order_id: orderId,
        instance_id: instanceId,
        quantity: quantity ?? null,
        reason: reason ?? null,
        authorizer_id: authorizer?.id ?? null,
        authorizer_name: authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Payment Operations ====================

  /**
   * Add a payment to the order
   */
  const addPayment = useCallback(
    async (orderId: string, payment: PaymentInput): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'ADD_PAYMENT',
        order_id: orderId,
        payment: {
          method: payment.method,
          amount: payment.amount,
          tendered: payment.tendered ?? null,
          note: payment.note ?? null,
        },
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Cancel a payment
   */
  const cancelPayment = useCallback(
    async (
      orderId: string,
      paymentId: string,
      reason?: string,
      authorizer?: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'CANCEL_PAYMENT',
        order_id: orderId,
        payment_id: paymentId,
        reason: reason ?? null,
        authorizer_id: authorizer?.id ?? null,
        authorizer_name: authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Split by items (菜品分单)
   */
  const splitByItems = useCallback(
    async (
      orderId: string,
      paymentMethod: string,
      items: SplitItem[],
      tendered?: number,
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'SPLIT_BY_ITEMS',
        order_id: orderId,
        payment_method: paymentMethod,
        items,
        tendered: tendered ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Split by amount (金额分单)
   */
  const splitByAmount = useCallback(
    async (
      orderId: string,
      splitAmount: number,
      paymentMethod: string,
      tendered?: number,
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'SPLIT_BY_AMOUNT',
        order_id: orderId,
        split_amount: splitAmount,
        payment_method: paymentMethod,
        tendered: tendered ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Start AA split (锁定人数 + 支付第一份)
   */
  const startAaSplit = useCallback(
    async (
      orderId: string,
      totalShares: number,
      shares: number,
      paymentMethod: string,
      tendered?: number,
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'START_AA_SPLIT',
        order_id: orderId,
        total_shares: totalShares,
        shares,
        payment_method: paymentMethod,
        tendered: tendered ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Pay AA split (后续 AA 支付)
   */
  const payAaSplit = useCallback(
    async (
      orderId: string,
      shares: number,
      paymentMethod: string,
      tendered?: number,
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'PAY_AA_SPLIT',
        order_id: orderId,
        shares,
        payment_method: paymentMethod,
        tendered: tendered ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Table Operations ====================

  /**
   * Move order to a different table
   */
  const moveOrder = useCallback(
    async (
      orderId: string,
      targetTableId: string,
      targetTableName: string,
      targetZoneId?: string | null,
      targetZoneName?: string | null,
      authorizer?: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'MOVE_ORDER',
        order_id: orderId,
        target_table_id: targetTableId,
        target_table_name: targetTableName,
        target_zone_id: targetZoneId ?? null,
        target_zone_name: targetZoneName ?? null,
        authorizer_id: authorizer?.id ?? null,
        authorizer_name: authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Merge two orders
   * Source order items are moved to target order
   */
  const mergeOrders = useCallback(
    async (
      sourceOrderId: string,
      targetOrderId: string,
      authorizer?: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'MERGE_ORDERS',
        source_order_id: sourceOrderId,
        target_order_id: targetOrderId,
        authorizer_id: authorizer?.id ?? null,
        authorizer_name: authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Order Settings ====================

  /**
   * Update order info (guest count, table name, etc.)
   * Note: receipt_number is immutable (set at OpenTable)
   */
  const updateOrderInfo = useCallback(
    async (
      orderId: string,
      info: {
        guest_count?: number;
        table_name?: string;
        is_pre_payment?: boolean;
      }
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'UPDATE_ORDER_INFO',
        order_id: orderId,
        guest_count: info.guest_count ?? null,
        table_name: info.table_name ?? null,
        is_pre_payment: info.is_pre_payment ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Comp Operations ====================

  /**
   * Comp (赠送) an item - splits quantity and marks as free
   */
  const compItem = useCallback(
    async (
      orderId: string,
      instanceId: string,
      quantity: number,
      reason: string,
      authorizer: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'COMP_ITEM',
        order_id: orderId,
        instance_id: instanceId,
        quantity,
        reason,
        authorizer_id: authorizer.id,
        authorizer_name: authorizer.name,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Uncomp (撤销赠送) an item - restore original price
   */
  const uncompItem = useCallback(
    async (
      orderId: string,
      instanceId: string,
      authorizer: { id: string; name: string },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'UNCOMP_ITEM',
        order_id: orderId,
        instance_id: instanceId,
        authorizer_id: authorizer.id,
        authorizer_name: authorizer.name,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Order Note ====================

  /**
   * Add or clear order-level note (空字符串 = 清除)
   */
  const addOrderNote = useCallback(
    async (orderId: string, note: string): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'ADD_ORDER_NOTE',
        order_id: orderId,
        note,
      });

      return sendCommand(command);
    },
    []
  );

  // ==================== Order-level Adjustments ====================

  /**
   * Apply order-level manual discount (percent or fixed, mutually exclusive)
   * Both null = clear discount
   */
  const applyOrderDiscount = useCallback(
    async (
      orderId: string,
      options?: {
        discountPercent?: number;
        discountFixed?: number;
        reason?: string;
        authorizer?: { id: string; name: string };
      },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'APPLY_ORDER_DISCOUNT',
        order_id: orderId,
        discount_percent: options?.discountPercent ?? null,
        discount_fixed: options?.discountFixed ?? null,
        reason: options?.reason ?? null,
        authorizer_id: options?.authorizer?.id ?? null,
        authorizer_name: options?.authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  /**
   * Apply order-level surcharge (fixed amount)
   * null = clear surcharge
   */
  const applyOrderSurcharge = useCallback(
    async (
      orderId: string,
      options?: {
        surchargeAmount?: number;
        reason?: string;
        authorizer?: { id: string; name: string };
      },
    ): Promise<CommandResponse> => {
      const command = createCommand({
        type: 'APPLY_ORDER_SURCHARGE',
        order_id: orderId,
        surcharge_amount: options?.surchargeAmount ?? null,
        reason: options?.reason ?? null,
        authorizer_id: options?.authorizer?.id ?? null,
        authorizer_name: options?.authorizer?.name ?? null,
      });

      return sendCommand(command);
    },
    []
  );

  return {
    // Order Lifecycle
    openTable,
    completeOrder,
    voidOrder,
    // Item Operations
    addItems,
    modifyItem,
    removeItem,

    // Payment Operations
    addPayment,
    cancelPayment,
    splitByItems,
    splitByAmount,
    startAaSplit,
    payAaSplit,

    // Table Operations
    moveOrder,
    mergeOrders,

    // Order Settings
    updateOrderInfo,

    // Comp Operations
    compItem,
    uncompItem,

    // Order Note
    addOrderNote,

    // Order-level Adjustments
    applyOrderDiscount,
    applyOrderSurcharge,
  };
}

// ============================================================================
// Type Exports
// ============================================================================

export type OrderCommandsHook = ReturnType<typeof useOrderCommands>;
