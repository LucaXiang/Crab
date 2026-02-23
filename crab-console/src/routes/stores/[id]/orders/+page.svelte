<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ShoppingBag } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getOrders, ApiError, type OrderSummary } from '$lib/api';
	import { formatDateTime, formatCurrency } from '$lib/format';

	const storeId = Number(page.params.id);

	let orders = $state<OrderSummary[]>([]);
	let loading = $state(true);
	let error = $state('');
	let currentPage = $state(1);
	let hasMore = $state(true);
	let statusFilter = $state<string | undefined>(undefined);
	let loadingMore = $state(false);

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	async function loadOrders(reset = false) {
		if (reset) {
			currentPage = 1;
			orders = [];
			hasMore = true;
		}
		try {
			const batch = await getOrders(token, storeId, currentPage, 20, statusFilter);
			if (reset) {
				orders = batch;
			} else {
				orders = [...orders, ...batch];
			}
			hasMore = batch.length === 20;
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		}
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }
		await loadOrders(true);
		loading = false;
	});

	async function loadMore() {
		loadingMore = true;
		currentPage++;
		await loadOrders();
		loadingMore = false;
	}

	async function setFilter(status: string | undefined) {
		statusFilter = status;
		loading = true;
		await loadOrders(true);
		loading = false;
	}

	function statusBadge(status: string): string {
		switch (status) {
			case 'COMPLETED': return 'bg-green-50 text-green-600';
			case 'VOID': return 'bg-red-50 text-red-600';
			case 'MERGED': return 'bg-blue-50 text-blue-600';
			default: return 'bg-slate-50 text-slate-600';
		}
	}

	function statusLabel(status: string): string {
		switch (status) {
			case 'COMPLETED': return $t('orders.completed');
			case 'VOID': return $t('orders.void');
			case 'MERGED': return $t('orders.merged');
			default: return status;
		}
	}
</script>

<svelte:head>
	<title>{$t('orders.title')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
	<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('orders.title')}</h1>

	<!-- Filters -->
		<div class="flex gap-2 overflow-x-auto pb-2 md:pb-0 no-scrollbar">
			{#each [
				{ value: undefined, label: 'orders.all' },
				{ value: 'COMPLETED', label: 'orders.completed' },
				{ value: 'VOID', label: 'orders.void' },
				{ value: 'MERGED', label: 'orders.merged' }
			] as filter}
				<button
					onclick={() => setFilter(filter.value)}
					class="px-3 py-1.5 rounded-lg text-sm font-medium cursor-pointer transition-colors duration-150 whitespace-nowrap {statusFilter === filter.value ? 'bg-coral-500 text-white' : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50'}"
				>
					{$t(filter.label)}
				</button>
			{/each}
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
		{:else if orders.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<ShoppingBag class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('orders.empty')}</p>
			</div>
		{:else}
			<!-- Desktop Table View -->
			<div class="hidden md:block bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-slate-100 text-left">
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('orders.receipt')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('orders.status')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400 text-right">{$t('orders.total')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('orders.date')}</th>
						</tr>
					</thead>
					<tbody>
						{#each orders as order}
							<tr class="border-b border-slate-50 hover:bg-slate-50/50 cursor-pointer" onclick={() => goto(`/stores/${storeId}/orders/${order.source_id}`)}>
								<td class="px-4 py-3 font-medium text-slate-900">{order.receipt_number ?? order.source_id.slice(0, 8)}</td>
								<td class="px-4 py-3">
									<span class="inline-flex px-2 py-0.5 rounded-full text-xs font-medium {statusBadge(order.status)}">
										{statusLabel(order.status)}
									</span>
								</td>
								<td class="px-4 py-3 text-right font-medium text-slate-900">{order.total != null ? formatCurrency(order.total) : '—'}</td>
								<td class="px-4 py-3 text-slate-500">{order.end_time ? formatDateTime(order.end_time) : '—'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Mobile Card View -->
			<div class="md:hidden space-y-3">
				{#each orders as order}
					<div
						class="bg-white rounded-xl border border-slate-200 p-4 shadow-sm active:scale-[0.98] transition-transform cursor-pointer"
						onclick={() => goto(`/stores/${storeId}/orders/${order.source_id}`)}
						onkeydown={(e) => e.key === 'Enter' && goto(`/stores/${storeId}/orders/${order.source_id}`)}
						role="button"
						tabindex="0"
					>
						<div class="flex items-start justify-between mb-3">
							<div>
								<p class="font-bold text-slate-900">#{order.receipt_number ?? order.source_id.slice(0, 8)}</p>
								<p class="text-xs text-slate-400 mt-0.5">{order.end_time ? formatDateTime(order.end_time) : '—'}</p>
							</div>
							<span class="inline-flex px-2.5 py-1 rounded-full text-xs font-medium {statusBadge(order.status)}">
								{statusLabel(order.status)}
							</span>
						</div>
						<div class="flex items-center justify-between pt-3 border-t border-slate-100">
							<span class="text-sm text-slate-500">{$t('orders.total')}</span>
							<span class="text-lg font-bold text-slate-900">{order.total != null ? formatCurrency(order.total) : '—'}</span>
						</div>
					</div>
				{/each}
			</div>

			{#if hasMore}
				<div class="text-center pt-2">
					<button
						onclick={loadMore}
						disabled={loadingMore}
						class="px-4 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-600 hover:bg-slate-50 cursor-pointer disabled:opacity-50"
					>
						{loadingMore ? $t('auth.loading') : $t('orders.load_more')}
					</button>
				</div>
			{/if}
		{/if}
	</div>
