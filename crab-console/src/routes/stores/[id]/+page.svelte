<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		ArrowLeft,
		ShoppingBag,
		BarChart3,
		ScrollText,
		Package,
		Terminal,
		Server,
		Clock
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, ApiError, type StoreDetail } from '$lib/api';
	import { formatDate, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let store = $state<StoreDetail | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	const storeId = Number(page.params.id);

	const subNav = [
		{ key: 'nav.orders', href: `/stores/${storeId}/orders`, icon: ShoppingBag },
		{ key: 'nav.overview', href: `/stores/${storeId}/overview`, icon: BarChart3 },
		{ key: 'nav.daily_report', href: `/stores/${storeId}/stats`, icon: ScrollText },
		{ key: 'nav.products', href: `/stores/${storeId}/products`, icon: Package },
		{ key: 'nav.commands', href: `/stores/${storeId}/commands`, icon: Terminal }
	];

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			const stores = await getStores(token);
			store = stores.find((s) => s.id === storeId) ?? null;
			if (!store) error = 'Store not found';
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('store.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<a href="/" class="inline-flex items-center gap-1.5 text-sm text-slate-500 hover:text-slate-700">
			<ArrowLeft class="w-4 h-4" />
			<span>{$t('store.back')}</span>
		</a>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if store}
			<!-- Store header -->
			<div class="bg-white rounded-2xl border border-slate-200 p-4 md:p-6">
				<div class="flex flex-col md:flex-row md:items-center gap-4">
					<div class="flex items-center gap-4">
						<div class="w-12 h-12 bg-coral-100 rounded-xl flex items-center justify-center shrink-0">
							<Server class="w-6 h-6 text-coral-600" />
						</div>
						<div class="md:hidden">
							<h1 class="font-heading text-xl font-bold text-slate-900">
								{store.store_info?.name ?? `Store #${store.id}`}
							</h1>
						</div>
					</div>
					
					<div class="flex-1 min-w-0">
						<h1 class="hidden md:block font-heading text-xl font-bold text-slate-900">
							{store.store_info?.name ?? `Store #${store.id}`}
						</h1>
						<div class="flex flex-col md:flex-row md:items-center gap-1 md:gap-4 text-xs text-slate-400 mt-1">
							<span class="truncate">{$t('store.device_id')}: {store.device_id.slice(0, 16)}...</span>
							<span class="hidden md:inline text-slate-300">•</span>
							<span class="inline-flex items-center gap-1">
								<Clock class="w-3 h-3" />
								{$t('dash.last_sync')}: {store.last_sync_at ? timeAgo(store.last_sync_at) : $t('dash.never')}
							</span>
							<span class="hidden md:inline text-slate-300">•</span>
							<span>{$t('store.registered')}: {formatDate(store.registered_at)}</span>
						</div>
					</div>
				</div>
			</div>

			<!-- Sub-navigation -->
			<div class="grid grid-cols-2 md:grid-cols-5 gap-3">
				{#each subNav as item}
					<a
						href={item.href}
						class="bg-white rounded-xl border border-slate-200 p-5 hover:border-coral-200 hover:bg-coral-50/30 transition-colors duration-150 flex flex-col items-center gap-2 text-center"
					>
						<item.icon class="w-6 h-6 text-coral-500" />
						<span class="text-sm font-medium text-slate-700">{$t(item.key)}</span>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</ConsoleLayout>
