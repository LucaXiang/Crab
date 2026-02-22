<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, ShieldAlert } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStoreRedFlags, ApiError, type RedFlagsResponse } from '$lib/api';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let data = $state<RedFlagsResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let dateInput = $state(new Date().toISOString().slice(0, 10));

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function getRange(dateStr: string): { from: number; to: number } {
		const d = new Date(dateStr + 'T00:00:00');
		const next = new Date(d);
		next.setDate(next.getDate() + 1);
		return { from: d.getTime(), to: next.getTime() };
	}

	async function loadData() {
		loading = true;
		error = '';
		try {
			const { from, to } = getRange(dateInput);
			data = await getStoreRedFlags(token, storeId, from, to);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				clearAuth();
				goto('/login');
				return;
			}
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}
		await loadData();
	});

	function handleDateChange() {
		loadData();
	}

	function avgFlags(total: number, count: number): number {
		return count > 0 ? total / count : 0;
	}
</script>

<svelte:head>
	<title>{$t('red_flags.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<a
			href="/stores/{storeId}"
			class="inline-flex items-center gap-1.5 text-sm text-slate-500 hover:text-slate-700"
		>
			<ArrowLeft class="w-4 h-4" />
			<span>{$t('store.back')}</span>
		</a>

		<div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
			<div class="flex items-center gap-3">
				<div
					class="w-10 h-10 bg-red-100 rounded-xl flex items-center justify-center shrink-0"
				>
					<ShieldAlert class="w-5 h-5 text-red-600" />
				</div>
				<h1 class="font-heading text-xl font-bold text-slate-900">
					{$t('red_flags.title')}
				</h1>
			</div>
			<input
				type="date"
				bind:value={dateInput}
				onchange={handleDateChange}
				class="rounded-lg border-slate-200 text-sm px-3 py-2 focus:border-coral-500 focus:ring-coral-500"
			/>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg
					class="animate-spin w-8 h-8 text-coral-500"
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
				>
					<circle
						class="opacity-25"
						cx="12"
						cy="12"
						r="10"
						stroke="currentColor"
						stroke-width="4"
					/>
					<path
						class="opacity-75"
						fill="currentColor"
						d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
					/>
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
				{error}
			</div>
		{:else if data}
			<!-- KPI Cards -->
			<div class="grid grid-cols-2 md:grid-cols-5 gap-3">
				<div class="bg-white rounded-xl border border-red-200 p-4 text-center">
					<div class="text-2xl font-bold text-red-600">
						{data.summary.item_removals}
					</div>
					<div class="text-xs text-slate-500 mt-1">{$t('red_flags.item_removals')}</div>
				</div>
				<div class="bg-white rounded-xl border border-orange-200 p-4 text-center">
					<div class="text-2xl font-bold text-orange-600">
						{data.summary.item_comps}
					</div>
					<div class="text-xs text-slate-500 mt-1">{$t('red_flags.item_comps')}</div>
				</div>
				<div class="bg-white rounded-xl border border-red-200 p-4 text-center">
					<div class="text-2xl font-bold text-red-600">
						{data.summary.order_voids}
					</div>
					<div class="text-xs text-slate-500 mt-1">{$t('red_flags.order_voids')}</div>
				</div>
				<div class="bg-white rounded-xl border border-yellow-200 p-4 text-center">
					<div class="text-2xl font-bold text-yellow-600">
						{data.summary.order_discounts}
					</div>
					<div class="text-xs text-slate-500 mt-1">
						{$t('red_flags.order_discounts')}
					</div>
				</div>
				<div class="bg-white rounded-xl border border-orange-200 p-4 text-center">
					<div class="text-2xl font-bold text-orange-600">
						{data.summary.price_modifications}
					</div>
					<div class="text-xs text-slate-500 mt-1">
						{$t('red_flags.price_modifications')}
					</div>
				</div>
			</div>

			<!-- Operator breakdown -->
			{#if data.operator_breakdown.length === 0}
				<div
					class="bg-white rounded-2xl border border-slate-200 p-8 text-center text-slate-400 text-sm"
				>
					{$t('red_flags.no_data')}
				</div>
			{:else}
				{@const totalFlags = data.operator_breakdown.reduce((s, o) => s + o.total_flags, 0)}
				{@const avgPerOperator = avgFlags(totalFlags, data.operator_breakdown.length)}
				<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
					<div class="px-6 py-4 border-b border-slate-100">
						<h2 class="font-heading font-semibold text-slate-900">
							{$t('red_flags.operator')}
						</h2>
					</div>
					<div class="overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-slate-100 text-left text-slate-500">
									<th class="px-6 py-3 font-medium">
										{$t('red_flags.operator')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.item_removals')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.item_comps')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.order_voids')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.order_discounts')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.price_modifications')}
									</th>
									<th class="px-4 py-3 font-medium text-center">
										{$t('red_flags.total')}
									</th>
								</tr>
							</thead>
							<tbody>
								{#each data.operator_breakdown as op}
									{@const isHigh = op.total_flags > avgPerOperator * 2}
									<tr
										class="border-b border-slate-50 {isHigh
											? 'bg-red-50'
											: ''}"
									>
										<td class="px-6 py-3 font-medium text-slate-900">
											{op.operator_name ?? $t('red_flags.unknown_operator')}
										</td>
										<td class="px-4 py-3 text-center tabular-nums">
											{op.item_removals || '-'}
										</td>
										<td class="px-4 py-3 text-center tabular-nums">
											{op.item_comps || '-'}
										</td>
										<td class="px-4 py-3 text-center tabular-nums">
											{op.order_voids || '-'}
										</td>
										<td class="px-4 py-3 text-center tabular-nums">
											{op.order_discounts || '-'}
										</td>
										<td class="px-4 py-3 text-center tabular-nums">
											{op.price_modifications || '-'}
										</td>
										<td
											class="px-4 py-3 text-center font-bold tabular-nums {isHigh
												? 'text-red-600'
												: 'text-slate-900'}"
										>
											{op.total_flags}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}

			<!-- Compliance -->
			<div class="text-xs text-slate-400 text-center py-2">
				{$t('red_flags.compliance')}
			</div>
		{/if}
	</div>
</ConsoleLayout>
