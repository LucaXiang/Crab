<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Server, Clock, ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, ApiError, type StoreDetail } from '$lib/api';
	import { timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let stores = $state<StoreDetail[]>([]);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			stores = await getStores(token);
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
	<title>{$t('nav.stores')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8">
		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg
					class="animate-spin w-8 h-8 text-coral-500"
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
				>
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else}
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
											<span>{$t('dash.last_sync')}: {store.last_sync_at ? timeAgo(store.last_sync_at) : $t('dash.never')}</span>
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
