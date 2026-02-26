import { useCallback, useEffect, useRef, useState } from 'react';
import type { LiveOrderSnapshot, ConsoleMessage, ConnectionState } from '@/core/types/live';

const WS_BASE = 'wss://cloud.redcoral.app';
const RECONNECT_MIN_MS = 1000;
const RECONNECT_MAX_MS = 30000;

export interface LiveOrdersState {
  orders: Map<string, LiveOrderSnapshot>;
  edgeOnline: boolean;
  connectionState: ConnectionState;
}

export function useLiveOrders(token: string | null, storeId: number) {
  const [state, setState] = useState<LiveOrdersState>({
    orders: new Map(),
    edgeOnline: false,
    connectionState: 'connecting',
  });

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectDelayRef = useRef(RECONNECT_MIN_MS);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const destroyedRef = useRef(false);

  const handleMessage = useCallback((msg: ConsoleMessage) => {
    switch (msg.type) {
      case 'Ready':
        setState(s => {
          const orders = new Map<string, LiveOrderSnapshot>();
          for (const snap of msg.snapshots) {
            if (snap.store_id === storeId) {
              orders.set(snap.order_id, snap);
            }
          }
          const edgeOnline = msg.online_edge_ids?.includes(storeId) ?? false;
          return { ...s, orders, edgeOnline };
        });
        break;
      case 'OrderUpdated':
        if (msg.snapshot.store_id === storeId) {
          setState(s => {
            const orders = new Map(s.orders);
            orders.set(msg.snapshot.order_id, msg.snapshot);
            return { ...s, orders };
          });
        }
        break;
      case 'OrderRemoved':
        if (msg.store_id === storeId) {
          setState(s => {
            const orders = new Map(s.orders);
            orders.delete(msg.order_id);
            return { ...s, orders };
          });
        }
        break;
      case 'EdgeStatus':
        if (msg.store_id === storeId) {
          setState(s => {
            const orders = new Map(s.orders);
            if (!msg.online && msg.cleared_order_ids) {
              for (const id of msg.cleared_order_ids) orders.delete(id);
            }
            return { ...s, orders, edgeOnline: msg.online };
          });
        }
        break;
    }
  }, [storeId]);

  useEffect(() => {
    if (!token) return;
    destroyedRef.current = false;
    reconnectDelayRef.current = RECONNECT_MIN_MS;

    function connect() {
      if (destroyedRef.current) return;
      setState(s => ({ ...s, connectionState: 'connecting' }));

      const url = `${WS_BASE}/api/tenant/live-orders/ws?token=${encodeURIComponent(token!)}`;
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        reconnectDelayRef.current = RECONNECT_MIN_MS;
        setState(s => ({ ...s, connectionState: 'connected' }));
        ws.send(JSON.stringify({ type: 'Subscribe', store_ids: [storeId] }));
      };

      ws.onmessage = (event) => {
        try {
          handleMessage(JSON.parse(event.data) as ConsoleMessage);
        } catch { /* ignore */ }
      };

      ws.onclose = () => {
        wsRef.current = null;
        if (!destroyedRef.current) {
          setState(s => ({ ...s, connectionState: 'reconnecting' }));
          reconnectTimerRef.current = setTimeout(() => {
            reconnectTimerRef.current = null;
            reconnectDelayRef.current = Math.min(reconnectDelayRef.current * 2, RECONNECT_MAX_MS);
            connect();
          }, reconnectDelayRef.current);
        }
      };
    }

    connect();

    return () => {
      destroyedRef.current = true;
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
      setState({ orders: new Map(), edgeOnline: false, connectionState: 'disconnected' });
    };
  }, [token, storeId, handleMessage]);

  const sortedOrders = Array.from(state.orders.values()).sort((a, b) => b.updated_at - a.updated_at);

  return { ...state, sortedOrders };
}
