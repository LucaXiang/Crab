<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, Package, Search } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getProducts, ApiError, type CatalogProduct, type ProductSpec } from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let products = $state<CatalogProduct[]>([]);
	let loading = $state(true);
	let error = $state('');
	let search = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function getDisplayPrice(specs: ProductSpec[]): { label: string; isRange: boolean } {
		const active = specs.filter((s) => s.is_active);
		if (active.length === 0) return { label: formatCurrency(0), isRange: false };
		if (active.length === 1) return { label: formatCurrency(active[0].price), isRange: false };

		const defaultSpec = active.find((s) => s.is_default);
		if (defaultSpec) return { label: formatCurrency(defaultSpec.price), isRange: false };

		const prices = active.map((s) => s.price);
		const min = Math.min(...prices);
		const max = Math.max(...prices);
		if (min === max) return { label: formatCurrency(min), isRange: false };
		return { label: `${formatCurrency(min)} – ${formatCurrency(max)}`, isRange: true };
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return products;
		const q = search.toLowerCase();
		return products.filter(
			(p) =>
				p.name.toLowerCase().includes(q) ||
				(p.category_name?.toLowerCase().includes(q) ?? false)
		);
	});

	// Group by category
	let grouped = $derived.by(() => {
		const groups = new Map<string, CatalogProduct[]>();
		for (const p of filtered) {
			const cat = p.category_name ?? $t('products.no_category');
			if (!groups.has(cat)) groups.set(cat, []);
			groups.get(cat)!.push(p);
		}
		return groups;
	});

	let activeCount = $derived(products.filter((p) => p.is_active).length);
	let inactiveCount = $derived(products.length - activeCount);

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			products = await getProducts(token, storeId);
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
	<title>{$t('products.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-3">
				<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
					<ArrowLeft class="w-5 h-5" />
				</a>
				<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('products.title')}</h1>
			</div>
			{#if !loading && products.length > 0}
				<div class="flex items-center gap-2 text-xs text-slate-400">
					<span class="bg-slate-100 px-2 py-0.5 rounded-full">{activeCount} {$t('products.active')}</span>
					{#if inactiveCount > 0}
						<span class="bg-slate-50 px-2 py-0.5 rounded-full">{inactiveCount} {$t('products.inactive')}</span>
					{/if}
				</div>
			{/if}
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
		{:else if products.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<Package class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('products.empty')}</p>
			</div>
		{:else}
			<!-- Search -->
			<div class="relative">
				<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
				<input
					type="text"
					bind:value={search}
					placeholder={$t('products.search')}
					class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-xl text-sm focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
				/>
			</div>

			<!-- Grouped products -->
			{#each [...grouped.entries()] as [category, items]}
				<div class="space-y-2">
					<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider px-1">
						{category}
						<span class="text-slate-300">({items.length})</span>
					</h3>
					<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden divide-y divide-slate-100">
						{#each items as p}
							{@const priceInfo = getDisplayPrice(p.specs)}
							<div class="flex items-center justify-between px-5 py-3.5">
								<div class="flex items-center gap-3 min-w-0">
									<div
										class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0 {p.is_active
											? 'bg-coral-50'
											: 'bg-slate-50'}"
									>
										<Package class="w-4 h-4 {p.is_active ? 'text-coral-500' : 'text-slate-300'}" />
									</div>
									<div class="min-w-0 space-y-0.5">
										<p
											class="text-sm font-medium truncate {p.is_active
												? 'text-slate-900'
												: 'text-slate-400 line-through'}"
										>
											{p.name}
										</p>
										{#if p.specs.filter((s) => s.is_active).length > 1}
											<p class="text-xs text-slate-400 truncate">
												{p.specs.filter((s) => s.is_active).map((s) => `${s.name} ${formatCurrency(s.price)}`).join(' · ')}
											</p>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2 shrink-0 ml-4">
									{#if !p.is_active}
										<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-400">
											{$t('products.inactive')}
										</span>
									{/if}
									{#if p.tax_rate > 0}
										<span class="text-[10px] text-slate-300">{p.tax_rate}%</span>
									{/if}
									<span class="text-sm font-semibold {p.is_active ? 'text-slate-900' : 'text-slate-400'}">
										{priceInfo.label}
									</span>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		{/if}
	</div>
</ConsoleLayout>
