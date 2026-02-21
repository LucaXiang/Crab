<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		CreditCard,
		Server,
		Clock,
		CheckCircle,
		AlertTriangle,
		XCircle,
		ArrowRight,
		Sparkles,
		DollarSign,
		ShoppingBag,
		Wallet,
		Percent,
		BarChart3
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getProfile,
		getStores,
		getStats,
		createBillingPortal,
		createCheckout,
		ApiError,
		type ProfileResponse,
		type StoreDetail,
		type DailyReportEntry
	} from '$lib/api';
	import { formatDate, formatCurrency, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	interface AggregateStats {
		total_sales: number;
		completed_orders: number;
		total_orders: number;
		void_orders: number;
		total_paid: number;
		total_discount: number;
		total_tax: number;
		total_unpaid: number;
		stores_with_data: number;
	}

	let profile = $state<ProfileResponse | null>(null);
	let stores = $state<StoreDetail[]>([]);
	let aggregate = $state<AggregateStats | null>(null);
	let loading = $state(true);
	let error = $state('');
	let billingLoading = $state(false);
	let checkoutLoading = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	// Tenant is verified but has no subscription yet
	let needsOnboarding = $derived(
		profile !== null && profile.profile.status === 'verified' && !profile.subscription
	);

	function aggregateReports(allReports: DailyReportEntry[][]): AggregateStats {
		const stats: AggregateStats = {
			total_sales: 0,
			completed_orders: 0,
			total_orders: 0,
			void_orders: 0,
			total_paid: 0,
			total_discount: 0,
			total_tax: 0,
			total_unpaid: 0,
			stores_with_data: 0
		};
		for (const reports of allReports) {
			if (reports.length === 0) continue;
			stats.stores_with_data++;
			const d = reports[0].data as Record<string, number>;
			stats.total_sales += d.total_sales ?? 0;
			stats.completed_orders += d.completed_orders ?? 0;
			stats.total_orders += d.total_orders ?? 0;
			stats.void_orders += d.void_orders ?? 0;
			stats.total_paid += d.total_paid ?? 0;
			stats.total_discount += d.total_discount ?? 0;
			stats.total_tax += d.total_tax ?? 0;
			stats.total_unpaid += d.total_unpaid ?? 0;
		}
		return stats;
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			const profileRes = await getProfile(token);
			profile = profileRes;

			// Only fetch stores if tenant is active (has subscription)
			if (profileRes.subscription) {
				stores = await getStores(token);

				// Fetch latest stats for all stores in parallel
				if (stores.length > 0) {
					const allStats = await Promise.all(
						stores.map((s) => getStats(token, s.id).catch(() => []))
					);
					aggregate = aggregateReports(allStats);
				}
			}
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

	async function handleBillingPortal() {
		billingLoading = true;
		try {
			const res = await createBillingPortal(token);
			window.location.href = res.url;
		} catch (err) {
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			billingLoading = false;
		}
	}

	async function handleChoosePlan(plan: string) {
		checkoutLoading = plan;
		error = '';
		try {
			const res = await createCheckout(token, plan);
			window.location.href = res.checkout_url;
		} catch (err) {
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			checkoutLoading = '';
		}
	}

	function statusLabel(status: string): string {
		switch (status) {
			case 'active':
				return $t('dash.active');
			case 'suspended':
				return $t('dash.suspended');
			default:
				return $t('dash.cancelled');
		}
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'active':
				return 'text-green-600 bg-green-50';
			case 'suspended':
				return 'text-amber-600 bg-amber-50';
			default:
				return 'text-red-600 bg-red-50';
		}
	}
</script>

<svelte:head>
	<title>{$t('dash.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout email={profile?.profile.email ?? ''}>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
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
		{:else if needsOnboarding}
			<!-- Onboarding: Choose plan -->
			<div class="text-center mb-2">
				<div
					class="w-14 h-14 bg-coral-100 rounded-2xl flex items-center justify-center mx-auto mb-4"
				>
					<Sparkles class="w-7 h-7 text-coral-500" />
				</div>
				<h1 class="font-heading text-2xl font-bold text-slate-900 mb-2">
					{$t('onboard.title')}
				</h1>
				<p class="text-sm text-slate-500 max-w-md mx-auto">{$t('onboard.subtitle')}</p>
			</div>

			<div class="grid md:grid-cols-2 gap-6 max-w-3xl mx-auto">
				<!-- Basic -->
				<div class="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col">
					<h3 class="font-heading font-bold text-lg text-slate-900">Basic</h3>
					<p class="text-sm text-slate-500 mt-1">{$t('onboard.basic_desc')}</p>
					<div class="mt-4 mb-6">
						<span class="text-3xl font-bold text-slate-900">&euro;39</span>
						<span class="text-sm text-slate-500">/{$t('onboard.month')}</span>
					</div>
					<ul class="space-y-2 text-sm text-slate-600 mb-6 flex-1">
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 1 {$t(
								'onboard.edge_server'
							)}
						</li>
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 5 {$t(
								'onboard.terminals'
							)}
						</li>
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
							{$t('onboard.cloud_sync')}
						</li>
					</ul>
					<button
						onclick={() => handleChoosePlan('basic')}
						disabled={checkoutLoading !== ''}
						class="w-full py-3 bg-slate-800 hover:bg-slate-900 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
					>
						{#if checkoutLoading === 'basic'}
							<svg
								class="animate-spin w-4 h-4"
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
						{/if}
						{$t('onboard.choose')}
					</button>
				</div>

				<!-- Pro -->
				<div class="bg-white rounded-2xl border-2 border-coral-500 p-6 flex flex-col relative">
					<span
						class="absolute -top-3 left-6 bg-coral-500 text-white text-xs font-bold px-3 py-0.5 rounded-full"
						>{$t('onboard.popular')}</span
					>
					<h3 class="font-heading font-bold text-lg text-slate-900">Pro</h3>
					<p class="text-sm text-slate-500 mt-1">{$t('onboard.pro_desc')}</p>
					<div class="mt-4 mb-6">
						<span class="text-3xl font-bold text-slate-900">&euro;69</span>
						<span class="text-sm text-slate-500">/{$t('onboard.month')}</span>
					</div>
					<ul class="space-y-2 text-sm text-slate-600 mb-6 flex-1">
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 3 {$t(
								'onboard.edge_server'
							)}
						</li>
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 10 {$t(
								'onboard.terminals'
							)}
						</li>
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
							{$t('onboard.cloud_sync')}
						</li>
						<li class="flex items-center gap-2">
							<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
							{$t('onboard.priority_support')}
						</li>
					</ul>
					<button
						onclick={() => handleChoosePlan('pro')}
						disabled={checkoutLoading !== ''}
						class="w-full py-3 bg-coral-500 hover:bg-coral-600 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
					>
						{#if checkoutLoading === 'pro'}
							<svg
								class="animate-spin w-4 h-4"
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
						{/if}
						{$t('onboard.choose')}
					</button>
				</div>
			</div>
		{:else if profile}
			<!-- Subscription Card -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<div class="flex items-start justify-between">
					<div>
						<h2 class="font-heading font-bold text-lg text-slate-900 mb-1">
							{$t('dash.subscription')}
						</h2>
						<p class="text-sm text-slate-500">{profile.profile.email}</p>
					</div>
					<span
						class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium {statusColor(
							profile.subscription.status
						)}"
					>
						{#if profile.subscription.status === 'active'}
							<CheckCircle class="w-3.5 h-3.5" />
						{:else if profile.subscription.status === 'suspended'}
							<AlertTriangle class="w-3.5 h-3.5" />
						{:else}
							<XCircle class="w-3.5 h-3.5" />
						{/if}
						{statusLabel(profile.subscription.status)}
					</span>
				</div>
				<div class="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.plan')}</p>
						<p class="text-sm font-semibold text-slate-900 capitalize">
							{profile.subscription.plan}
						</p>
					</div>
					{#if profile.subscription.current_period_end}
						<div>
							<p class="text-xs text-slate-400 mb-0.5">{$t('dash.next_billing')}</p>
							<p class="text-sm font-semibold text-slate-900">
								{formatDate(profile.subscription.current_period_end * 1000)}
							</p>
						</div>
					{/if}
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.quota_servers')}</p>
						<p class="text-sm font-semibold text-slate-900">
							{stores.length} / {profile.subscription.max_edge_servers}
						</p>
					</div>
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.stores_count')}</p>
						<p class="text-sm font-semibold text-slate-900">{stores.length}</p>
					</div>
				</div>
				<div class="mt-4">
					<button
						onclick={handleBillingPortal}
						disabled={billingLoading}
						class="inline-flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
					>
						<CreditCard class="w-4 h-4" />
						<span>{$t('dash.manage_billing')}</span>
					</button>
				</div>
			</div>

			<!-- Aggregate Overview (all stores combined) -->
			{#if aggregate && aggregate.stores_with_data > 0}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<div class="flex items-center gap-2 mb-4">
						<BarChart3 class="w-5 h-5 text-coral-500" />
						<h2 class="font-heading font-bold text-lg text-slate-900">
							{$t('stats.overview')}
						</h2>
						<span class="text-xs text-slate-400 bg-slate-100 px-2 py-0.5 rounded-full">
							{$t('stats.all_stores')} ({aggregate.stores_with_data})
						</span>
					</div>
					<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
						<div class="bg-coral-50/50 rounded-xl p-4">
							<div class="flex items-center gap-2 mb-1">
								<DollarSign class="w-4 h-4 text-coral-500" />
								<span class="text-xs text-slate-500">{$t('stats.total_sales')}</span>
							</div>
							<p class="text-lg font-bold text-slate-900">
								{formatCurrency(aggregate.total_sales)}
							</p>
						</div>
						<div class="bg-green-50/50 rounded-xl p-4">
							<div class="flex items-center gap-2 mb-1">
								<ShoppingBag class="w-4 h-4 text-green-500" />
								<span class="text-xs text-slate-500">{$t('stats.completed_orders')}</span>
							</div>
							<p class="text-lg font-bold text-slate-900">
								{aggregate.completed_orders}
								<span class="text-xs font-normal text-slate-400"
									>/ {aggregate.total_orders}</span
								>
							</p>
						</div>
						<div class="bg-blue-50/50 rounded-xl p-4">
							<div class="flex items-center gap-2 mb-1">
								<Wallet class="w-4 h-4 text-blue-500" />
								<span class="text-xs text-slate-500">{$t('stats.total_paid')}</span>
							</div>
							<p class="text-lg font-bold text-slate-900">
								{formatCurrency(aggregate.total_paid)}
							</p>
						</div>
						<div class="bg-orange-50/50 rounded-xl p-4">
							<div class="flex items-center gap-2 mb-1">
								<Percent class="w-4 h-4 text-orange-500" />
								<span class="text-xs text-slate-500">{$t('stats.total_discount')}</span>
							</div>
							<p class="text-lg font-bold text-slate-900">
								{formatCurrency(aggregate.total_discount)}
							</p>
						</div>
					</div>
					<div class="grid grid-cols-3 gap-3 mt-3">
						<div class="bg-slate-50 rounded-xl p-3">
							<p class="text-xs text-slate-400 mb-0.5">{$t('stats.void_orders')}</p>
							<p class="text-sm font-semibold text-slate-900">{aggregate.void_orders}</p>
						</div>
						<div class="bg-slate-50 rounded-xl p-3">
							<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_tax')}</p>
							<p class="text-sm font-semibold text-slate-900">
								{formatCurrency(aggregate.total_tax)}
							</p>
						</div>
						<div class="bg-slate-50 rounded-xl p-3">
							<p class="text-xs text-slate-400 mb-0.5">{$t('stats.total_unpaid')}</p>
							<p class="text-sm font-semibold text-slate-900">
								{formatCurrency(aggregate.total_unpaid)}
							</p>
						</div>
					</div>
				</div>
			{/if}

			<!-- Stores -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<h2 class="font-heading font-bold text-lg text-slate-900 mb-4">{$t('nav.stores')}</h2>
				{#if stores.length === 0}
					<div class="text-center py-8">
						<Server class="w-10 h-10 text-slate-300 mx-auto mb-3" />
						<p class="text-sm text-slate-500">{$t('dash.no_stores')}</p>
						<p class="text-xs text-slate-400 mt-1">{$t('dash.no_stores_hint')}</p>
					</div>
				{:else}
					<div class="space-y-3">
						{#each stores as store}
							<a
								href="/stores/{store.id}"
								class="flex items-center justify-between p-4 bg-slate-50 rounded-xl border border-slate-100 hover:border-slate-200 transition-colors duration-150"
							>
								<div class="flex items-center gap-3">
									<div class="w-10 h-10 bg-coral-100 rounded-lg flex items-center justify-center">
										<Server class="w-5 h-5 text-coral-600" />
									</div>
									<div>
										<p class="text-sm font-medium text-slate-900">
											{store.store_info?.name ?? `Store #${store.id}`}
										</p>
										<p class="text-xs text-slate-400">
											ID: {store.device_id.slice(0, 12)}...
										</p>
									</div>
								</div>
								<div class="flex items-center gap-3">
									<div class="text-right">
										<div class="inline-flex items-center gap-1 text-xs text-slate-500">
											<Clock class="w-3.5 h-3.5" />
											<span
												>{$t('dash.last_sync')}: {store.last_sync_at
													? timeAgo(store.last_sync_at)
													: $t('dash.never')}</span
											>
										</div>
									</div>
									<ArrowRight class="w-4 h-4 text-slate-400" />
								</div>
							</a>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</ConsoleLayout>
