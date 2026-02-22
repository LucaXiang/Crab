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
		Receipt,
		TrendingUp,
		Award,
		PieChart,
		Users,
		Clock,
		AlertTriangle,
		Tag
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStoreOverview, ApiError, type StoreOverview } from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let overview = $state<StoreOverview | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function getTodayRange(): { from: number; to: number } {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		return { from: start.getTime(), to: now.getTime() + 60000 };
	}

	let maxTrendRevenue = $derived(
		overview ? Math.max(...overview.revenue_trend.map((p) => p.revenue), 1) : 1
	);

	let totalCategorySales = $derived(
		overview ? overview.category_sales.reduce((sum, c) => sum + c.revenue, 0) : 0
	);

	const CATEGORY_COLORS = [
		'bg-coral-500',
		'bg-blue-500',
		'bg-green-500',
		'bg-amber-500',
		'bg-purple-500',
		'bg-pink-500',
		'bg-teal-500',
		'bg-orange-500',
		'bg-indigo-500',
		'bg-rose-500'
	];

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			const { from, to } = getTodayRange();
			overview = await getStoreOverview(token, storeId, from, to);
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
	<title>{$t('stats.overview')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('stats.overview')}</h1>
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
		{:else if overview}
			<!-- Today header -->
			<div class="bg-white rounded-2xl border border-slate-200 px-6 py-4 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<BarChart3 class="w-5 h-5 text-coral-500" />
					<span class="font-heading font-bold text-slate-900">{$t('stats.today')}</span>
				</div>
				<span class="text-sm text-slate-400">{new Date().toLocaleDateString()}</span>
			</div>

			<!-- KPI Row 1: Revenue / Orders / Guests / Avg Order -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-coral-100 rounded-lg flex items-center justify-center mb-2">
						<DollarSign class="w-4 h-4 text-coral-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(overview.revenue)}</p>
					<p class="text-xs text-slate-400">{$t('stats.total_sales')}</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-green-100 rounded-lg flex items-center justify-center mb-2">
						<ShoppingBag class="w-4 h-4 text-green-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{overview.orders}</p>
					<p class="text-xs text-slate-400">{$t('stats.completed_orders')}</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-blue-100 rounded-lg flex items-center justify-center mb-2">
						<Users class="w-4 h-4 text-blue-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{overview.guests}</p>
					<p class="text-xs text-slate-400">{$t('stats.guests')}</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-purple-100 rounded-lg flex items-center justify-center mb-2">
						<TrendingUp class="w-4 h-4 text-purple-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(overview.average_order_value)}</p>
					<p class="text-xs text-slate-400">{$t('stats.average_order')}</p>
				</div>
			</div>

			<!-- KPI Row 2: Per-payment breakdown + Per Guest + Avg Dining Time -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				{#each overview.payment_breakdown.slice(0, 2) as pb}
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="w-8 h-8 bg-indigo-100 rounded-lg flex items-center justify-center mb-2">
							<CreditCard class="w-4 h-4 text-indigo-600" />
						</div>
						<p class="text-lg font-bold text-slate-900">{formatCurrency(pb.amount)}</p>
						<p class="text-xs text-slate-400 capitalize">{pb.method}</p>
					</div>
				{/each}
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-teal-100 rounded-lg flex items-center justify-center mb-2">
						<Users class="w-4 h-4 text-teal-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(overview.per_guest_spend)}</p>
					<p class="text-xs text-slate-400">{$t('stats.per_guest')}</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-amber-100 rounded-lg flex items-center justify-center mb-2">
						<Clock class="w-4 h-4 text-amber-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">
						{overview.average_dining_minutes > 0 ? `${Math.round(overview.average_dining_minutes)} min` : '-'}
					</p>
					<p class="text-xs text-slate-400">{$t('stats.avg_dining_time')}</p>
				</div>
			</div>

			<!-- KPI Row 3: Voided / Loss / Discount -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-red-100 rounded-lg flex items-center justify-center mb-2">
						<XCircle class="w-4 h-4 text-red-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{overview.voided_orders}</p>
					<p class="text-xs text-slate-400">{$t('stats.void_orders')} ({formatCurrency(overview.voided_amount)})</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-orange-100 rounded-lg flex items-center justify-center mb-2">
						<AlertTriangle class="w-4 h-4 text-orange-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{overview.loss_orders}</p>
					<p class="text-xs text-slate-400">{$t('stats.loss_orders')} ({formatCurrency(overview.loss_amount)})</p>
				</div>
				<div class="bg-white rounded-xl border border-slate-200 p-4">
					<div class="w-8 h-8 bg-yellow-100 rounded-lg flex items-center justify-center mb-2">
						<Tag class="w-4 h-4 text-yellow-600" />
					</div>
					<p class="text-lg font-bold text-slate-900">{formatCurrency(overview.total_discount)}</p>
					<p class="text-xs text-slate-400">{$t('stats.total_discount')}</p>
				</div>
			</div>

			<!-- Revenue trend (hourly bar chart) -->
			{#if overview.revenue_trend.length > 0}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<div class="flex items-center gap-2 mb-4">
						<TrendingUp class="w-5 h-5 text-slate-400" />
						<h3 class="font-heading font-bold text-slate-900">{$t('stats.revenue_trend')}</h3>
					</div>
					<div class="flex items-end gap-1 h-32">
						{#each overview.revenue_trend as point}
							{@const pct = (point.revenue / maxTrendRevenue) * 100}
							<div class="flex-1 flex flex-col items-center gap-1 group relative">
								<div
									class="w-full bg-coral-400 rounded-t transition-all hover:bg-coral-500 min-h-[2px]"
									style="height: {pct}%"
								></div>
								<span class="text-[10px] text-slate-400">{point.hour}h</span>
								<div class="absolute bottom-full mb-2 hidden group-hover:block bg-slate-800 text-white text-xs rounded px-2 py-1 whitespace-nowrap z-10">
									{formatCurrency(point.revenue)} · {point.orders} {$t('stats.orders_label')}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Two columns: Top Products + Category Sales -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#if overview.top_products.length > 0}
					<div class="bg-white rounded-2xl border border-slate-200 p-6">
						<div class="flex items-center gap-2 mb-4">
							<Award class="w-5 h-5 text-slate-400" />
							<h3 class="font-heading font-bold text-slate-900">{$t('stats.top_products')}</h3>
						</div>
						<div class="space-y-2">
							{#each overview.top_products as product, i}
								<div class="flex items-center justify-between py-1.5">
									<div class="flex items-center gap-2 min-w-0">
										<span class="text-xs font-bold text-slate-300 w-5 text-right">{i + 1}</span>
										<span class="text-sm text-slate-700 truncate">{product.name}</span>
									</div>
									<div class="flex items-center gap-3 shrink-0 ml-2">
										<span class="text-xs text-slate-400">{product.quantity}x</span>
										<span class="text-sm font-semibold text-slate-900 w-20 text-right">{formatCurrency(product.revenue)}</span>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				{#if overview.category_sales.length > 0}
					<div class="bg-white rounded-2xl border border-slate-200 p-6">
						<div class="flex items-center gap-2 mb-4">
							<PieChart class="w-5 h-5 text-slate-400" />
							<h3 class="font-heading font-bold text-slate-900">{$t('stats.category_sales')}</h3>
						</div>
						<div class="flex h-3 rounded-full overflow-hidden mb-4">
							{#each overview.category_sales as cat, i}
								{@const pct = totalCategorySales > 0 ? (cat.revenue / totalCategorySales) * 100 : 0}
								<div class="{CATEGORY_COLORS[i % CATEGORY_COLORS.length]}" style="width: {pct}%"></div>
							{/each}
						</div>
						<div class="space-y-2">
							{#each overview.category_sales as cat, i}
								{@const pct = totalCategorySales > 0 ? (cat.revenue / totalCategorySales) * 100 : 0}
								<div class="flex items-center justify-between py-1">
									<div class="flex items-center gap-2">
										<div class="w-2.5 h-2.5 rounded-sm {CATEGORY_COLORS[i % CATEGORY_COLORS.length]}"></div>
										<span class="text-sm text-slate-700">{cat.name}</span>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-slate-400">{pct.toFixed(0)}%</span>
										<span class="text-sm font-semibold text-slate-900 w-20 text-right">{formatCurrency(cat.revenue)}</span>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			</div>

			<!-- Two columns: Payment + Tax breakdown -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#if overview.payment_breakdown.length > 0}
					<div class="bg-white rounded-2xl border border-slate-200 p-6">
						<div class="flex items-center gap-2 mb-4">
							<CreditCard class="w-5 h-5 text-slate-400" />
							<h3 class="font-heading font-bold text-slate-900">{$t('stats.payment_breakdown')}</h3>
						</div>
						<div class="space-y-2">
							{#each overview.payment_breakdown as pb}
								<div class="flex items-center justify-between py-2 border-b border-slate-50 last:border-0">
									<div class="flex items-center gap-2">
										<span class="text-sm font-medium text-slate-700 capitalize">{pb.method}</span>
										<span class="text-xs text-slate-400">{pb.count}x</span>
									</div>
									<span class="text-sm font-semibold text-slate-900">{formatCurrency(pb.amount)}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				{#if overview.tax_breakdown.length > 0}
					<div class="bg-white rounded-2xl border border-slate-200 p-6">
						<div class="flex items-center gap-2 mb-4">
							<Receipt class="w-5 h-5 text-slate-400" />
							<h3 class="font-heading font-bold text-slate-900">{$t('stats.tax_breakdown')}</h3>
						</div>
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-slate-100">
									<th class="text-left py-2 text-xs font-medium text-slate-400">{$t('stats.tax_rate')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.tax_base')}</th>
									<th class="text-right py-2 text-xs font-medium text-slate-400">{$t('stats.tax_amount')}</th>
								</tr>
							</thead>
							<tbody>
								{#each overview.tax_breakdown as tb}
									<tr class="border-b border-slate-50 last:border-0">
										<td class="py-2 text-slate-700">{tb.tax_rate}%</td>
										<td class="py-2 text-right text-slate-700">{formatCurrency(tb.base_amount)}</td>
										<td class="py-2 text-right font-semibold text-slate-900">{formatCurrency(tb.tax_amount)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<BarChart3 class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('stats.no_data')}</p>
			</div>
		{/if}
	</div>
</ConsoleLayout>
