<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, ScrollText } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStats, getStoreOverview, ApiError, type DailyReportEntry, type StoreOverview } from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';
	import StoreOverviewDisplay from '$lib/components/StoreOverviewDisplay.svelte';

	const storeId = Number(page.params.id);

	let reports = $state<DailyReportEntry[]>([]);
	let todayOverview = $state<StoreOverview | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function getTodayRange(): { from: number; to: number } {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 0, 0, 0, 0);
		const end = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 23, 59, 59, 999);
		return { from: start.getTime(), to: end.getTime() };
	}

	interface DailyReport {
		business_date: string;
		total_orders: number;
		completed_orders: number;
		void_orders: number;
		total_sales: number;
		total_paid: number;
		total_discount: number;
	}

	function parseReport(entry: DailyReportEntry): DailyReport {
		const d = entry.data as Record<string, unknown>;
		return {
			business_date: (d.business_date as string) ?? '',
			total_orders: (d.total_orders as number) ?? 0,
			completed_orders: (d.completed_orders as number) ?? 0,
			void_orders: (d.void_orders as number) ?? 0,
			total_sales: (d.total_sales as number) ?? 0,
			total_paid: (d.total_paid as number) ?? 0,
			total_discount: (d.total_discount as number) ?? 0
		};
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			const { from, to } = getTodayRange();
			const [reportsData, overviewData] = await Promise.all([
				getStats(token, storeId),
				getStoreOverview(token, storeId, from, to)
			]);
			reports = reportsData;
			todayOverview = overviewData;
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
	<title>{$t('stats.daily_report')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<h1 class="font-heading text-xl font-bold text-slate-900">{$t('stats.daily_report')}</h1>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg
					class="animate-spin w-8 h-8 text-coral-500"
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
				>
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
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
		{:else}
			{#if todayOverview}
				<div class="mb-6">
					<StoreOverviewDisplay overview={todayOverview} />
				</div>
			{/if}

			{#if reports.length > 0}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<div class="overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-slate-100">
									<th class="text-left py-2 text-xs font-medium text-slate-400">{$t('stats.business_date')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.total_sales')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.completed_orders')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.void_orders')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.total_paid')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.total_discount')}</th>
								</tr>
							</thead>
							<tbody>
								{#each reports as entry}
									{@const r = parseReport(entry)}
									<tr class="border-b border-slate-50 last:border-0 hover:bg-slate-50 transition-colors">
										<td class="py-2 text-slate-700">
											<a
												href="/stores/{storeId}/stats/{r.business_date}"
												class="text-coral-500 hover:text-coral-600 font-medium hover:underline"
											>
												{r.business_date}
											</a>
										</td>
										<td class="py-2 text-right font-semibold text-slate-900">{formatCurrency(r.total_sales)}</td>
										<td class="py-2 text-right text-slate-700">{r.completed_orders}</td>
										<td class="py-2 text-right text-slate-700">{r.void_orders}</td>
										<td class="py-2 text-right text-slate-700">{formatCurrency(r.total_paid)}</td>
										<td class="py-2 text-right text-orange-500">{formatCurrency(r.total_discount)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{:else if !todayOverview}
				<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
					<ScrollText class="w-10 h-10 text-slate-300 mx-auto mb-3" />
					<p class="text-sm text-slate-500">{$t('stats.no_data')}</p>
				</div>
			{/if}
		{/if}
	</div>
</ConsoleLayout>
