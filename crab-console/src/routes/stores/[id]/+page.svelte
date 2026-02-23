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
		Server,
		Clock,
		Save,
		ShieldAlert,
		Radio,
		Check,
		Copy
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, updateStore, ApiError, type StoreDetail } from '$lib/api';
	import { formatDate, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let store = $state<StoreDetail | null>(null);
	let loading = $state(true);
	let error = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let saveError = $state('');
	let copied = $state(false);

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
		{ key: 'nav.products', href: `/stores/${storeId}/products`, icon: Package },
		{ key: 'nav.red_flags', href: `/stores/${storeId}/red-flags`, icon: ShieldAlert }
	];

	function initForm(s: StoreDetail) {
		editForm = {
			name: s.name ?? s.store_info?.name as string ?? '',
			address: s.address ?? s.store_info?.address as string ?? '',
			phone: s.phone ?? s.store_info?.phone as string ?? '',
			nif: s.nif ?? s.store_info?.nif as string ?? '',
			email: s.email ?? s.store_info?.email as string ?? '',
			website: s.website ?? s.store_info?.website as string ?? '',
			business_day_cutoff: s.business_day_cutoff ?? s.store_info?.business_day_cutoff as string ?? ''
		};
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }

		try {
			const stores = await getStores(token);
			store = stores.find((s) => s.id === storeId) ?? null;
			if (!store) error = 'Store not found';
			else initForm(store);
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
		saved = false;
		saveError = '';
		try {
			await updateStore(
				token,
				store.id,
				editForm.name || undefined,
				editForm.address || undefined,
				editForm.phone || undefined,
				editForm.nif || undefined,
				editForm.email || undefined,
				editForm.website || undefined,
				editForm.business_day_cutoff || undefined
			);

			store.name = editForm.name;
			store.address = editForm.address;
			store.phone = editForm.phone;
			store.nif = editForm.nif;
			store.email = editForm.email;
			store.website = editForm.website;
			store.business_day_cutoff = editForm.business_day_cutoff;

			saved = true;
			setTimeout(() => { saved = false; }, 2000);
		} catch (err) {
			saveError = err instanceof ApiError ? err.message : $t('auth.error_generic');
			setTimeout(() => { saveError = ''; }, 3000);
		} finally {
			saving = false;
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		copied = true;
		setTimeout(() => { copied = false; }, 1500);
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
							<button
								class="inline-flex items-center gap-1 hover:text-slate-600 transition-colors cursor-pointer truncate"
								onclick={() => copyToClipboard(store!.device_id)}
								title={store.device_id}
							>
								{#if copied}
									<Check class="w-3 h-3 text-green-500" />
								{:else}
									<Copy class="w-3 h-3" />
								{/if}
								{store.device_id.slice(0, 16)}...
							</button>
							<span class="hidden md:inline text-slate-300">·</span>
							<span class="inline-flex items-center gap-1">
								<Clock class="w-3 h-3" />
								{$t('dash.last_sync')}: {store.last_sync_at ? timeAgo(store.last_sync_at) : $t('dash.never')}
							</span>
							<span class="hidden md:inline text-slate-300">·</span>
							<span>{$t('store.registered')}: {formatDate(store.registered_at)}</span>
						</div>
					</div>

					<div class="flex items-center gap-2">
						{#if store.is_online}
							<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-green-50 text-green-700 text-xs font-medium">
								<Radio class="w-3 h-3" />
								Online
							</span>
						{:else}
							<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-slate-100 text-slate-500 text-xs font-medium">
								Offline
							</span>
						{/if}
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

			<!-- Store Details Form (always editable) -->
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<div class="px-6 py-4 border-b border-slate-100">
					<h2 class="font-heading font-semibold text-slate-900">{$t('store.title')}</h2>
				</div>

				<form class="p-6 grid grid-cols-1 md:grid-cols-2 gap-6" onsubmit={handleSave}>
					<div class="space-y-1">
						<label for="name" class="block text-sm font-medium text-slate-700">{$t('store.name')}</label>
						<input
							type="text"
							id="name"
							bind:value={editForm.name}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="space-y-1">
						<label for="nif" class="block text-sm font-medium text-slate-700">{$t('store.nif')}</label>
						<input
							type="text"
							id="nif"
							bind:value={editForm.nif}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="space-y-1">
						<label for="phone" class="block text-sm font-medium text-slate-700">{$t('store.phone')}</label>
						<input
							type="tel"
							id="phone"
							bind:value={editForm.phone}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="space-y-1">
						<label for="email" class="block text-sm font-medium text-slate-700">{$t('store.email')}</label>
						<input
							type="email"
							id="email"
							bind:value={editForm.email}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="space-y-1 md:col-span-2">
						<label for="address" class="block text-sm font-medium text-slate-700">{$t('store.address')}</label>
						<input
							type="text"
							id="address"
							bind:value={editForm.address}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="space-y-1">
						<label for="website" class="block text-sm font-medium text-slate-700">{$t('store.website')}</label>
						<input
							type="url"
							id="website"
							bind:value={editForm.website}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
							placeholder="https://"
						/>
					</div>

					<div class="space-y-1">
						<label for="cutoff" class="block text-sm font-medium text-slate-700">{$t('store.business_day_cutoff')}</label>
						<input
							type="time"
							id="cutoff"
							bind:value={editForm.business_day_cutoff}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm"
						/>
					</div>

					<div class="md:col-span-2 flex items-center justify-end gap-3 pt-2">
						{#if saveError}
							<span class="text-sm text-red-600">{saveError}</span>
						{/if}
						{#if saved}
							<span class="inline-flex items-center gap-1 text-sm text-green-600">
								<Check class="w-4 h-4" />
								{$t('store.saved')}
							</span>
						{/if}
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
			</div>
		{/if}
	</div>
</ConsoleLayout>
