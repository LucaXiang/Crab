<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Tag, Plus, Pencil, Trash2, X } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getTags,
		createTag,
		updateTag,
		deleteTag,
		ApiError,
		type CatalogTag,
		type TagCreate,
		type TagUpdate
	} from '$lib/api';
	const storeId = Number(page.params.id);

	let tags = $state<CatalogTag[]>([]);
	let loading = $state(true);
	let error = $state('');
	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	// Modal state
	let showModal = $state(false);
	let editing = $state<CatalogTag | null>(null);
	let saving = $state(false);
	let modalError = $state('');

	let form = $state({ name: '', color: '#6366f1', display_order: 0 });

	// Delete confirm
	let deleting = $state<CatalogTag | null>(null);
	let deleteLoading = $state(false);

	function openCreate() {
		editing = null;
		form = { name: '', color: '#6366f1', display_order: 0 };
		modalError = '';
		showModal = true;
	}

	function openEdit(tag: CatalogTag) {
		editing = tag;
		form = { name: tag.name, color: tag.color || '#6366f1', display_order: tag.display_order };
		modalError = '';
		showModal = true;
	}

	async function handleSave() {
		if (!form.name.trim()) return;
		saving = true;
		modalError = '';
		try {
			if (editing) {
				const data: TagUpdate = {
					name: form.name,
					color: form.color,
					display_order: form.display_order
				};
				const res = await updateTag(token, storeId, editing.source_id, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			} else {
				const data: TagCreate = {
					name: form.name,
					color: form.color,
					display_order: form.display_order
				};
				const res = await createTag(token, storeId, data);
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
			const res = await deleteTag(token, storeId, deleting.source_id);
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
			tags = await getTags(token, storeId);
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
			tags = await getTags(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('tags.title')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-3xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('tags.title')}</h1>
		<button
				onclick={openCreate}
				class="bg-coral-600 text-white px-3 py-1.5 rounded-lg text-sm font-medium hover:bg-coral-700 flex items-center gap-1.5 cursor-pointer"
			>
				<Plus class="w-4 h-4" />
				{$t('tags.new')}
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
		{:else if tags.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<Tag class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('tags.empty')}</p>
			</div>
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden divide-y divide-slate-100">
				{#each tags as tag}
					<div class="flex items-center justify-between px-5 py-3.5">
						<div class="flex items-center gap-3 min-w-0">
							<div class="w-6 h-6 rounded-full shrink-0 border border-slate-200" style="background-color: {tag.color || '#6366f1'}"></div>
							<span class="text-sm font-medium {tag.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}">{tag.name}</span>
							{#if tag.is_system}
								<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-50 text-blue-500">{$t('tags.system')}</span>
							{/if}
							{#if !tag.is_active}
								<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-400">{$t('catalog.inactive')}</span>
							{/if}
						</div>
						{#if !tag.is_system}
							<div class="flex items-center gap-1 shrink-0 ml-4">
								<button onclick={() => openEdit(tag)} class="p-1.5 text-slate-400 hover:text-slate-600 rounded-lg hover:bg-slate-50 cursor-pointer">
									<Pencil class="w-3.5 h-3.5" />
								</button>
								<button onclick={() => (deleting = tag)} class="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 cursor-pointer">
									<Trash2 class="w-3.5 h-3.5" />
								</button>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>

<!-- Create/Edit Modal -->
{#if showModal}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-center justify-center p-4" onclick={() => (showModal = false)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-md" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between px-6 py-4 border-b border-slate-100">
				<h2 class="font-heading font-semibold text-slate-900">{editing ? $t('tags.edit') : $t('tags.new')}</h2>
				<button onclick={() => (showModal = false)} class="text-slate-400 hover:text-slate-600 cursor-pointer">
					<X class="w-5 h-5" />
				</button>
			</div>
			<div class="p-6 space-y-4">
				<div class="space-y-1">
					<label for="tag-name" class="block text-sm font-medium text-slate-700">{$t('catalog.name')}</label>
					<input type="text" id="tag-name" bind:value={form.name}
						class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
				</div>
				<div class="space-y-1">
					<label for="tag-color" class="block text-sm font-medium text-slate-700">{$t('tags.color')}</label>
					<div class="flex items-center gap-3">
						<input type="color" id="tag-color" bind:value={form.color} class="w-10 h-10 rounded-lg border border-slate-300 cursor-pointer" />
						<input type="text" bind:value={form.color} class="flex-1 rounded-lg border border-slate-300 px-3 py-2 text-sm font-mono" />
					</div>
				</div>
				<div class="space-y-1">
					<label for="tag-order" class="block text-sm font-medium text-slate-700">{$t('catalog.sort_order')}</label>
					<input type="number" id="tag-order" bind:value={form.display_order}
						class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
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
