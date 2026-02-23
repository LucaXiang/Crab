<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		Radio, Wifi, WifiOff, Users, Clock, Receipt,
		X, CreditCard, Tag, Gift, StickyNote, User, CalendarClock, Hash,
		Plus, Minus, Pencil, Trash2, Ban, ArrowRightLeft, Merge,
		DollarSign, CircleDot, History, ChevronDown, ChevronUp
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, ApiError, type StoreDetail } from '$lib/api';
	import { formatCurrency, formatDateTime, timeAgo } from '$lib/format';
	import {
		createLiveOrdersConnection,
		type LiveOrdersStore,
		type LiveOrderSnapshot,
		type OrderEvent,
	} from '$lib/stores/liveOrders';
	const storeId = Number(page.params.id);

	let store = $state<StoreDetail | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	let connection: ReturnType<typeof createLiveOrdersConnection> | null = null;
	let liveState = $state<LiveOrdersStore>({
		orders: new Map(),
		edgeOnline: true,
		connectionState: 'connecting'
	});
	let sortedOrders = $state<LiveOrderSnapshot[]>([]);
	let selectedOrderId = $state<string | null>(null);

	// Derived: selected order from the live map (auto-updates on WS push)
	let selectedOrder = $derived(
		selectedOrderId ? liveState.orders.get(selectedOrderId) ?? null : null
	);

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			const stores = await getStores(token);
			store = stores.find((s) => s.id === storeId) ?? null;
			if (!store) { error = 'Store not found'; return; }

			connection = createLiveOrdersConnection(token, storeId);
			connection.subscribe((s) => { liveState = s; });
			connection.sortedOrders.subscribe((orders) => { sortedOrders = orders; });
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});

	onDestroy(() => {
		connection?.destroy();
	});

	function selectOrder(orderId: string) {
		selectedOrderId = selectedOrderId === orderId ? null : orderId;
	}

	function statusBadge(status: string): string {
		switch (status) {
			case 'ACTIVE': return 'bg-blue-100 text-blue-700';
			case 'COMPLETED': return 'bg-green-100 text-green-700';
			case 'VOID': return 'bg-red-100 text-red-700';
			default: return 'bg-slate-100 text-slate-600';
		}
	}

	function statusLabel(status: string): string {
		switch (status) {
			case 'ACTIVE': return 'Active';
			case 'COMPLETED': return 'Completed';
			case 'VOID': return 'Void';
			case 'MERGED': return 'Merged';
			default: return status;
		}
	}

	function connectionColor(state: string): string {
		switch (state) {
			case 'connected': return 'text-green-500';
			case 'connecting': case 'reconnecting': return 'text-amber-500';
			default: return 'text-slate-400';
		}
	}

	function serviceTypeLabel(type: string | undefined): string | null {
		if (!type) return null;
		if (type === 'DINE_IN') return $t('live.dine_in');
		if (type === 'TAKEOUT') return $t('live.takeout');
		return type;
	}

	function paymentMethodLabel(method: string): string {
		switch (method) {
			case 'cash': return 'Cash';
			case 'card': return 'Card';
			case 'transfer': return 'Transfer';
			default: return method;
		}
	}

	function orderTitle(order: LiveOrderSnapshot): string {
		if (order.queue_number) return `#${order.queue_number}`;
		if (order.table_name) return order.table_name;
		return order.order_id.slice(0, 8);
	}

	function activeItems(order: LiveOrderSnapshot) {
		return order.items;
	}

	let showEvents = $state(true);

	function eventDotColor(eventType: string): string {
		switch (eventType) {
			case 'TABLE_OPENED': return 'bg-blue-500';
			case 'ORDER_COMPLETED': return 'bg-green-500';
			case 'ORDER_VOIDED': return 'bg-red-500';
			case 'ITEMS_ADDED': return 'bg-emerald-500';
			case 'ITEM_MODIFIED': return 'bg-amber-500';
			case 'ITEM_REMOVED': return 'bg-red-400';
			case 'ITEM_COMPED': case 'ITEM_UNCOMPED': return 'bg-emerald-400';
			case 'PAYMENT_ADDED': return 'bg-green-400';
			case 'PAYMENT_CANCELLED': return 'bg-red-400';
			case 'ORDER_MOVED': case 'ORDER_MERGED': case 'TABLE_REASSIGNED': return 'bg-indigo-400';
			case 'MEMBER_LINKED': case 'MEMBER_UNLINKED': return 'bg-purple-400';
			case 'ORDER_DISCOUNT_APPLIED': case 'ORDER_SURCHARGE_APPLIED': return 'bg-orange-400';
			case 'STAMP_REDEEMED': case 'STAMP_REDEMPTION_CANCELLED': return 'bg-pink-400';
			default: return 'bg-slate-400';
		}
	}

	function eventLabel(eventType: string): string {
		const labels: Record<string, string> = {
			TABLE_OPENED: $t('live.ev.table_opened'),
			ORDER_COMPLETED: $t('live.ev.order_completed'),
			ORDER_VOIDED: $t('live.ev.order_voided'),
			ITEMS_ADDED: $t('live.ev.items_added'),
			ITEM_MODIFIED: $t('live.ev.item_modified'),
			ITEM_REMOVED: $t('live.ev.item_removed'),
			ITEM_COMPED: $t('live.ev.item_comped'),
			ITEM_UNCOMPED: $t('live.ev.item_uncomped'),
			PAYMENT_ADDED: $t('live.ev.payment_added'),
			PAYMENT_CANCELLED: $t('live.ev.payment_cancelled'),
			ITEM_SPLIT: $t('live.ev.item_split'),
			AMOUNT_SPLIT: $t('live.ev.amount_split'),
			AA_SPLIT_STARTED: $t('live.ev.aa_split_started'),
			AA_SPLIT_PAID: $t('live.ev.aa_split_paid'),
			AA_SPLIT_CANCELLED: $t('live.ev.aa_split_cancelled'),
			ORDER_MOVED: $t('live.ev.order_moved'),
			ORDER_MERGED: $t('live.ev.order_merged'),
			TABLE_REASSIGNED: $t('live.ev.table_reassigned'),
			ORDER_INFO_UPDATED: $t('live.ev.order_info_updated'),
			RULE_SKIP_TOGGLED: $t('live.ev.rule_skip_toggled'),
			ORDER_DISCOUNT_APPLIED: $t('live.ev.order_discount_applied'),
			ORDER_SURCHARGE_APPLIED: $t('live.ev.order_surcharge_applied'),
			ORDER_NOTE_ADDED: $t('live.ev.order_note_added'),
			MEMBER_LINKED: $t('live.ev.member_linked'),
			MEMBER_UNLINKED: $t('live.ev.member_unlinked'),
			STAMP_REDEEMED: $t('live.ev.stamp_redeemed'),
			STAMP_REDEMPTION_CANCELLED: $t('live.ev.stamp_redemption_cancelled'),
			ORDER_MOVED_OUT: $t('live.ev.order_moved_out'),
			ORDER_MERGED_OUT: $t('live.ev.order_merged_out'),
		};
		return labels[eventType] ?? eventType;
	}

	function eventDetail(event: OrderEvent): string | null {
		const p = event.payload;
		const type = p.type as string;
		switch (type) {
			case 'ITEMS_ADDED': {
				const items = p.items as Array<{name: string; quantity: number}>;
				if (!items) return null;
				return items.map(i => `${i.quantity}x ${i.name}`).join(', ');
			}
			case 'ITEM_REMOVED':
				return `${p.item_name}${p.quantity ? ` x${p.quantity}` : ''}${p.reason ? ` — ${p.reason}` : ''}`;
			case 'ITEM_COMPED':
				return `${p.item_name} x${p.quantity} — ${p.reason}`;
			case 'ITEM_UNCOMPED':
				return `${p.item_name}`;
			case 'ITEM_MODIFIED':
				return `${p.operation} — ${(p.source as {name?: string})?.name ?? ''}`;
			case 'PAYMENT_ADDED':
				return `${p.method} ${formatCurrency(p.amount as number)}`;
			case 'PAYMENT_CANCELLED':
				return `${p.method} ${formatCurrency(p.amount as number)}${p.reason ? ` — ${p.reason}` : ''}`;
			case 'ORDER_MOVED':
				return `${p.source_table_name} → ${p.target_table_name}`;
			case 'ORDER_MERGED':
				return `← ${p.source_table_name}`;
			case 'TABLE_REASSIGNED':
				return `${p.source_table_name} → ${p.target_table_name}`;
			case 'ORDER_DISCOUNT_APPLIED':
				if (p.discount_percent) return `-${p.discount_percent}% (${formatCurrency(p.discount as number)})`;
				if (p.discount_fixed) return `-${formatCurrency(p.discount_fixed as number)}`;
				return null;
			case 'ORDER_SURCHARGE_APPLIED':
				if (p.surcharge_percent) return `+${p.surcharge_percent}%`;
				if (p.surcharge_amount) return `+${formatCurrency(p.surcharge_amount as number)}`;
				return null;
			case 'ORDER_NOTE_ADDED':
				return p.note as string;
			case 'MEMBER_LINKED':
				return `${p.member_name} (${p.marketing_group_name})`;
			case 'MEMBER_UNLINKED':
				return `${p.previous_member_name}`;
			case 'STAMP_REDEEMED':
				return `${p.product_name} — ${p.stamp_activity_name}`;
			case 'ORDER_VOIDED':
				return p.note ? `${p.note}` : null;
			case 'ORDER_COMPLETED':
				return formatCurrency(p.final_total as number);
			case 'TABLE_OPENED':
				return p.table_name ? `${p.table_name}` : (p.queue_number ? `#${p.queue_number}` : null);
			case 'ITEM_SPLIT':
			case 'AMOUNT_SPLIT':
				return `${p.payment_method} ${formatCurrency(p.split_amount as number)}`;
			case 'AA_SPLIT_STARTED':
				return `${p.total_shares} shares · ${formatCurrency(p.per_share_amount as number)}/share`;
			case 'AA_SPLIT_PAID':
				return `${p.shares} shares · ${p.payment_method} ${formatCurrency(p.amount as number)} (${p.progress_paid}/${p.progress_total})`;
			case 'RULE_SKIP_TOGGLED':
				return `${p.rule_name}: ${p.skipped ? 'skip' : 'apply'}`;
			default:
				return null;
		}
	}
</script>

<svelte:head>
	<title>{$t('live.title')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
	<div class="flex items-center justify-end">
		<!-- Connection status -->
		<div class="flex items-center gap-2 text-sm">
				{#if !liveState.edgeOnline}
					<WifiOff class="w-4 h-4 text-red-500" />
					<span class="text-red-600 font-medium">{$t('live.edge_offline')}</span>
				{:else}
					<Wifi class="w-4 h-4 {connectionColor(liveState.connectionState)}" />
					<span class="text-slate-500">{$t(`live.ws_${liveState.connectionState}`)}</span>
				{/if}
			</div>
		</div>

		<div class="flex items-center gap-3">
			<div class="w-10 h-10 bg-coral-100 rounded-xl flex items-center justify-center">
				<Radio class="w-5 h-5 text-coral-600" />
			</div>
			<div>
				<h1 class="font-heading text-xl font-bold text-slate-900">{$t('live.title')}</h1>
				<p class="text-sm text-slate-500">
					{store?.name ?? `Store #${storeId}`} — {sortedOrders.length} {$t('live.active_orders')}
				</p>
			</div>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if sortedOrders.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-12 text-center">
				<Receipt class="w-12 h-12 text-slate-300 mx-auto mb-3" />
				<p class="text-slate-500">{$t('live.empty')}</p>
			</div>
		{:else}
			<div class="flex gap-6">
				<!-- Order cards grid -->
				<div class="flex-1 min-w-0">
					<div class="grid grid-cols-1 md:grid-cols-2 {selectedOrderId ? '' : 'xl:grid-cols-3'} gap-4">
						{#each sortedOrders as order (order.order_id)}
							<button
								type="button"
								class="bg-white rounded-xl border p-4 transition-all text-left w-full
									{selectedOrderId === order.order_id
										? 'border-coral-400 ring-2 ring-coral-100 shadow-md'
										: 'border-slate-200 hover:border-coral-200 hover:shadow-sm'}"
								onclick={() => selectOrder(order.order_id)}
							>
								<!-- Header -->
								<div class="flex items-center justify-between mb-3">
									<div class="flex items-center gap-2">
										<span class="text-lg font-bold text-slate-900">{orderTitle(order)}</span>
										<span class="px-2 py-0.5 rounded-full text-xs font-medium {statusBadge(order.status)}">
											{statusLabel(order.status)}
										</span>
									</div>
									<span class="text-lg font-bold text-slate-900">{formatCurrency(order.total)}</span>
								</div>

								<!-- Meta -->
								<div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-slate-500 mb-3">
									{#if order.zone_name}
										<span>{order.zone_name}</span>
									{/if}
									{#if order.guest_count > 0}
										<span class="inline-flex items-center gap-1">
											<Users class="w-3 h-3" />
											{order.guest_count}
										</span>
									{/if}
									{#if order.operator_name}
										<span>{order.operator_name}</span>
									{/if}
									<span class="inline-flex items-center gap-1">
										<Clock class="w-3 h-3" />
										{timeAgo(order.created_at)}
									</span>
									{#if order.is_retail}
										<span class="text-coral-600 font-medium">{$t('live.retail')}</span>
									{/if}
								</div>

								<!-- Items preview -->
								<div class="border-t border-slate-100 pt-2 space-y-1">
									{#each activeItems(order).slice(0, 4) as item}
										<div class="flex items-center justify-between text-sm">
											<span class="text-slate-700 truncate flex-1">
												<span class="font-medium">{item.quantity}x</span> {item.name}
												{#if item.is_comped}
													<span class="text-emerald-600 text-xs ml-1">({$t('live.comped')})</span>
												{/if}
											</span>
											<span class="text-slate-500 ml-2 shrink-0">{formatCurrency(item.line_total)}</span>
										</div>
									{/each}
									{#if activeItems(order).length > 4}
										<p class="text-xs text-slate-400">
											+{activeItems(order).length - 4} {$t('live.more_items')}
										</p>
									{/if}
								</div>

								<!-- Payment bar -->
								{#if order.paid_amount > 0}
									<div class="mt-2 pt-2 border-t border-slate-100 flex justify-between text-xs">
										<span class="text-slate-500">{$t('live.paid')}</span>
										<span class="text-green-600 font-medium">{formatCurrency(order.paid_amount)}</span>
									</div>
								{/if}
								{#if order.remaining_amount > 0}
									<div class="flex justify-between text-xs {order.paid_amount > 0 ? '' : 'mt-2 pt-2 border-t border-slate-100'}">
										<span class="text-slate-500">{$t('live.remaining')}</span>
										<span class="text-amber-600 font-medium">{formatCurrency(order.remaining_amount)}</span>
									</div>
								{/if}
							</button>
						{/each}
					</div>
				</div>

				<!-- Detail panel (slide-in) -->
				{#if selectedOrder}
					<div class="w-[420px] shrink-0 hidden lg:block">
						<div class="bg-white rounded-2xl border border-slate-200 shadow-lg sticky top-6 overflow-hidden">
							<!-- Detail header -->
							<div class="px-5 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50">
								<div>
									<h2 class="text-lg font-bold text-slate-900">{orderTitle(selectedOrder)}</h2>
									<div class="flex items-center gap-2 mt-0.5">
										<span class="px-2 py-0.5 rounded-full text-xs font-medium {statusBadge(selectedOrder.status)}">
											{statusLabel(selectedOrder.status)}
										</span>
										{#if selectedOrder.receipt_number}
											<span class="text-xs text-slate-400">#{selectedOrder.receipt_number}</span>
										{/if}
									</div>
								</div>
								<button
									type="button"
									class="p-1.5 hover:bg-slate-200 rounded-lg transition-colors"
									onclick={() => selectedOrderId = null}
								>
									<X class="w-4 h-4 text-slate-500" />
								</button>
							</div>

							<div class="max-h-[calc(100vh-200px)] overflow-y-auto">
								<!-- Order info -->
								<div class="px-5 py-3 space-y-2 text-sm border-b border-slate-100">
									{#if selectedOrder.zone_name || selectedOrder.table_name}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('orders.zone')} / {$t('orders.table')}</span>
											<span class="text-slate-900 font-medium">
												{[selectedOrder.zone_name, selectedOrder.table_name].filter(Boolean).join(' · ')}
											</span>
										</div>
									{/if}
									{#if selectedOrder.guest_count > 0}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('orders.guests')}</span>
											<span class="text-slate-900">{selectedOrder.guest_count}</span>
										</div>
									{/if}
									{#if selectedOrder.operator_name}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('live.operator')}</span>
											<span class="text-slate-900">{selectedOrder.operator_name}</span>
										</div>
									{/if}
									{#if selectedOrder.member_name}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('live.member')}</span>
											<span class="text-slate-900">{selectedOrder.member_name}</span>
										</div>
									{/if}
									{#if selectedOrder.marketing_group_name}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('live.mg')}</span>
											<span class="text-slate-900">{selectedOrder.marketing_group_name}</span>
										</div>
									{/if}
									{#if serviceTypeLabel(selectedOrder.service_type)}
										<div class="flex justify-between">
											<span class="text-slate-500">Service</span>
											<span class="text-slate-900">{serviceTypeLabel(selectedOrder.service_type)}</span>
										</div>
									{/if}
									<div class="flex justify-between">
										<span class="text-slate-500">{$t('live.opened_at')}</span>
										<span class="text-slate-900 text-xs">{formatDateTime(selectedOrder.start_time)}</span>
									</div>
									{#if selectedOrder.note}
										<div class="mt-1 p-2 bg-amber-50 border border-amber-100 rounded-lg text-xs text-amber-800">
											<span class="font-medium">{$t('live.note')}:</span> {selectedOrder.note}
										</div>
									{/if}
								</div>

								<!-- Items -->
								<div class="px-5 py-3 border-b border-slate-100">
									<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{$t('live.items')}</h3>
									<div class="space-y-2">
										{#each activeItems(selectedOrder) as item}
											<div class="flex items-start justify-between text-sm gap-2">
												<div class="flex-1 min-w-0">
													<div class="flex items-center gap-1.5">
														<span class="font-medium text-slate-900">{item.quantity}x</span>
														<span class="text-slate-800 truncate">{item.name}</span>
														{#if item.is_comped}
															<span class="px-1.5 py-0.5 text-[10px] font-medium bg-emerald-100 text-emerald-700 rounded">
																{$t('live.comped')}
															</span>
														{/if}
													</div>
													<!-- Spec -->
													{#if item.selected_specification?.is_multi_spec}
														<span class="text-xs text-slate-500 ml-5">{item.selected_specification.name}</span>
													{/if}
													<!-- Options -->
													{#if item.selected_options && item.selected_options.length > 0}
														<div class="ml-5 text-xs text-slate-500">
															{#each item.selected_options as opt}
																<span class="inline-block mr-2">
																	{opt.option_name}{#if opt.quantity && opt.quantity > 1} x{opt.quantity}{/if}
																	{#if opt.price_modifier && opt.price_modifier !== 0}
																		<span class="text-slate-400">({opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)})</span>
																	{/if}
																</span>
															{/each}
														</div>
													{/if}
													<!-- Manual discount -->
													{#if item.manual_discount_percent}
														<span class="ml-5 text-xs text-orange-500">-{item.manual_discount_percent}%</span>
													{/if}
													<!-- Note -->
													{#if item.note}
														<p class="ml-5 text-xs text-amber-600 italic">{item.note}</p>
													{/if}
												</div>
												<div class="text-right shrink-0">
													<span class="text-slate-900 font-medium">{formatCurrency(item.line_total)}</span>
													{#if item.unit_price !== item.original_price}
														<div class="text-xs text-slate-400 line-through">{formatCurrency(item.original_price * item.quantity)}</div>
													{/if}
												</div>
											</div>
										{/each}
									</div>
								</div>

								<!-- Price breakdown -->
								<div class="px-5 py-3 border-b border-slate-100 space-y-1.5 text-sm">
									<div class="flex justify-between">
										<span class="text-slate-500">{$t('live.subtotal')}</span>
										<span class="text-slate-900">{formatCurrency(selectedOrder.subtotal)}</span>
									</div>
									{#if selectedOrder.total_discount > 0}
										<div class="flex justify-between">
											<span class="text-orange-500">{$t('live.discount')}</span>
											<span class="text-orange-500">-{formatCurrency(selectedOrder.total_discount)}</span>
										</div>
									{/if}
									{#if selectedOrder.total_surcharge > 0}
										<div class="flex justify-between">
											<span class="text-purple-500">{$t('live.surcharge')}</span>
											<span class="text-purple-500">+{formatCurrency(selectedOrder.total_surcharge)}</span>
										</div>
									{/if}
									{#if selectedOrder.comp_total_amount > 0}
										<div class="flex justify-between">
											<span class="text-emerald-600">{$t('live.comp')}</span>
											<span class="text-emerald-600">-{formatCurrency(selectedOrder.comp_total_amount)}</span>
										</div>
									{/if}
									{#if selectedOrder.tax > 0}
										<div class="flex justify-between">
											<span class="text-slate-500">{$t('live.tax')}</span>
											<span class="text-slate-900">{formatCurrency(selectedOrder.tax)}</span>
										</div>
									{/if}
									<div class="flex justify-between pt-1.5 border-t border-slate-100 font-bold text-base">
										<span class="text-slate-900">{$t('live.total')}</span>
										<span class="text-slate-900">{formatCurrency(selectedOrder.total)}</span>
									</div>
								</div>

								<!-- Payments -->
								<div class="px-5 py-3 border-b border-slate-100">
									<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{$t('live.payments')}</h3>
									{#if selectedOrder.payments.length === 0}
										<p class="text-sm text-slate-400 italic">{$t('live.no_payments')}</p>
									{:else}
										<div class="space-y-2">
											{#each selectedOrder.payments as payment}
												<div class="flex items-center justify-between text-sm {payment.cancelled ? 'opacity-50' : ''}">
													<div class="flex items-center gap-2">
														<CreditCard class="w-3.5 h-3.5 text-slate-400" />
														<span class="text-slate-700 capitalize">{paymentMethodLabel(payment.method)}</span>
														{#if payment.cancelled}
															<span class="px-1.5 py-0.5 text-[10px] bg-red-100 text-red-600 rounded font-medium">
																{$t('live.cancelled')}
															</span>
														{/if}
														{#if payment.split_type}
															<span class="text-[10px] text-slate-400">({payment.split_type})</span>
														{/if}
													</div>
													<span class="font-medium {payment.cancelled ? 'text-slate-400 line-through' : 'text-green-600'}">
														{formatCurrency(payment.amount)}
													</span>
												</div>
												{#if payment.tendered && payment.change}
													<div class="ml-6 text-xs text-slate-400">
														Tendered: {formatCurrency(payment.tendered)} · Change: {formatCurrency(payment.change)}
													</div>
												{/if}
											{/each}
										</div>

										<!-- Payment summary -->
										<div class="mt-3 pt-2 border-t border-slate-100 space-y-1">
											<div class="flex justify-between text-sm">
												<span class="text-slate-500">{$t('live.paid')}</span>
												<span class="text-green-600 font-medium">{formatCurrency(selectedOrder.paid_amount)}</span>
											</div>
											{#if selectedOrder.remaining_amount > 0}
												<div class="flex justify-between text-sm">
													<span class="text-slate-500">{$t('live.remaining')}</span>
													<span class="text-amber-600 font-medium">{formatCurrency(selectedOrder.remaining_amount)}</span>
												</div>
											{/if}
										</div>
									{/if}
								</div>

								<!-- Event Timeline -->
								{#if selectedOrder.events && selectedOrder.events.length > 0}
									<div class="px-5 py-3">
										<button
											type="button"
											class="flex items-center justify-between w-full text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2"
											onclick={() => showEvents = !showEvents}
										>
											<span class="flex items-center gap-1.5">
												<History class="w-3.5 h-3.5" />
												{$t('live.events')} ({selectedOrder.events.length})
											</span>
											{#if showEvents}
												<ChevronUp class="w-3.5 h-3.5" />
											{:else}
												<ChevronDown class="w-3.5 h-3.5" />
											{/if}
										</button>
										{#if showEvents}
											<div class="relative ml-2">
												<!-- Timeline line -->
												<div class="absolute left-[5px] top-2 bottom-2 w-px bg-slate-200"></div>
												<div class="space-y-3">
													{#each [...selectedOrder.events].reverse() as event}
														<div class="relative flex gap-3 pl-5">
															<!-- Dot -->
															<div class="absolute left-0 top-1.5 w-[11px] h-[11px] rounded-full border-2 border-white {eventDotColor(event.event_type)} shadow-sm"></div>
															<!-- Content -->
															<div class="flex-1 min-w-0">
																<div class="flex items-center justify-between gap-2">
																	<span class="text-xs font-medium text-slate-700 truncate">{eventLabel(event.event_type)}</span>
																	<span class="text-[10px] text-slate-400 shrink-0">{formatDateTime(event.timestamp)}</span>
																</div>
																{#if eventDetail(event)}
																	<p class="text-[11px] text-slate-500 mt-0.5 truncate">{eventDetail(event)}</p>
																{/if}
																<p class="text-[10px] text-slate-400 mt-0.5">{event.operator_name}</p>
															</div>
														</div>
													{/each}
												</div>
											</div>
										{/if}
									</div>
								{/if}
							</div>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Mobile detail modal (lg 以下) -->
	{#if selectedOrder}
		<div
			class="lg:hidden fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-end justify-center"
			onclick={() => selectedOrderId = null}
			onkeydown={(e) => e.key === 'Escape' && (selectedOrderId = null)}
			role="dialog"
			tabindex="-1"
		>
			<div
				class="bg-white rounded-t-2xl w-full max-h-[85vh] overflow-y-auto animate-slide-up"
				onclick={(e) => e.stopPropagation()}
				onkeydown={() => {}}
				role="document"
			>
				<!-- Mobile header -->
				<div class="px-5 py-4 border-b border-slate-100 flex items-center justify-between sticky top-0 bg-white z-10">
					<div>
						<h2 class="text-lg font-bold text-slate-900">{orderTitle(selectedOrder)}</h2>
						<div class="flex items-center gap-2 mt-0.5">
							<span class="px-2 py-0.5 rounded-full text-xs font-medium {statusBadge(selectedOrder.status)}">
								{statusLabel(selectedOrder.status)}
							</span>
							{#if selectedOrder.receipt_number}
								<span class="text-xs text-slate-400">#{selectedOrder.receipt_number}</span>
							{/if}
						</div>
					</div>
					<button
						type="button"
						class="p-2 hover:bg-slate-100 rounded-xl transition-colors"
						onclick={() => selectedOrderId = null}
					>
						<X class="w-5 h-5 text-slate-500" />
					</button>
				</div>

				<!-- Reuse same content structure -->
				<div class="px-5 py-3 space-y-2 text-sm border-b border-slate-100">
					{#if selectedOrder.zone_name || selectedOrder.table_name}
						<div class="flex justify-between">
							<span class="text-slate-500">{$t('orders.zone')} / {$t('orders.table')}</span>
							<span class="text-slate-900 font-medium">
								{[selectedOrder.zone_name, selectedOrder.table_name].filter(Boolean).join(' · ')}
							</span>
						</div>
					{/if}
					{#if selectedOrder.guest_count > 0}
						<div class="flex justify-between">
							<span class="text-slate-500">{$t('orders.guests')}</span>
							<span class="text-slate-900">{selectedOrder.guest_count}</span>
						</div>
					{/if}
					{#if selectedOrder.operator_name}
						<div class="flex justify-between">
							<span class="text-slate-500">{$t('live.operator')}</span>
							<span class="text-slate-900">{selectedOrder.operator_name}</span>
						</div>
					{/if}
					{#if selectedOrder.member_name}
						<div class="flex justify-between">
							<span class="text-slate-500">{$t('live.member')}</span>
							<span class="text-slate-900">{selectedOrder.member_name}</span>
						</div>
					{/if}
					<div class="flex justify-between">
						<span class="text-slate-500">{$t('live.opened_at')}</span>
						<span class="text-slate-900 text-xs">{formatDateTime(selectedOrder.start_time)}</span>
					</div>
					{#if selectedOrder.note}
						<div class="mt-1 p-2 bg-amber-50 border border-amber-100 rounded-lg text-xs text-amber-800">
							<span class="font-medium">{$t('live.note')}:</span> {selectedOrder.note}
						</div>
					{/if}
				</div>

				<!-- Items -->
				<div class="px-5 py-3 border-b border-slate-100">
					<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{$t('live.items')}</h3>
					<div class="space-y-2">
						{#each activeItems(selectedOrder) as item}
							<div class="flex items-start justify-between text-sm gap-2">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-1.5">
										<span class="font-medium text-slate-900">{item.quantity}x</span>
										<span class="text-slate-800 truncate">{item.name}</span>
										{#if item.is_comped}
											<span class="px-1.5 py-0.5 text-[10px] font-medium bg-emerald-100 text-emerald-700 rounded">
												{$t('live.comped')}
											</span>
										{/if}
									</div>
									{#if item.selected_specification?.is_multi_spec}
										<span class="text-xs text-slate-500 ml-5">{item.selected_specification.name}</span>
									{/if}
									{#if item.selected_options && item.selected_options.length > 0}
										<div class="ml-5 text-xs text-slate-500">
											{#each item.selected_options as opt}
												<span class="inline-block mr-2">
													{opt.option_name}{#if opt.quantity && opt.quantity > 1} x{opt.quantity}{/if}
													{#if opt.price_modifier && opt.price_modifier !== 0}
														<span class="text-slate-400">({opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)})</span>
													{/if}
												</span>
											{/each}
										</div>
									{/if}
									{#if item.manual_discount_percent}
										<span class="ml-5 text-xs text-orange-500">-{item.manual_discount_percent}%</span>
									{/if}
									{#if item.note}
										<p class="ml-5 text-xs text-amber-600 italic">{item.note}</p>
									{/if}
								</div>
								<div class="text-right shrink-0">
									<span class="text-slate-900 font-medium">{formatCurrency(item.line_total)}</span>
								</div>
							</div>
						{/each}
					</div>
				</div>

				<!-- Price breakdown -->
				<div class="px-5 py-3 border-b border-slate-100 space-y-1.5 text-sm">
					<div class="flex justify-between">
						<span class="text-slate-500">{$t('live.subtotal')}</span>
						<span class="text-slate-900">{formatCurrency(selectedOrder.subtotal)}</span>
					</div>
					{#if selectedOrder.total_discount > 0}
						<div class="flex justify-between">
							<span class="text-orange-500">{$t('live.discount')}</span>
							<span class="text-orange-500">-{formatCurrency(selectedOrder.total_discount)}</span>
						</div>
					{/if}
					{#if selectedOrder.total_surcharge > 0}
						<div class="flex justify-between">
							<span class="text-purple-500">{$t('live.surcharge')}</span>
							<span class="text-purple-500">+{formatCurrency(selectedOrder.total_surcharge)}</span>
						</div>
					{/if}
					{#if selectedOrder.comp_total_amount > 0}
						<div class="flex justify-between">
							<span class="text-emerald-600">{$t('live.comp')}</span>
							<span class="text-emerald-600">-{formatCurrency(selectedOrder.comp_total_amount)}</span>
						</div>
					{/if}
					{#if selectedOrder.tax > 0}
						<div class="flex justify-between">
							<span class="text-slate-500">{$t('live.tax')}</span>
							<span class="text-slate-900">{formatCurrency(selectedOrder.tax)}</span>
						</div>
					{/if}
					<div class="flex justify-between pt-1.5 border-t border-slate-100 font-bold text-base">
						<span class="text-slate-900">{$t('live.total')}</span>
						<span class="text-slate-900">{formatCurrency(selectedOrder.total)}</span>
					</div>
				</div>

				<!-- Payments -->
				<div class="px-5 py-3 border-b border-slate-100">
					<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{$t('live.payments')}</h3>
					{#if selectedOrder.payments.length === 0}
						<p class="text-sm text-slate-400 italic">{$t('live.no_payments')}</p>
					{:else}
						<div class="space-y-2">
							{#each selectedOrder.payments as payment}
								<div class="flex items-center justify-between text-sm {payment.cancelled ? 'opacity-50' : ''}">
									<div class="flex items-center gap-2">
										<CreditCard class="w-3.5 h-3.5 text-slate-400" />
										<span class="text-slate-700 capitalize">{paymentMethodLabel(payment.method)}</span>
										{#if payment.cancelled}
											<span class="px-1.5 py-0.5 text-[10px] bg-red-100 text-red-600 rounded font-medium">
												{$t('live.cancelled')}
											</span>
										{/if}
									</div>
									<span class="font-medium {payment.cancelled ? 'text-slate-400 line-through' : 'text-green-600'}">
										{formatCurrency(payment.amount)}
									</span>
								</div>
							{/each}
						</div>

						<div class="mt-3 pt-2 border-t border-slate-100 space-y-1">
							<div class="flex justify-between text-sm">
								<span class="text-slate-500">{$t('live.paid')}</span>
								<span class="text-green-600 font-medium">{formatCurrency(selectedOrder.paid_amount)}</span>
							</div>
							{#if selectedOrder.remaining_amount > 0}
								<div class="flex justify-between text-sm">
									<span class="text-slate-500">{$t('live.remaining')}</span>
									<span class="text-amber-600 font-medium">{formatCurrency(selectedOrder.remaining_amount)}</span>
								</div>
							{/if}
						</div>
					{/if}
				</div>

				<!-- Event Timeline (Mobile) -->
				{#if selectedOrder.events && selectedOrder.events.length > 0}
					<div class="px-5 py-3 pb-8">
						<button
							type="button"
							class="flex items-center justify-between w-full text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2"
							onclick={() => showEvents = !showEvents}
						>
							<span class="flex items-center gap-1.5">
								<History class="w-3.5 h-3.5" />
								{$t('live.events')} ({selectedOrder.events.length})
							</span>
							{#if showEvents}
								<ChevronUp class="w-3.5 h-3.5" />
							{:else}
								<ChevronDown class="w-3.5 h-3.5" />
							{/if}
						</button>
						{#if showEvents}
							<div class="relative ml-2">
								<div class="absolute left-[5px] top-2 bottom-2 w-px bg-slate-200"></div>
								<div class="space-y-3">
									{#each [...selectedOrder.events].reverse() as event}
										<div class="relative flex gap-3 pl-5">
											<div class="absolute left-0 top-1.5 w-[11px] h-[11px] rounded-full border-2 border-white {eventDotColor(event.event_type)} shadow-sm"></div>
											<div class="flex-1 min-w-0">
												<div class="flex items-center justify-between gap-2">
													<span class="text-xs font-medium text-slate-700 truncate">{eventLabel(event.event_type)}</span>
													<span class="text-[10px] text-slate-400 shrink-0">{formatDateTime(event.timestamp)}</span>
												</div>
												{#if eventDetail(event)}
													<p class="text-[11px] text-slate-500 mt-0.5 truncate">{eventDetail(event)}</p>
												{/if}
												<p class="text-[10px] text-slate-400 mt-0.5">{event.operator_name}</p>
											</div>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}

<style>
	@keyframes slide-up {
		from { transform: translateY(100%); }
		to { transform: translateY(0); }
	}
	.animate-slide-up {
		animation: slide-up 0.25s ease-out;
	}
</style>
