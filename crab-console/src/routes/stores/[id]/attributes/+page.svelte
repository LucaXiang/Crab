<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { SlidersHorizontal, Plus, Pencil, Trash2, X } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getAttributes,
		createAttribute,
		updateAttribute,
		deleteAttribute,
		ApiError,
		type CatalogAttribute,
		type AttributeOptionInput,
		type AttributeCreate,
		type AttributeUpdate
	} from '$lib/api';
	import { formatCurrency } from '$lib/format';

	const storeId = Number(page.params.id);

	let attributes = $state<CatalogAttribute[]>([]);
	let loading = $state(true);
	let error = $state('');
	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	let showModal = $state(false);
	let editing = $state<CatalogAttribute | null>(null);
	let saving = $state(false);
	let modalError = $state('');

	let form = $state({
		name: '',
		is_multi_select: false,
		max_selections: 0,
		display_order: 0,
		options: [] as AttributeOptionInput[]
	});

	let deleting = $state<CatalogAttribute | null>(null);
	let deleteLoading = $state(false);

	function openCreate() {
		editing = null;
		form = { name: '', is_multi_select: false, max_selections: 0, display_order: 0, options: [] };
		modalError = '';
		showModal = true;
	}

	function openEdit(attr: CatalogAttribute) {
		editing = attr;
		form = {
			name: attr.name,
			is_multi_select: attr.is_multi_select,
			max_selections: attr.max_selections ?? 0,
			display_order: attr.display_order,
			options: attr.options.map((o) => ({
				name: o.name,
				price_modifier: o.price_modifier,
				display_order: o.display_order,
				enable_quantity: o.enable_quantity,
				max_quantity: o.max_quantity ?? undefined
			}))
		};
		modalError = '';
		showModal = true;
	}

	function addOption() {
		form.options = [...form.options, { name: '', price_modifier: 0, display_order: form.options.length, enable_quantity: false }];
	}

	function removeOption(idx: number) {
		form.options = form.options.filter((_, i) => i !== idx);
	}

	async function handleSave() {
		if (!form.name.trim()) return;
		saving = true;
		modalError = '';
		try {
			if (editing) {
				const data: AttributeUpdate = {
					name: form.name,
					is_multi_select: form.is_multi_select,
					max_selections: form.max_selections || undefined,
					display_order: form.display_order,
					options: form.options
				};
				const res = await updateAttribute(token, storeId, editing.source_id, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			} else {
				const data: AttributeCreate = {
					name: form.name,
					is_multi_select: form.is_multi_select,
					max_selections: form.max_selections || undefined,
					display_order: form.display_order,
					options: form.options
				};
				const res = await createAttribute(token, storeId, data);
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
			const res = await deleteAttribute(token, storeId, deleting.source_id);
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
			attributes = await getAttributes(token, storeId);
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
			attributes = await getAttributes(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('attributes.title')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-3xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('attributes.title')}</h1>
		<button onclick={openCreate}
				class="bg-coral-600 text-white px-3 py-1.5 rounded-lg text-sm font-medium hover:bg-coral-700 flex items-center gap-1.5 cursor-pointer">
				<Plus class="w-4 h-4" />
				{$t('attributes.new')}
			</button>
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
		{:else if attributes.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<SlidersHorizontal class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('attributes.empty')}</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each attributes as attr}
					<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
						<div class="flex items-center justify-between px-5 py-3.5">
							<div class="min-w-0">
								<p class="text-sm font-medium {attr.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}">{attr.name}</p>
								<div class="flex items-center gap-2 mt-0.5">
									{#if attr.is_multi_select}
										<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-50 text-blue-500">{$t('attributes.multi_select')}</span>
									{/if}
									{#if !attr.is_active}
										<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-400">{$t('catalog.inactive')}</span>
									{/if}
									<span class="text-xs text-slate-400">{attr.options.length} {$t('attributes.options').toLowerCase()}</span>
								</div>
							</div>
							<div class="flex items-center gap-1 shrink-0 ml-4">
								<button onclick={() => openEdit(attr)} class="p-1.5 text-slate-400 hover:text-slate-600 rounded-lg hover:bg-slate-50 cursor-pointer">
									<Pencil class="w-3.5 h-3.5" />
								</button>
								<button onclick={() => (deleting = attr)} class="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 cursor-pointer">
									<Trash2 class="w-3.5 h-3.5" />
								</button>
							</div>
						</div>
						{#if attr.options.length > 0}
							<div class="border-t border-slate-100 px-5 py-2">
								<div class="flex flex-wrap gap-1.5">
									{#each attr.options.filter((o) => o.is_active) as opt}
										<span class="text-xs px-2 py-0.5 rounded-full bg-slate-50 text-slate-600">
											{opt.name}
											{#if opt.price_modifier !== 0}
												<span class="text-slate-400 ml-0.5">{opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}</span>
											{/if}
										</span>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>

<!-- Create/Edit Modal -->
{#if showModal}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-start justify-center p-4 overflow-y-auto" onclick={() => (showModal = false)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-lg my-8" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between px-6 py-4 border-b border-slate-100">
				<h2 class="font-heading font-semibold text-slate-900">{editing ? $t('attributes.edit') : $t('attributes.new')}</h2>
				<button onclick={() => (showModal = false)} class="text-slate-400 hover:text-slate-600 cursor-pointer">
					<X class="w-5 h-5" />
				</button>
			</div>
			<div class="p-6 space-y-5 max-h-[70vh] overflow-y-auto">
				<div class="space-y-1">
					<label for="attr-name" class="block text-sm font-medium text-slate-700">{$t('catalog.name')}</label>
					<input type="text" id="attr-name" bind:value={form.name}
						class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
				</div>

				<div class="flex items-center gap-6">
					<label class="flex items-center gap-2 cursor-pointer">
						<input type="checkbox" bind:checked={form.is_multi_select} class="rounded border-slate-300 text-coral-500 focus:ring-coral-500" />
						<span class="text-sm text-slate-700">{$t('attributes.multi_select')}</span>
					</label>
					{#if form.is_multi_select}
						<div class="flex items-center gap-2">
							<label for="attr-max" class="text-sm text-slate-700">{$t('attributes.max_selections')}</label>
							<input type="number" id="attr-max" bind:value={form.max_selections} min="0" max="20"
								class="w-16 rounded-lg border border-slate-300 px-2 py-1 text-sm" />
						</div>
					{/if}
				</div>

				<!-- Options -->
				<div class="space-y-3">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium text-slate-700">{$t('attributes.options')}</span>
						<button onclick={addOption} class="text-xs text-coral-600 hover:text-coral-700 font-medium cursor-pointer">
							+ {$t('attributes.add_option')}
						</button>
					</div>
					{#each form.options as opt, idx}
						<div class="flex items-center gap-2 p-3 bg-slate-50 rounded-lg">
							<div class="flex-1 grid grid-cols-2 gap-2">
								<input type="text" bind:value={opt.name} placeholder={$t('attributes.option_name')}
									class="rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm" />
								<input type="number" bind:value={opt.price_modifier} placeholder="0.00" step="0.01"
									class="rounded-lg border border-slate-300 px-2.5 py-1.5 text-sm" />
							</div>
							<button onclick={() => removeOption(idx)} class="text-slate-400 hover:text-red-500 cursor-pointer">
								<X class="w-4 h-4" />
							</button>
						</div>
					{/each}
				</div>

				{#if modalError}
					<div class="text-sm text-red-600">{modalError}</div>
				{/if}
			</div>
			<div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-100">
				<button onclick={() => (showModal = false)} class="px-4 py-2 text-sm text-slate-600 hover:text-slate-800 cursor-pointer">{$t('catalog.cancel')}</button>
				<button onclick={handleSave} disabled={saving || !form.name.trim()}
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
