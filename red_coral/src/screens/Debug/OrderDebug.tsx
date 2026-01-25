/**
 * Order Debug Page
 * 用于调试幽灵订单等问题，显示所有订单的详细信息
 */

import React, { useState, useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import {
  Bug,
  RefreshCw,
  Trash2,
  ChevronDown,
  ChevronRight,
  ArrowLeft,
  AlertTriangle,
  CheckCircle,
  Clock,
  MapPin,
  ShoppingCart,
  DollarSign,
  Hash,
} from 'lucide-react';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useOrderCommands } from '@/core/stores/order/useOrderCommands';
import { invokeApi } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/shared/components';
import type { OrderSnapshot, SyncResponse } from '@/core/domain/types/orderEvent';

export const OrderDebug: React.FC = () => {
  const navigate = useNavigate();
  const [expandedOrders, setExpandedOrders] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(false);
  const [isVoiding, setIsVoiding] = useState(false);
  const [confirmDialog, setConfirmDialog] = useState<{
    type: 'reset' | 'voidAll';
    isOpen: boolean;
  }>({ type: 'reset', isOpen: false });
  const { voidOrder } = useOrderCommands();

  // 获取 store 状态 - 使用 useShallow 避免不必要的重渲染
  const { ordersMap, timelines, lastSequence, connectionState, isInitialized, serverEpoch } =
    useActiveOrdersStore(useShallow((state) => ({
      ordersMap: state.orders,
      timelines: state.timelines,
      lastSequence: state.lastSequence,
      connectionState: state.connectionState,
      isInitialized: state.isInitialized,
      serverEpoch: state.serverEpoch,
    })));

  // 获取 actions（这些是稳定的引用）
  const _fullSync = useActiveOrdersStore((state) => state._fullSync);
  const _reset = useActiveOrdersStore((state) => state._reset);

  // 使用 useMemo 避免每次渲染都创建新数组
  const orders = useMemo(() => Array.from(ordersMap.values()), [ordersMap]);

  // 分类订单
  const activeOrders = useMemo(() => orders.filter((o) => o.status === 'ACTIVE'), [orders]);
  const completedOrders = useMemo(() => orders.filter((o) => o.status === 'COMPLETED'), [orders]);
  const voidedOrders = useMemo(() => orders.filter((o) => o.status === 'VOID'), [orders]);

  // 检测幽灵订单（有订单但没有 table_id）
  const ghostOrders = useMemo(() => activeOrders.filter((o) => !o.table_id && !o.is_retail), [activeOrders]);

  const toggleExpand = (orderId: string) => {
    setExpandedOrders((prev) => {
      const next = new Set(prev);
      if (next.has(orderId)) {
        next.delete(orderId);
      } else {
        next.add(orderId);
      }
      return next;
    });
  };

  // 强制全量同步
  const handleForceSync = async () => {
    setIsLoading(true);
    try {
      const response = await invokeApi<SyncResponse>('order_sync_since', {
        since_sequence: 0,
      });
      _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      toast.success(`同步完成: ${response.active_orders.length} 个订单`);
    } catch (error) {
      console.error('Force sync failed:', error);
      toast.error(`同步失败: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  // 重置 store
  const handleReset = () => {
    setConfirmDialog({ type: 'reset', isOpen: true });
  };

  const doReset = () => {
    setConfirmDialog({ type: 'reset', isOpen: false });
    _reset();
    toast.success('Store 已重置');
  };

  // 全部作废
  const handleVoidAll = () => {
    if (activeOrders.length === 0) {
      toast.error('没有激活订单可作废');
      return;
    }
    setConfirmDialog({ type: 'voidAll', isOpen: true });
  };

  const doVoidAll = async () => {
    setConfirmDialog({ type: 'voidAll', isOpen: false });
    setIsVoiding(true);
    let successCount = 0;
    let failCount = 0;

    for (const order of activeOrders) {
      try {
        const response = await voidOrder(order.order_id, 'Debug: Batch void');
        if (response.success) {
          successCount++;
        } else {
          failCount++;
        }
      } catch {
        failCount++;
      }
    }

    setIsVoiding(false);
    toast.success(`作废完成: 成功 ${successCount}, 失败 ${failCount}`);
  };

  const formatTime = (timestamp: number | null) => {
    if (!timestamp) return '-';
    return new Date(timestamp).toLocaleString('zh-CN');
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'ACTIVE':
        return 'text-green-600 bg-green-100';
      case 'COMPLETED':
        return 'text-blue-600 bg-blue-100';
      case 'VOIDED':
        return 'text-red-600 bg-red-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  };

  const renderOrderCard = (order: OrderSnapshot) => {
    const isExpanded = expandedOrders.has(order.order_id);
    const timeline = timelines.get(order.order_id) || [];
    const isGhost = !order.table_id && !order.is_retail && order.status === 'ACTIVE';

    return (
      <div
        key={order.order_id}
        className={`border rounded-lg overflow-hidden ${
          isGhost ? 'border-red-300 bg-red-50' : 'border-gray-200 bg-white'
        }`}
      >
        {/* Header */}
        <div
          className="p-4 cursor-pointer hover:bg-gray-50 flex items-center justify-between"
          onClick={() => toggleExpand(order.order_id)}
        >
          <div className="flex items-center gap-3">
            {isExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}

            {isGhost && <AlertTriangle className="text-red-500" size={18} />}

            <div>
              <div className="flex items-center gap-2">
                <span className="font-mono text-sm font-medium">{order.order_id}</span>
                <span
                  className={`px-2 py-0.5 rounded text-xs font-medium ${getStatusColor(
                    order.status
                  )}`}
                >
                  {order.status}
                </span>
                {order.is_retail && (
                  <span className="px-2 py-0.5 rounded text-xs font-medium bg-purple-100 text-purple-600">
                    零售
                  </span>
                )}
              </div>
              <div className="text-sm text-gray-500 mt-1 flex items-center gap-4">
                <span className="flex items-center gap-1">
                  <MapPin size={12} />
                  {order.table_name || order.table_id || '无桌台'}
                  {order.zone_name && ` (${order.zone_name})`}
                </span>
                <span className="flex items-center gap-1">
                  <ShoppingCart size={12} />
                  {order.items.length} 项
                </span>
                <span className="flex items-center gap-1">
                  <DollarSign size={12} />
                  ¥{order.total.toFixed(2)}
                </span>
              </div>
            </div>
          </div>

          <div className="text-right text-sm text-gray-500">
            <div>Seq: {order.last_sequence}</div>
            <div>{formatTime(order.created_at)}</div>
          </div>
        </div>

        {/* Expanded Content */}
        {isExpanded && (
          <div className="border-t border-gray-200 p-4 bg-gray-50 space-y-4">
            {/* 基本信息 */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div>
                <div className="text-gray-500">Table ID</div>
                <div className="font-mono">{order.table_id || <span className="text-red-500">null</span>}</div>
              </div>
              <div>
                <div className="text-gray-500">Zone ID</div>
                <div className="font-mono">{order.zone_id || 'null'}</div>
              </div>
              <div>
                <div className="text-gray-500">Guest Count</div>
                <div>{order.guest_count}</div>
              </div>
              <div>
                <div className="text-gray-500">Receipt #</div>
                <div className="font-mono">{order.receipt_number || 'null'}</div>
              </div>
            </div>

            {/* 金额信息 */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div>
                <div className="text-gray-500">Original Total</div>
                <div>¥{order.original_total.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-gray-500">Subtotal</div>
                <div>¥{order.subtotal.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-gray-500">Discount</div>
                <div className="text-red-600">-¥{order.total_discount.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-gray-500">Total</div>
                <div className="font-bold">¥{order.total.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-gray-500">Paid</div>
                <div className="text-green-600">¥{order.paid_amount.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-gray-500">Remaining</div>
                <div className={order.remaining_amount > 0 ? 'text-orange-600' : ''}>
                  ¥{order.remaining_amount.toFixed(2)}
                </div>
              </div>
            </div>

            {/* 时间信息 */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div>
                <div className="text-gray-500">Start Time</div>
                <div>{formatTime(order.start_time)}</div>
              </div>
              <div>
                <div className="text-gray-500">End Time</div>
                <div>{formatTime(order.end_time)}</div>
              </div>
              <div>
                <div className="text-gray-500">Created At</div>
                <div>{formatTime(order.created_at)}</div>
              </div>
              <div>
                <div className="text-gray-500">Updated At</div>
                <div>{formatTime(order.updated_at)}</div>
              </div>
            </div>

            {/* 商品列表 */}
            {order.items.length > 0 && (
              <div>
                <div className="text-sm font-medium text-gray-700 mb-2">商品 ({order.items.length})</div>
                <div className="bg-white rounded border border-gray-200 divide-y divide-gray-100">
                  {order.items.map((item, idx) => (
                    <div key={idx} className="p-2 text-sm flex justify-between">
                      <div>
                        <span className="font-medium">{item.name}</span>
                        <span className="text-gray-500 ml-2">x{item.quantity}</span>
                        {item.selected_options && item.selected_options.length > 0 && (
                          <span className="text-gray-400 ml-2 text-xs">
                            [{item.selected_options.map(o => o.option_name).join(', ')}]
                          </span>
                        )}
                      </div>
                      <div>¥{(item.price * item.quantity).toFixed(2)}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 事件时间线 */}
            {timeline.length > 0 && (
              <div>
                <div className="text-sm font-medium text-gray-700 mb-2">
                  事件时间线 ({timeline.length})
                </div>
                <div className="bg-white rounded border border-gray-200 max-h-48 overflow-y-auto">
                  {timeline.map((event, idx) => (
                    <div
                      key={idx}
                      className="p-2 text-xs border-b border-gray-100 last:border-0 font-mono"
                    >
                      <div className="flex justify-between">
                        <span className="text-blue-600">{event.event_type}</span>
                        <span className="text-gray-400">seq: {event.sequence}</span>
                      </div>
                      <div className="text-gray-500">{formatTime(event.timestamp)}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Raw JSON */}
            <details className="text-xs">
              <summary className="cursor-pointer text-gray-500 hover:text-gray-700">
                Raw JSON
              </summary>
              <pre className="mt-2 p-2 bg-gray-800 text-gray-100 rounded overflow-x-auto">
                {JSON.stringify(order, null, 2)}
              </pre>
            </details>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="h-screen overflow-y-auto bg-gray-100 p-4">
      {/* Header */}
      <div className="bg-white rounded-lg shadow-sm p-4 mb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate(-1)}
              className="p-2 hover:bg-gray-100 rounded-lg"
            >
              <ArrowLeft size={20} />
            </button>
            <Bug className="text-orange-500" size={24} />
            <div>
              <h1 className="text-xl font-bold">订单调试</h1>
              <p className="text-sm text-gray-500">查看所有订单状态，排查幽灵订单</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleForceSync}
              disabled={isLoading}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
              强制同步
            </button>
            <button
              onClick={handleVoidAll}
              disabled={isVoiding || activeOrders.length === 0}
              className="flex items-center gap-2 px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:opacity-50"
            >
              <AlertTriangle size={16} className={isVoiding ? 'animate-pulse' : ''} />
              {isVoiding ? '作废中...' : `全部作废 (${activeOrders.length})`}
            </button>
            <button
              onClick={handleReset}
              className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
            >
              <Trash2 size={16} />
              重置 Store
            </button>
          </div>
        </div>
      </div>

      {/* Store 状态 */}
      <div className="bg-white rounded-lg shadow-sm p-4 mb-4">
        <h2 className="font-medium mb-3">Store 状态</h2>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
          <div className="p-3 bg-gray-50 rounded">
            <div className="text-gray-500">连接状态</div>
            <div
              className={`font-medium ${
                connectionState === 'connected'
                  ? 'text-green-600'
                  : connectionState === 'syncing'
                  ? 'text-yellow-600'
                  : 'text-red-600'
              }`}
            >
              {connectionState}
            </div>
          </div>
          <div className="p-3 bg-gray-50 rounded">
            <div className="text-gray-500">已初始化</div>
            <div className={isInitialized ? 'text-green-600' : 'text-red-600'}>
              {isInitialized ? 'Yes' : 'No'}
            </div>
          </div>
          <div className="p-3 bg-gray-50 rounded">
            <div className="text-gray-500">Last Sequence</div>
            <div className="font-mono">{lastSequence}</div>
          </div>
          <div className="p-3 bg-gray-50 rounded">
            <div className="text-gray-500">Server Epoch</div>
            <div className="font-mono text-xs truncate">{serverEpoch || 'null'}</div>
          </div>
          <div className="p-3 bg-gray-50 rounded">
            <div className="text-gray-500">订单总数</div>
            <div>{orders.length}</div>
          </div>
        </div>
      </div>

      {/* 统计 */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        <div className="bg-white rounded-lg shadow-sm p-4">
          <div className="flex items-center gap-2 text-green-600">
            <CheckCircle size={20} />
            <span className="font-medium">激活订单</span>
          </div>
          <div className="text-2xl font-bold mt-2">{activeOrders.length}</div>
        </div>
        <div className="bg-white rounded-lg shadow-sm p-4">
          <div className="flex items-center gap-2 text-blue-600">
            <Clock size={20} />
            <span className="font-medium">已完成</span>
          </div>
          <div className="text-2xl font-bold mt-2">{completedOrders.length}</div>
        </div>
        <div className="bg-white rounded-lg shadow-sm p-4">
          <div className="flex items-center gap-2 text-gray-600">
            <Hash size={20} />
            <span className="font-medium">已作废</span>
          </div>
          <div className="text-2xl font-bold mt-2">{voidedOrders.length}</div>
        </div>
        <div className={`bg-white rounded-lg shadow-sm p-4 ${ghostOrders.length > 0 ? 'ring-2 ring-red-500' : ''}`}>
          <div className="flex items-center gap-2 text-red-600">
            <AlertTriangle size={20} />
            <span className="font-medium">幽灵订单</span>
          </div>
          <div className="text-2xl font-bold mt-2">{ghostOrders.length}</div>
        </div>
      </div>

      {/* 幽灵订单警告 */}
      {ghostOrders.length > 0 && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
          <div className="flex items-center gap-2 text-red-700 font-medium">
            <AlertTriangle size={20} />
            检测到 {ghostOrders.length} 个幽灵订单
          </div>
          <p className="text-red-600 text-sm mt-1">
            这些订单没有关联的桌台 (table_id 为 null)，但状态为 ACTIVE。
            可能是数据同步问题或订单创建时的 bug。
          </p>
        </div>
      )}

      {/* 订单列表 */}
      <div className="space-y-2">
        {/* 激活订单 */}
        {activeOrders.length > 0 && (
          <div>
            <h3 className="font-medium text-gray-700 mb-2">激活订单 ({activeOrders.length})</h3>
            <div className="space-y-2">
              {activeOrders.map(renderOrderCard)}
            </div>
          </div>
        )}

        {/* 已完成订单 */}
        {completedOrders.length > 0 && (
          <div className="mt-6">
            <h3 className="font-medium text-gray-700 mb-2">已完成 ({completedOrders.length})</h3>
            <div className="space-y-2">
              {completedOrders.map(renderOrderCard)}
            </div>
          </div>
        )}

        {/* 已作废订单 */}
        {voidedOrders.length > 0 && (
          <div className="mt-6">
            <h3 className="font-medium text-gray-700 mb-2">已作废 ({voidedOrders.length})</h3>
            <div className="space-y-2">
              {voidedOrders.map(renderOrderCard)}
            </div>
          </div>
        )}

        {orders.length === 0 && (
          <div className="text-center py-12 text-gray-500">
            <Bug size={48} className="mx-auto mb-4 opacity-50" />
            <p>没有订单数据</p>
            <p className="text-sm mt-1">点击"强制同步"从服务器获取数据</p>
          </div>
        )}
      </div>

      {/* 确认对话框 */}
      <ConfirmDialog
        isOpen={confirmDialog.isOpen && confirmDialog.type === 'reset'}
        title="重置 Store"
        description="确定要重置订单 Store 吗？这将清除所有本地订单数据。"
        confirmText="确认重置"
        onConfirm={doReset}
        onCancel={() => setConfirmDialog({ ...confirmDialog, isOpen: false })}
        variant="danger"
      />

      <ConfirmDialog
        isOpen={confirmDialog.isOpen && confirmDialog.type === 'voidAll'}
        title="全部作废"
        description={`确定要作废全部 ${activeOrders.length} 个激活订单吗？此操作不可撤销！`}
        confirmText="确认作废"
        onConfirm={doVoidAll}
        onCancel={() => setConfirmDialog({ ...confirmDialog, isOpen: false })}
        variant="danger"
      />
    </div>
  );
};
