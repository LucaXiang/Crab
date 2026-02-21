<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		ArrowLeft,
		BarChart3,
		DollarSign,
		ShoppingBag,
		XCircle,
		CreditCard,
		Wallet,
		Receipt,
		Percent
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStats, ApiError, type DailyReportEntry } from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	interface PaymentBreakdown {
		method: string;
		count: number;
		amount: number;
	}

	interface TaxBreakdown {
		tax_rate: number;
		base_amount: number;
		tax_amount: number;
	}

	interface DailyReport {
		business_date: string;
		total_orders: number;
		completed_orders: number;
		void_orders: number;
		total_sales: number;
		total_paid: number;
		total_unpaid: number;
		void_amount: number;
		total_tax: number;
		total_discount: number;
		total_surcharge: number;
		payment_breakdowns: PaymentBreakdown[];
		tax_breakdowns: TaxBreakdown[];
	}

	let reports = $state<DailyReportEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function parseReport(entry: DailyReportEntry): DailyReport {
		const d = entry.data as Record<string, unknown>;
		return {
			business_date: (d.business_date as string) ?? '',
			total_orders: (d.total_orders as number) ?? 0,
			completed_orders: (d.completed_orders as number) ?? 0,
			void_orders: (d.void_orders as number) ?? 0,
			total_sales: (d.total_sales as number) ?? 0,
			total_paid: (d.total_paid as number) ?? 0,
			total_unpaid: (d.total_unpaid as number) ?? 0,
			void_amount: (d.void_amount as number) ?? 0,
			total_tax: (d.total_tax as number) ?? 0,
			total_discount: (d.total_discount as number) ?? 0,
			total_surcharge: (d.total_surcharge as number) ?? 0,
			payment_breakdowns: (d.payment_breakdowns as PaymentBreakdown[]) ?? [],
			tax_breakdowns: (d.tax_breakdowns as TaxBreakdown[]) ?? []
		};
	}

	let latest = $derived(reports.length > 0 ? parseReport(reports[0]) : null);

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			reports = await getStats(token, storeId);
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
	});
</script>

<svelte:head>
	<title>{$t('stats.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<h1 class="font-heading text-xl font-bold text-slate-900">{$t('stats.title')}</h1>
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
		{:else if !latest}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<BarChart3 class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('stats.no_data')}</p>
			</div>
		{:else}
			<!-- Latest report header -->
			<div
				class="bg-white rounded-2xl border border-slate-200 px-6 py-4 flex items-center justify-between"
			>
				<div class="flex items-center gap-2">
					<BarChart3 class="w-5 h-5 text-coral-500" />
					<span class="font-heading font-bold text-slate-900">{$t('stats.latest_report')}</span>
				</div>
				<span class="text-sm text-slate-500">{latest.business_date}</span>
			</div>

			<!-- KPI Cards -->
			<div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
				<!-- Revenue -->
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="flex items-center gap-2 mb-2">
						<div class="w-8 h-8 bg-coral-100 rounded-lg flex items-center justify-center">
							<DollarSign class="w-4 h-4 text-coral-600" />
						</div>
					</div>
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_sales')}</p>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(latest.total_sales)}</p>
				</div>

				<!-- Completed orders -->
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="flex items-center gap-2 mb-2">
						<div class="w-8 h-8 bg-green-100 rounded-lg flex items-center justify-center">
							<ShoppingBag class="w-4 h-4 text-green-600" />
						</div>
					</div>
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.completed_orders')}</p>
					<p class="text-lg font-bold text-slate-900">
						{latest.completed_orders}
						<span class="text-xs font-normal text-slate-400">/ {latest.total_orders}</span>
					</p>
				</div>

				<!-- Collected -->
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="flex items-center gap-2 mb-2">
						<div class="w-8 h-8 bg-blue-100 rounded-lg flex items-center justify-center">
							<Wallet class="w-4 h-4 text-blue-600" />
						</div>
					</div>
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_paid')}</p>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(latest.total_paid)}</p>
				</div>

				<!-- Discounts -->
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="flex items-center gap-2 mb-2">
						<div class="w-8 h-8 bg-orange-100 rounded-lg flex items-center justify-center">
							<Percent class="w-4 h-4 text-orange-600" />
						</div>
					</div>
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_discount')}</p>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(latest.total_discount)}</p>
				</div>

				<!-- Void orders -->
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="flex items-center gap-2 mb-2">
						<div class="w-8 h-8 bg-red-100 rounded-lg flex items-center justify-center">
							<XCircle class="w-4 h-4 text-red-600" />
						</div>
					</div>
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.void_orders')}</p>
					<p class="text-lg font-bold text-slate-900">
						{latest.void_orders}
						{#if latest.void_amount > 0}
							<span class="text-xs font-normal text-red-400"
								>{formatCurrency(latest.void_amount)}</span
							>
						{/if}
					</p>
				</div>
			</div>

			<!-- Secondary KPI row -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_unpaid')}</p>
					<p class="text-sm font-semibold text-slate-900">
						{formatCurrency(latest.total_unpaid)}
					</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_surcharge')}</p>
					<p class="text-sm font-semibold text-slate-900">
						{formatCurrency(latest.total_surcharge)}
					</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_tax')}</p>
					<p class="text-sm font-semibold text-slate-900">
						{formatCurrency(latest.total_tax)}
					</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_orders')}</p>
					<p class="text-sm font-semibold text-slate-900">{latest.total_orders}</p>
				</div>
			</div>

			<!-- Payment breakdown -->
			{#if latest.payment_breakdowns.length > 0}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<div class="flex items-center gap-2 mb-4">
						<CreditCard class="w-5 h-5 text-slate-400" />
						<h3 class="font-heading font-bold text-slate-900">
							{$t('stats.payment_breakdown')}
						</h3>
					</div>
					<div class="space-y-2">
						{#each latest.payment_breakdowns as pb}
							<div class="flex items-center justify-between py-2 border-b border-slate-50 last:border-0">
								<div class="flex items-center gap-3">
									<span class="text-sm font-medium text-slate-700 capitalize">{pb.method}</span>
									<span class="text-xs text-slate-400">{pb.count} {$t('stats.count')}</span>
								</div>
								<span class="text-sm font-semibold text-slate-900"
									>{formatCurrency(pb.amount)}</span
								>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Tax breakdown -->
			{#if latest.tax_breakdowns.length > 0}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<div class="flex items-center gap-2 mb-4">
						<Receipt class="w-5 h-5 text-slate-400" />
						<h3 class="font-heading font-bold text-slate-900">
							{$t('stats.tax_breakdown')}
						</h3>
					</div>
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-slate-100">
								<th class="text-left py-2 text-xs font-medium text-slate-400"
									>{$t('stats.tax_rate')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.tax_base')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.tax_amount')}</th
								>
							</tr>
						</thead>
						<tbody>
							{#each latest.tax_breakdowns as tb}
								<tr class="border-b border-slate-50 last:border-0">
									<td class="py-2 text-slate-700">{(tb.tax_rate * 100).toFixed(0)}%</td>
									<td class="py-2 text-right text-slate-700"
										>{formatCurrency(tb.base_amount)}</td
									>
									<td class="py-2 text-right font-semibold text-slate-900"
										>{formatCurrency(tb.tax_amount)}</td
									>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}

			<!-- History (remaining reports) -->
			{#if reports.length > 1}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<h3 class="font-heading font-bold text-slate-900 mb-4">{$t('stats.history')}</h3>
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-slate-100">
								<th class="text-left py-2 text-xs font-medium text-slate-400"
									>{$t('stats.business_date')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.total_sales')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.completed_orders')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.void_orders')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.total_paid')}</th
								>
								<th class="text-right py-2 text-xs font-medium text-slate-400"
									>{$t('stats.total_discount')}</th
								>
							</tr>
						</thead>
						<tbody>
							{#each reports.slice(1) as entry}
								{@const r = parseReport(entry)}
								<tr class="border-b border-slate-50 last:border-0">
									<td class="py-2 text-slate-700">{r.business_date}</td>
									<td class="py-2 text-right font-semibold text-slate-900"
										>{formatCurrency(r.total_sales)}</td
									>
									<td class="py-2 text-right text-slate-700">{r.completed_orders}</td>
									<td class="py-2 text-right text-slate-700">{r.void_orders}</td>
									<td class="py-2 text-right text-slate-700"
										>{formatCurrency(r.total_paid)}</td
									>
									<td class="py-2 text-right text-orange-500"
										>{formatCurrency(r.total_discount)}</td
									>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		{/if}
	</div>
</ConsoleLayout>
