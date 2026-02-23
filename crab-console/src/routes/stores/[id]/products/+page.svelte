<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, Package, Search, Plus, Pencil, Trash2, X, GripVertical } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getProducts,
		getCategories,
		getTags,
		createProduct,
		updateProduct,
		deleteProduct,
		ApiError,
		type CatalogProduct,
		type CatalogCategory,
		type CatalogTag,
		type ProductSpec,
		type ProductSpecInput,
		type ProductCreate,
		type ProductUpdate
	} from '$lib/api';
	import { formatCurrency } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let products = $state<CatalogProduct[]>([]);
	let categories = $state<CatalogCategory[]>([]);
	let allTags = $state<CatalogTag[]>([]);
	let loading = $state(true);
	let error = $state('');
	let search = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	// Modal
	let showModal = $state(false);
	let editing = $state<CatalogProduct | null>(null);
	let saving = $state(false);
	let modalError = $state('');

	let form = $state({
		name: '',
		category_id: 0,
		tax_rate: 0,
		sort_order: 0,
		is_active: true,
		tag_ids: [] as number[],
		specs: [{ name: 'Default', price: 0, display_order: 0, is_default: true, is_active: true, is_root: true }] as ProductSpecInput[]
	});

	let deleting = $state<CatalogProduct | null>(null);
	let deleteLoading = $state(false);

	function getDisplayPrice(specs: ProductSpec[]): string {
		const active = specs.filter((s) => s.is_active);
		if (active.length === 0) return formatCurrency(0);
		if (active.length === 1) return formatCurrency(active[0].price);
		const defaultSpec = active.find((s) => s.is_default);
		if (defaultSpec) return formatCurrency(defaultSpec.price);
		const prices = active.map((s) => s.price);
		const min = Math.min(...prices);
		const max = Math.max(...prices);
		if (min === max) return formatCurrency(min);
		return `${formatCurrency(min)} – ${formatCurrency(max)}`;
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return products;
		const q = search.toLowerCase();
		return products.filter(
			(p) => p.name.toLowerCase().includes(q) || (p.category_name?.toLowerCase().includes(q) ?? false)
		);
	});

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

	function openCreate() {
		editing = null;
		form = {
			name: '',
			category_id: categories[0]?.source_id ?? 0,
			tax_rate: 0,
			sort_order: 0,
			is_active: true,
			tag_ids: [],
			specs: [{ name: 'Default', price: 0, display_order: 0, is_default: true, is_active: true, is_root: true }]
		};
		modalError = '';
		showModal = true;
	}

	function openEdit(p: CatalogProduct) {
		editing = p;
		form = {
			name: p.name,
			category_id: p.category_source_id,
			tax_rate: p.tax_rate,
			sort_order: p.sort_order,
			is_active: p.is_active,
			tag_ids: [...p.tag_ids],
			specs: p.specs.map((s) => ({
				name: s.name,
				price: s.price,
				display_order: s.display_order,
				is_default: s.is_default,
				is_active: s.is_active,
				is_root: s.is_root
			}))
		};
		modalError = '';
		showModal = true;
	}

	function addSpec() {
		form.specs = [...form.specs, { name: '', price: 0, display_order: form.specs.length, is_default: false, is_active: true, is_root: false }];
	}

	function removeSpec(idx: number) {
		form.specs = form.specs.filter((_, i) => i !== idx);
	}

	function setDefaultSpec(idx: number) {
		form.specs = form.specs.map((s, i) => ({ ...s, is_default: i === idx }));
	}

	function toggleTag(tagId: number) {
		if (form.tag_ids.includes(tagId)) {
			form.tag_ids = form.tag_ids.filter((id) => id !== tagId);
		} else {
			form.tag_ids = [...form.tag_ids, tagId];
		}
	}

	async function handleSave() {
		if (!form.name.trim() || form.specs.length === 0) return;
		saving = true;
		modalError = '';
		try {
			if (editing) {
				const data: ProductUpdate = {
					name: form.name,
					category_id: form.category_id,
					tax_rate: form.tax_rate,
					sort_order: form.sort_order,
					is_active: form.is_active,
					tags: form.tag_ids,
					specs: form.specs
				};
				const res = await updateProduct(token, storeId, editing.source_id, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			} else {
				const data: ProductCreate = {
					name: form.name,
					category_id: form.category_id,
					tax_rate: form.tax_rate,
					sort_order: form.sort_order,
					tags: form.tag_ids,
					specs: form.specs
				};
				const res = await createProduct(token, storeId, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			}
			showModal = false;
			await reload();
		} catch (err) {
			modalError = err instanceof ApiError ? err.message : $t('catalog.error');
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!deleting) return;
		deleteLoading = true;
		try {
			const res = await deleteProduct(token, storeId, deleting.source_id);
			if (!res.success) { error = res.error ?? $t('catalog.error'); }
			deleting = null;
			await reload();
		} catch (err) {
			error = err instanceof ApiError ? err.message : $t('catalog.error');
			deleting = null;
		} finally {
			deleteLoading = false;
		}
	}

	async function reload() {
		try {
			products = await getProducts(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('catalog.error');
		}
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }
		try {
			[products, categories, allTags] = await Promise.all([
				getProducts(token, storeId),
				getCategories(token, storeId),
				getTags(token, storeId)
			]);
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
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-3">
				<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
					<ArrowLeft class="w-5 h-5" />
				</a>
				<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('products.title')}</h1>
			</div>
			<div class="flex items-center gap-3">
				{#if !loading && products.length > 0}
					<div class="hidden md:flex items-center gap-2 text-xs text-slate-400">
						<span class="bg-slate-100 px-2 py-0.5 rounded-full">{activeCount} {$t('products.active')}</span>
						{#if inactiveCount > 0}
							<span class="bg-slate-50 px-2 py-0.5 rounded-full">{inactiveCount} {$t('products.inactive')}</span>
						{/if}
					</div>
				{/if}
				<button
					onclick={openCreate}
					class="bg-coral-600 text-white px-3 py-1.5 rounded-lg text-sm font-medium hover:bg-coral-700 flex items-center gap-1.5 cursor-pointer"
				>
					<Plus class="w-4 h-4" />
					{$t('products.new')}
				</button>
			</div>
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
			<div class="relative">
				<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
				<input type="text" bind:value={search} placeholder={$t('products.search')}
					class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-xl text-sm focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500" />
			</div>

			{#each [...grouped.entries()] as [category, items]}
				<div class="space-y-2">
					<h3 class="text-xs font-semibold text-slate-400 uppercase tracking-wider px-1">
						{category} <span class="text-slate-300">({items.length})</span>
					</h3>
					<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden divide-y divide-slate-100">
						{#each items as p}
							<div class="flex items-center justify-between px-5 py-3.5 group">
								<div class="flex items-center gap-3 min-w-0">
									<div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0 {p.is_active ? 'bg-coral-50' : 'bg-slate-50'}">
										<Package class="w-4 h-4 {p.is_active ? 'text-coral-500' : 'text-slate-300'}" />
									</div>
									<div class="min-w-0 space-y-0.5">
										<p class="text-sm font-medium truncate {p.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}">{p.name}</p>
										{#if p.specs.filter((s) => s.is_active).length > 1}
											<p class="text-xs text-slate-400 truncate">
												{p.specs.filter((s) => s.is_active).map((s) => `${s.name} ${formatCurrency(s.price)}`).join(' · ')}
											</p>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2 shrink-0 ml-4">
									{#if !p.is_active}
										<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-400">{$t('products.inactive')}</span>
									{/if}
									{#if p.tax_rate > 0}
										<span class="text-[10px] text-slate-300">{p.tax_rate}%</span>
									{/if}
									<span class="text-sm font-semibold {p.is_active ? 'text-slate-900' : 'text-slate-400'}">{getDisplayPrice(p.specs)}</span>
									<div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
										<button onclick={() => openEdit(p)} class="p-1.5 text-slate-400 hover:text-slate-600 rounded-lg hover:bg-slate-50 cursor-pointer">
											<Pencil class="w-3.5 h-3.5" />
										</button>
										<button onclick={() => (deleting = p)} class="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 cursor-pointer">
											<Trash2 class="w-3.5 h-3.5" />
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		{/if}
	</div>
</ConsoleLayout>

<!-- Create/Edit Product Modal -->
{#if showModal}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-start justify-center p-4 overflow-y-auto" onclick={() => (showModal = false)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-lg my-8" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between px-6 py-4 border-b border-slate-100">
				<h2 class="font-heading font-semibold text-slate-900">{editing ? $t('products.edit') : $t('products.new')}</h2>
				<button onclick={() => (showModal = false)} class="text-slate-400 hover:text-slate-600 cursor-pointer">
					<X class="w-5 h-5" />
				</button>
			</div>
			<div class="p-6 space-y-5 max-h-[70vh] overflow-y-auto">
				<!-- Name -->
				<div class="space-y-1">
					<label for="prod-name" class="block text-sm font-medium text-slate-700">{$t('catalog.name')}</label>
					<input type="text" id="prod-name" bind:value={form.name}
						class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
				</div>

				<!-- Category + Tax -->
				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-1">
						<label for="prod-cat" class="block text-sm font-medium text-slate-700">{$t('products.category')}</label>
						<select id="prod-cat" bind:value={form.category_id}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-coral-500 focus:ring-coral-500">
							{#each categories as cat}
								<option value={cat.source_id}>{cat.name}</option>
							{/each}
						</select>
					</div>
					<div class="space-y-1">
						<label for="prod-tax" class="block text-sm font-medium text-slate-700">{$t('products.tax_rate')}</label>
						<input type="number" id="prod-tax" bind:value={form.tax_rate} min="0" max="100"
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
					</div>
				</div>

				<!-- Tags -->
				{#if allTags.length > 0}
					<div class="space-y-2">
						<span class="block text-sm font-medium text-slate-700">{$t('nav.tags')}</span>
						<div class="flex flex-wrap gap-2">
							{#each allTags.filter((t) => t.is_active) as tag}
								<button
									onclick={() => toggleTag(tag.source_id)}
									class="px-2.5 py-1 text-xs rounded-full border transition-colors cursor-pointer {form.tag_ids.includes(tag.source_id)
										? 'border-coral-300 bg-coral-50 text-coral-700'
										: 'border-slate-200 bg-white text-slate-600 hover:bg-slate-50'}"
								>
									<span class="inline-block w-2 h-2 rounded-full mr-1" style="background-color: {tag.color}"></span>
									{tag.name}
								</button>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Active toggle (edit only) -->
				{#if editing}
					<label class="flex items-center gap-2 cursor-pointer">
						<input type="checkbox" bind:checked={form.is_active} class="rounded border-slate-300 text-coral-500 focus:ring-coral-500" />
						<span class="text-sm text-slate-700">{$t('catalog.active')}</span>
					</label>
				{/if}

				<!-- Specs -->
				<div class="space-y-3">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium text-slate-700">{$t('products.specs')}</span>
						<button onclick={addSpec} class="text-xs text-coral-600 hover:text-coral-700 font-medium cursor-pointer">
							+ {$t('products.add_spec')}
						</button>
					</div>
					{#each form.specs as spec, idx}
						<div class="flex items-center gap-2 p-3 bg-slate-50 rounded-lg">
							<div class="flex-1 grid grid-cols-2 gap-2">
								<input type="text" bind:value={spec.name} placeholder={$t('products.spec_name')}
									class="rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm" />
								<input type="number" bind:value={spec.price} placeholder="0.00" step="0.01" min="0"
									class="rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm" />
							</div>
							<button onclick={() => setDefaultSpec(idx)}
								class="text-xs px-2 py-1 rounded {spec.is_default ? 'bg-coral-100 text-coral-700' : 'bg-white text-slate-400 hover:text-slate-600'} cursor-pointer border border-slate-200">
								{$t('products.default_spec')}
							</button>
							{#if form.specs.length > 1}
								<button onclick={() => removeSpec(idx)} class="text-slate-400 hover:text-red-500 cursor-pointer">
									<X class="w-4 h-4" />
								</button>
							{/if}
						</div>
					{/each}
				</div>

				{#if modalError}
					<div class="text-sm text-red-600">{modalError}</div>
				{/if}
			</div>
			<div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-100">
				<button onclick={() => (showModal = false)} class="px-4 py-2 text-sm text-slate-600 hover:text-slate-800 cursor-pointer">{$t('catalog.cancel')}</button>
				<button onclick={handleSave} disabled={saving || !form.name.trim() || form.specs.length === 0}
					class="bg-coral-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-coral-700 disabled:opacity-50 cursor-pointer">
					{saving ? $t('catalog.saving') : $t('catalog.save')}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirm -->
{#if deleting}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-center justify-center p-4" onclick={() => (deleting = null)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-sm" onclick={(e) => e.stopPropagation()}>
			<div class="p-6 space-y-3">
				<h3 class="font-heading font-semibold text-slate-900">{$t('catalog.confirm_delete')}</h3>
				<p class="text-sm text-slate-500">{$t('catalog.confirm_delete_desc')}</p>
				<p class="text-sm font-medium text-slate-700">{deleting.name}</p>
			</div>
			<div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-100">
				<button onclick={() => (deleting = null)} class="px-4 py-2 text-sm text-slate-600 hover:text-slate-800 cursor-pointer">{$t('catalog.cancel')}</button>
				<button onclick={handleDelete} disabled={deleteLoading}
					class="bg-red-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-red-700 disabled:opacity-50 cursor-pointer">
					{deleteLoading ? $t('catalog.deleting') : $t('catalog.delete')}
				</button>
			</div>
		</div>
	</div>
{/if}
