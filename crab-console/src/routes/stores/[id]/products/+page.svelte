<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, Package } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getProducts, ApiError, type ProductEntry } from '$lib/api';
	import { formatDateTime } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let products = $state<ProductEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			products = await getProducts(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('products.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<h1 class="font-heading text-xl font-bold text-slate-900">{$t('products.title')}</h1>
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
		{:else if products.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<Package class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('products.empty')}</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each products as product}
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="flex items-center justify-between mb-2">
							<span class="text-sm font-medium text-slate-900">{product.data?.name ?? product.source_id}</span>
							<span class="text-xs text-slate-400">{$t('products.synced')}: {formatDateTime(product.synced_at)}</span>
						</div>
						<pre class="text-xs text-slate-600 bg-slate-50 rounded-lg p-3 overflow-x-auto">{JSON.stringify(product.data, null, 2)}</pre>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</ConsoleLayout>
