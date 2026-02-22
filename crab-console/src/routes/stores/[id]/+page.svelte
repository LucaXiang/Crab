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
		Clock,
		Pencil,
		Save
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, updateStore, ApiError, type StoreDetail } from '$lib/api';
	import { formatDate, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let store = $state<StoreDetail | null>(null);
	let loading = $state(true);
	let error = $state('');
	
	let editMode = $state(false);
	let saving = $state(false);
	let editForm = $state({
		name: '',
		address: '',
		phone: '',
		nif: '',
		email: '',
		website: '',
		business_day_cutoff: ''
	});

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	const storeId = Number(page.params.id);

	const subNav = [
    { key: 'nav.orders', href: `/stores/${storeId}/orders`, icon: ShoppingBag },
    { key: 'nav.overview', href: `/stores/${storeId}/overview`, icon: BarChart3 },
    { key: 'nav.daily_report', href: `/stores/${storeId}/stats`, icon: ScrollText },
    { key: 'nav.products', href: `/stores/${storeId}/products`, icon: Package }
  ];

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			const stores = await getStores(token);
			store = stores.find((s) => s.id === storeId) ?? null;
			if (!store) error = 'Store not found';
			else {
				// Initialize form
				editForm = {
					name: store.name ?? store.store_info?.name as string ?? '',
					address: store.address ?? store.store_info?.address as string ?? '',
					phone: store.phone ?? store.store_info?.phone as string ?? '',
					nif: store.nif ?? store.store_info?.nif as string ?? '',
					email: store.email ?? store.store_info?.email as string ?? '',
					website: store.website ?? store.store_info?.website as string ?? '',
					business_day_cutoff: store.business_day_cutoff ?? store.store_info?.business_day_cutoff as string ?? ''
				};
			}
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});

	async function handleSave(e: Event) {
		e.preventDefault();
		if (!store || !token) return;
		
		saving = true;
		try {
			await updateStore(
				token, 
				store.id, 
				editForm.name, 
				editForm.address, 
				editForm.phone,
				editForm.nif,
				editForm.email,
				editForm.website,
				editForm.business_day_cutoff
			);
			
			// Update local state
			store.name = editForm.name;
			store.address = editForm.address;
			store.phone = editForm.phone;
			store.nif = editForm.nif;
			store.email = editForm.email;
			store.website = editForm.website;
			store.business_day_cutoff = editForm.business_day_cutoff;
			
			editMode = false;
		} catch (err) {
			console.error(err);
			alert($t('auth.error_generic'));
		} finally {
			saving = false;
		}
	}
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
								{store.name ?? store.store_info?.name ?? `Store #${store.id}`}
							</h1>
						</div>
					</div>
					
					<div class="flex-1 min-w-0">
						<h1 class="hidden md:block font-heading text-xl font-bold text-slate-900">
							{store.name ?? store.store_info?.name ?? `Store #${store.id}`}
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

			<!-- Store Details -->
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<div class="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
					<h2 class="font-heading font-semibold text-slate-900">{$t('store.title')}</h2>
					<button
						class="text-sm font-medium text-coral-600 hover:text-coral-700 disabled:opacity-50 flex items-center gap-1"
						onclick={() => {
							if (editMode) {
								// Cancel
								editMode = false;
								if (store) {
									editForm = {
										name: store.name ?? store.store_info?.name as string ?? '',
										address: store.address ?? store.store_info?.address as string ?? '',
										phone: store.phone ?? store.store_info?.phone as string ?? '',
										nif: store.nif ?? store.store_info?.nif as string ?? '',
										email: store.email ?? store.store_info?.email as string ?? '',
										website: store.website ?? store.store_info?.website as string ?? '',
										business_day_cutoff: store.business_day_cutoff ?? store.store_info?.business_day_cutoff as string ?? ''
									};
								}
							} else {
								editMode = true;
							}
						}}
						disabled={saving}
					>
						{#if editMode}
							{$t('store.back')}
						{:else}
							<Pencil class="w-3.5 h-3.5" />
							{$t('store.edit')}
						{/if}
					</button>
				</div>
				
				<div class="p-6">
					{#if editMode}
						<form class="grid grid-cols-1 md:grid-cols-2 gap-6" onsubmit={handleSave}>
							<div class="space-y-1">
								<label for="name" class="block text-sm font-medium text-slate-700">{$t('store.name')}</label>
								<input
									type="text"
									id="name"
									bind:value={editForm.name}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>
							
							<div class="space-y-1">
								<label for="nif" class="block text-sm font-medium text-slate-700">{$t('store.nif')}</label>
								<input
									type="text"
									id="nif"
									bind:value={editForm.nif}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>

							<div class="space-y-1">
								<label for="phone" class="block text-sm font-medium text-slate-700">{$t('store.phone')}</label>
								<input
									type="tel"
									id="phone"
									bind:value={editForm.phone}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>

							<div class="space-y-1">
								<label for="email" class="block text-sm font-medium text-slate-700">{$t('store.email')}</label>
								<input
									type="email"
									id="email"
									bind:value={editForm.email}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>

							<div class="space-y-1 md:col-span-2">
								<label for="address" class="block text-sm font-medium text-slate-700">{$t('store.address')}</label>
								<input
									type="text"
									id="address"
									bind:value={editForm.address}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>

							<div class="space-y-1">
								<label for="website" class="block text-sm font-medium text-slate-700">{$t('store.website')}</label>
								<input
									type="url"
									id="website"
									bind:value={editForm.website}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
									placeholder="https://"
								/>
							</div>

							<div class="space-y-1">
								<label for="cutoff" class="block text-sm font-medium text-slate-700">{$t('store.business_day_cutoff')}</label>
								<input
									type="time"
									id="cutoff"
									bind:value={editForm.business_day_cutoff}
									class="w-full rounded-lg border-slate-200 focus:border-coral-500 focus:ring-coral-500 text-sm"
								/>
							</div>
							
							<div class="md:col-span-2 flex justify-end pt-2">
								<button
									type="submit"
									disabled={saving}
									class="bg-coral-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-coral-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-coral-500 disabled:opacity-50 flex items-center gap-2"
								>
									{#if saving}
										<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
											<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
											<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
										</svg>
										{$t('store.saving')}
									{:else}
										<Save class="w-4 h-4" />
										{$t('store.save')}
									{/if}
								</button>
							</div>
						</form>
					{:else}
						<div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-8 text-sm">
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.name')}</span>
								<span class="font-medium text-slate-900">{store.name || '-'}</span>
							</div>
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.nif')}</span>
								<span class="font-medium text-slate-900 font-mono">{store.nif || '-'}</span>
							</div>
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.phone')}</span>
								<span class="font-medium text-slate-900">{store.phone || '-'}</span>
							</div>
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.email')}</span>
								<span class="font-medium text-slate-900">{store.email || '-'}</span>
							</div>
							<div class="md:col-span-2">
								<span class="block text-slate-500 mb-1">{$t('store.address')}</span>
								<span class="font-medium text-slate-900">{store.address || '-'}</span>
							</div>
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.website')}</span>
								{#if store.website}
									<a href={store.website} target="_blank" rel="noreferrer" class="font-medium text-coral-600 hover:underline">{store.website}</a>
								{:else}
									<span class="text-slate-400">-</span>
								{/if}
							</div>
							<div>
								<span class="block text-slate-500 mb-1">{$t('store.business_day_cutoff')}</span>
								<span class="font-medium text-slate-900">{store.business_day_cutoff || '-'}</span>
							</div>
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</ConsoleLayout>
