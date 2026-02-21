<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, Package, Search } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getProducts, ApiError, type ProductEntry } from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	interface ProductSpec {
		name: string;
		price: number;
	}

	interface ProductData {
		name: string;
		category_name: string | null;
		price: number;
		is_active: boolean;
		specs: ProductSpec[];
		image_url: string | null;
	}

	let products = $state<ProductEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let search = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function parseProduct(entry: ProductEntry): ProductData {
		const d = entry.data as Record<string, unknown>;
		const specs = (d.specs as ProductSpec[]) ?? [];
		return {
			name: (d.name as string) ?? entry.source_id,
			category_name: (d.category_name as string) ?? null,
			price: (d.price as number) ?? 0,
			is_active: (d.is_active as boolean) ?? true,
			specs,
			image_url: (d.image_url as string) ?? null
		};
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return products;
		const q = search.toLowerCase();
		return products.filter((p) => {
			const data = parseProduct(p);
			return (
				data.name.toLowerCase().includes(q) ||
				(data.category_name?.toLowerCase().includes(q) ?? false)
			);
		});
	});

	// Group by category
	let grouped = $derived.by(() => {
		const groups = new Map<string, { product: ProductEntry; data: ProductData }[]>();
		for (const p of filtered) {
			const data = parseProduct(p);
			const cat = data.category_name ?? $t('products.no_category');
			if (!groups.has(cat)) groups.set(cat, []);
			groups.get(cat)!.push({ product: p, data });
		}
		return groups;
	});

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
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-3">
				<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
					<ArrowLeft class="w-5 h-5" />
				</a>
				<h1 class="font-heading text-xl font-bold text-slate-900">{$t('products.title')}</h1>
				{#if !loading && products.length > 0}
					<span class="text-xs text-slate-400 bg-slate-100 px-2 py-0.5 rounded-full"
						>{products.length}</span
					>
				{/if}
			</div>
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
					<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
						{#each items as { data }, i}
							<div
								class="flex items-center justify-between px-5 py-3 {i > 0
									? 'border-t border-slate-50'
									: ''}"
							>
								<div class="flex items-center gap-3 min-w-0">
									<div
										class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 {data.is_active
											? 'bg-coral-100'
											: 'bg-slate-100'}"
									>
										<Package
											class="w-4 h-4 {data.is_active
												? 'text-coral-600'
												: 'text-slate-400'}"
										/>
									</div>
									<div class="min-w-0">
										<p
											class="text-sm font-medium truncate {data.is_active
												? 'text-slate-900'
												: 'text-slate-400 line-through'}"
										>
											{data.name}
										</p>
										{#if data.specs.length > 0}
											<p class="text-xs text-slate-400 truncate">
												{data.specs.map((s) => `${s.name} ${formatCurrency(s.price)}`).join(' / ')}
											</p>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-3 shrink-0">
									{#if !data.is_active}
										<span
											class="text-xs px-2 py-0.5 rounded-full bg-slate-100 text-slate-500"
											>{$t('products.inactive')}</span
										>
									{/if}
									<span class="text-sm font-semibold text-slate-900"
										>{formatCurrency(data.price)}</span
									>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		{/if}
	</div>
</ConsoleLayout>
