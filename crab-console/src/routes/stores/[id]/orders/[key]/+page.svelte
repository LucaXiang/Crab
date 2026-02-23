<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Cloud, Wifi } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getOrderDetail, ApiError, type OrderDetailResponse } from '$lib/api';
	import { formatDateTime, formatCurrency } from '$lib/format';

	const storeId = Number(page.params.id);
	const orderKey = page.params.key ?? '';

	let detail = $state<OrderDetailResponse | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			detail = await getOrderDetail(token, storeId, orderKey);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('orders.detail')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-4xl mx-auto px-6 py-8 space-y-6">
	<h1 class="font-heading text-xl font-bold text-slate-900">{$t('orders.detail')}</h1>

	{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if detail}
			<!-- Source badge -->
			<div class="flex items-center gap-2 text-xs text-slate-400">
				{#if detail.source === 'cache'}
					<Cloud class="w-3.5 h-3.5" />
					<span>{$t('orders.source_cache')}</span>
				{:else}
					<Wifi class="w-3.5 h-3.5" />
					<span>{$t('orders.source_edge')}</span>
				{/if}
			</div>

			<!-- Order summary -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
					{#if detail.detail.zone_name}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.zone')}</p>
							<p class="font-medium text-slate-900">{detail.detail.zone_name}</p>
						</div>
					{/if}
					{#if detail.detail.table_name}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.table')}</p>
							<p class="font-medium text-slate-900">{detail.detail.table_name}</p>
						</div>
					{/if}
					{#if detail.detail.guest_count}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.guests')}</p>
							<p class="font-medium text-slate-900">{detail.detail.guest_count}</p>
						</div>
					{/if}
					{#if detail.detail.operator_name}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.operator')}</p>
							<p class="font-medium text-slate-900">{detail.detail.operator_name}</p>
						</div>
					{/if}
					{#if detail.detail.member_name}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.member')}</p>
							<p class="font-medium text-slate-900">{detail.detail.member_name}</p>
						</div>
					{/if}
				</div>

				{#if detail.detail.void_type}
					<div class="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm">
						<p class="text-red-600 font-medium">{$t('orders.voided')}: {detail.detail.void_type}</p>
						{#if detail.detail.loss_reason}<p class="text-red-500">{$t('orders.void_reason')}: {detail.detail.loss_reason}</p>{/if}
						{#if detail.detail.void_note}<p class="text-red-500">{$t('orders.void_note')}: {detail.detail.void_note}</p>{/if}
						{#if detail.detail.loss_amount != null}<p class="text-red-500">{$t('orders.loss_amount')}: {formatCurrency(detail.detail.loss_amount)}</p>{/if}
					</div>
				{/if}

				<!-- Totals -->
				<div class="mt-4 pt-4 border-t border-slate-100 grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
					<div>
						<p class="text-xs text-slate-400">{$t('orders.subtotal')}</p>
						<p class="font-semibold text-slate-900">{formatCurrency(detail.detail.subtotal)}</p>
					</div>
					{#if detail.detail.discount_amount > 0}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.discount')}</p>
							<p class="font-semibold text-green-600">-{formatCurrency(detail.detail.discount_amount)}</p>
						</div>
					{/if}
					{#if detail.detail.surcharge_amount > 0}
						<div>
							<p class="text-xs text-slate-400">{$t('orders.surcharge')}</p>
							<p class="font-semibold text-amber-600">+{formatCurrency(detail.detail.surcharge_amount)}</p>
						</div>
					{/if}
					<div>
						<p class="text-xs text-slate-400">{$t('orders.paid')}</p>
						<p class="font-semibold text-slate-900">{formatCurrency(detail.detail.paid_amount)}</p>
					</div>
				</div>
			</div>

			<!-- Items -->
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<div class="px-6 py-4 border-b border-slate-100">
					<h3 class="font-heading font-bold text-slate-900">{$t('orders.items')}</h3>
				</div>
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-slate-100">
							<th class="px-4 py-2 text-xs text-slate-400 text-left">Item</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-center">Qty</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-right">Unit</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-right">{$t('orders.total')}</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-right">IVA</th>
						</tr>
					</thead>
					<tbody>
						{#each detail.detail.items as item}
							<tr class="border-b border-slate-50">
								<td class="px-4 py-2.5">
									<div>
										<span class="text-slate-900 {item.is_comped ? 'line-through' : ''}">{item.name}</span>
										{#if item.spec_name}<span class="text-slate-400 ml-1">({item.spec_name})</span>{/if}
										{#if item.is_comped}<span class="text-xs text-amber-500 ml-1">{$t('orders.comped')}</span>{/if}
									</div>
									{#if item.note}<p class="text-xs text-slate-400 mt-0.5">{item.note}</p>{/if}
									{#each item.options as opt}
										<p class="text-xs text-slate-400">+ {opt.option_name} {opt.price > 0 ? formatCurrency(opt.price) : ''}</p>
									{/each}
								</td>
								<td class="px-4 py-2.5 text-center text-slate-600">{item.quantity}</td>
								<td class="px-4 py-2.5 text-right text-slate-600">{formatCurrency(item.unit_price)}</td>
								<td class="px-4 py-2.5 text-right font-medium text-slate-900">{formatCurrency(item.line_total)}</td>
								<td class="px-4 py-2.5 text-right text-slate-500">{item.tax_rate}%</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Payments -->
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<div class="px-6 py-4 border-b border-slate-100">
					<h3 class="font-heading font-bold text-slate-900">{$t('orders.payments')}</h3>
				</div>
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-slate-100">
							<th class="px-4 py-2 text-xs text-slate-400 text-left">#</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-left">Method</th>
							<th class="px-4 py-2 text-xs text-slate-400 text-right">{$t('orders.total')}</th>
							<th class="px-4 py-2 text-xs text-slate-400">{$t('orders.date')}</th>
						</tr>
					</thead>
					<tbody>
						{#each detail.detail.payments as payment}
							<tr class="border-b border-slate-50">
								<td class="px-4 py-2.5 text-slate-500">{payment.seq}</td>
								<td class="px-4 py-2.5 text-slate-900 capitalize {payment.cancelled ? 'line-through text-slate-400' : ''}">
									{payment.method}
									{#if payment.cancelled}<span class="text-xs text-red-500 ml-1">{$t('orders.cancelled_payment')}</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-right font-medium text-slate-900">{formatCurrency(payment.amount)}</td>
								<td class="px-4 py-2.5 text-slate-500">{formatDateTime(payment.timestamp)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Tax breakdown (VeriFactu desglose) -->
			{#if detail.desglose.length > 0}
				<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
					<div class="px-6 py-4 border-b border-slate-100">
						<h3 class="font-heading font-bold text-slate-900">{$t('orders.tax_breakdown')}</h3>
					</div>
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-slate-100">
								<th class="px-4 py-2 text-xs text-slate-400 text-left">{$t('orders.tax_rate')}</th>
								<th class="px-4 py-2 text-xs text-slate-400 text-right">{$t('orders.tax_base')}</th>
								<th class="px-4 py-2 text-xs text-slate-400 text-right">{$t('orders.tax_amount')}</th>
							</tr>
						</thead>
						<tbody>
							{#each detail.desglose as d}
								<tr class="border-b border-slate-50">
									<td class="px-4 py-2.5 text-slate-900">{d.tax_rate}%</td>
									<td class="px-4 py-2.5 text-right text-slate-600">{formatCurrency(d.base_amount)}</td>
									<td class="px-4 py-2.5 text-right font-medium text-slate-900">{formatCurrency(d.tax_amount)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		{/if}
	</div>
