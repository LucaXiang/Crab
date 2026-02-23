/**
 * Live Orders WebSocket Store
 *
 * 连接 crab-cloud Console WS 端点，接收实时活跃订单推送。
 * 每个门店 (edge_server_id) 独立连接，进入 live 页面时创建，离开时销毁。
 */

import { writable, derived } from 'svelte/store';

const WS_BASE = 'wss://cloud.redcoral.app';

// 重连参数
const RECONNECT_MIN_MS = 1000;
const RECONNECT_MAX_MS = 30000;

// --- ConsoleMessage 协议类型 ---

interface LiveOrderSnapshot {
	edge_server_id: number;
	order_id: string;
	table_id?: number;
	table_name?: string;
	zone_id?: number;
	zone_name?: string;
	guest_count: number;
	is_retail: boolean;
	service_type?: string;
	queue_number?: number;
	status: string;
	items: CartItemSnapshot[];
	payments: PaymentRecord[];
	original_total: number;
	subtotal: number;
	total_discount: number;
	total_surcharge: number;
	tax: number;
	total: number;
	paid_amount: number;
	remaining_amount: number;
	comp_total_amount: number;
	order_manual_discount_amount: number;
	order_manual_surcharge_amount: number;
	order_rule_discount_amount: number;
	order_rule_surcharge_amount: number;
	receipt_number: string;
	note?: string;
	created_at: number;
	updated_at: number;
	start_time: number;
	operator_id?: number;
	operator_name?: string;
	member_id?: number;
	member_name?: string;
	marketing_group_name?: string;
	events?: OrderEvent[];
	[key: string]: unknown;
}

interface ItemOption {
	attribute_id: number;
	attribute_name: string;
	option_id: number;
	option_name: string;
	price_modifier?: number;
	quantity?: number;
}

interface SpecificationInfo {
	id: number;
	name: string;
	receipt_name?: string;
	price?: number;
	is_multi_spec?: boolean;
}

interface PaymentRecord {
	payment_id: string;
	method: string;
	amount: number;
	tendered?: number;
	change?: number;
	note?: string;
	timestamp: number;
	cancelled: boolean;
	cancel_reason?: string;
	split_type?: string;
	aa_shares?: number;
}

interface CartItemSnapshot {
	id: number;
	instance_id: string;
	name: string;
	price: number;
	original_price: number;
	quantity: number;
	unpaid_quantity: number;
	unit_price: number;
	line_total: number;
	tax: number;
	tax_rate: number;
	is_comped: boolean;
	selected_options?: ItemOption[];
	selected_specification?: SpecificationInfo;
	manual_discount_percent?: number;
	rule_discount_amount: number;
	rule_surcharge_amount: number;
	mg_discount_amount: number;
	note?: string;
	authorizer_name?: string;
	category_name?: string;
	[key: string]: unknown;
}

// --- OrderEvent 类型 (event sourcing) ---

interface OrderEvent {
	event_id: string;
	sequence: number;
	order_id: string;
	timestamp: number;
	client_timestamp?: number;
	operator_id: number;
	operator_name: string;
	command_id: string;
	event_type: string;
	payload: Record<string, unknown>;
}

type ConsoleMessage =
	| { type: 'Ready'; snapshots: LiveOrderSnapshot[]; online_edge_ids?: number[] }
	| { type: 'OrderUpdated'; snapshot: LiveOrderSnapshot }
	| { type: 'OrderRemoved'; order_id: string; edge_server_id: number }
	| {
			type: 'EdgeStatus';
			edge_server_id: number;
			online: boolean;
			cleared_order_ids?: string[];
	  };

type ConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected';

export type {
	LiveOrderSnapshot,
	CartItemSnapshot,
	PaymentRecord,
	ItemOption,
	SpecificationInfo,
	OrderEvent
};

export interface LiveOrdersStore {
	orders: Map<string, LiveOrderSnapshot>;
	edgeOnline: boolean;
	connectionState: ConnectionState;
}

export function createLiveOrdersConnection(token: string, storeId: number) {
	const { subscribe, set, update } = writable<LiveOrdersStore>({
		orders: new Map(),
		edgeOnline: false,
		connectionState: 'connecting'
	});

	let ws: WebSocket | null = null;
	let reconnectDelay = RECONNECT_MIN_MS;
	let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	let destroyed = false;

	function connect() {
		if (destroyed) return;

		update((s) => ({ ...s, connectionState: 'connecting' }));

		const url = `${WS_BASE}/api/tenant/live-orders/ws?token=${encodeURIComponent(token)}`;
		ws = new WebSocket(url);

		ws.onopen = () => {
			reconnectDelay = RECONNECT_MIN_MS;
			update((s) => ({ ...s, connectionState: 'connected' }));

			// 订阅当前门店
			ws?.send(
				JSON.stringify({
					type: 'Subscribe',
					edge_server_ids: [storeId]
				})
			);
		};

		ws.onmessage = (event) => {
			try {
				const msg: ConsoleMessage = JSON.parse(event.data);
				handleMessage(msg);
			} catch {
				// ignore malformed messages
			}
		};

		ws.onclose = () => {
			ws = null;
			if (!destroyed) {
				update((s) => ({ ...s, connectionState: 'reconnecting' }));
				scheduleReconnect();
			}
		};

		ws.onerror = () => {
			// onclose will fire after onerror
		};
	}

	function handleMessage(msg: ConsoleMessage) {
		switch (msg.type) {
			case 'Ready':
				update((s) => {
					const orders = new Map<string, LiveOrderSnapshot>();
					for (const snap of msg.snapshots) {
						if (snap.edge_server_id === storeId) {
							orders.set(snap.order_id, snap);
						}
					}
					const edgeOnline = msg.online_edge_ids?.includes(storeId) ?? false;
				return { ...s, orders, edgeOnline };
				});
				break;

			case 'OrderUpdated':
				if (msg.snapshot.edge_server_id === storeId) {
					update((s) => {
						const orders = new Map(s.orders);
						orders.set(msg.snapshot.order_id, msg.snapshot);
						return { ...s, orders };
					});
				}
				break;

			case 'OrderRemoved':
				if (msg.edge_server_id === storeId) {
					update((s) => {
						const orders = new Map(s.orders);
						orders.delete(msg.order_id);
						return { ...s, orders };
					});
				}
				break;

			case 'EdgeStatus':
				if (msg.edge_server_id === storeId) {
					update((s) => {
						const orders = new Map(s.orders);
						if (!msg.online && msg.cleared_order_ids) {
							for (const id of msg.cleared_order_ids) {
								orders.delete(id);
							}
						}
						return { ...s, orders, edgeOnline: msg.online };
					});
				}
				break;
		}
	}

	function scheduleReconnect() {
		if (destroyed) return;
		reconnectTimer = setTimeout(() => {
			reconnectTimer = null;
			reconnectDelay = Math.min(reconnectDelay * 2, RECONNECT_MAX_MS);
			connect();
		}, reconnectDelay);
	}

	function destroy() {
		destroyed = true;
		if (reconnectTimer) {
			clearTimeout(reconnectTimer);
			reconnectTimer = null;
		}
		if (ws) {
			ws.onclose = null;
			ws.close();
			ws = null;
		}
		set({
			orders: new Map(),
			edgeOnline: false,
			connectionState: 'disconnected'
		});
	}

	// 启动连接
	connect();

	// 衍生 store：按更新时间排序的订单数组
	const sortedOrders = derived({ subscribe }, ($state) =>
		Array.from($state.orders.values()).sort((a, b) => b.updated_at - a.updated_at)
	);

	return {
		subscribe,
		sortedOrders,
		destroy
	};
}
