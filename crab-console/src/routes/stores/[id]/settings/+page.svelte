<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { getStores, updateStore, type StoreDetail } from '$lib/api';
	import { t } from '$lib/i18n';
	import { goto } from '$app/navigation';
	import { ArrowLeft, Save, Check, Copy } from 'lucide-svelte';

	let storeId = $state(0);
	let store = $state<StoreDetail | null>(null);
	let name = $state('');
	let address = $state('');
	let phone = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let success = $state<string | null>(null);
	let copied = $state('');

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		copied = text;
		setTimeout(() => {
			copied = '';
		}, 2000);
	}

	$effect(() => {
		storeId = parseInt($page.params.id || '0');
	});

	async function loadStore() {
		try {
			const token = localStorage.getItem('token');
			if (!token) return;
			const stores = await getStores(token);
			store = stores.find((s) => s.id === storeId) || null;
			if (store) {
				name = store.name || (store.store_info?.name as string) || '';
				address = store.address || '';
				phone = store.phone || '';
			}
		} catch (e) {
			console.error(e);
		}
	}

	async function handleSave() {
		if (!name.trim()) return;
		loading = true;
		error = null;
		success = null;
		try {
			const token = localStorage.getItem('token');
			if (!token) throw new Error('No token');
			await updateStore(token, storeId, name, address, phone);
			await loadStore(); // Reload to confirm
			success = 'Store settings updated successfully';
			setTimeout(() => {
				success = null;
			}, 3000);
		} catch (e: any) {
			error = e.message;
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadStore();
	});
</script>

<div class="p-6 max-w-2xl mx-auto">
	<div class="flex items-center gap-4 mb-8">
		<a
			href="/stores/{storeId}"
			class="p-2 hover:bg-slate-100 rounded-lg transition-colors text-slate-500"
		>
			<ArrowLeft class="w-5 h-5" />
		</a>
		<h1 class="text-2xl font-bold text-slate-900">Store Settings</h1>
	</div>

	{#if store}
		<div class="bg-white rounded-xl border border-slate-200 p-6 shadow-sm">
			<h2 class="text-lg font-semibold text-slate-900 mb-4">General Information</h2>

			<div class="space-y-4">
				<div>
					<label class="block text-sm font-medium text-slate-700 mb-1" for="store-name">
						Store Name
					</label>
					<input
						id="store-name"
						type="text"
						bind:value={name}
						placeholder="Enter store name"
						class="w-full px-3 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent transition-all"
					/>
					<p class="mt-1 text-xs text-slate-500">
						This name will be displayed in your dashboard.
					</p>
				</div>

				<div>
					<label class="block text-sm font-medium text-slate-700 mb-1" for="store-address">
						Address
					</label>
					<input
						id="store-address"
						type="text"
						bind:value={address}
						placeholder="Enter store address"
						class="w-full px-3 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent transition-all"
					/>
				</div>

				<div>
					<label class="block text-sm font-medium text-slate-700 mb-1" for="store-phone">
						Phone Number
					</label>
					<input
						id="store-phone"
						type="text"
						bind:value={phone}
						placeholder="Enter phone number"
						class="w-full px-3 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent transition-all"
					/>
				</div>

				<div>
					<p class="text-sm font-medium text-slate-700 mb-1"> Store ID </p>
					<div
						class="flex items-center justify-between p-3 bg-slate-50 border border-slate-200 rounded-lg"
					>
						<code class="text-sm text-slate-600 font-mono">{store.entity_id}</code>
						<button
							onclick={() => copyToClipboard(store?.entity_id || '')}
							class="text-slate-400 hover:text-slate-600 transition-colors"
							aria-label="Copy Store ID"
						>
							{#if copied === store.entity_id}
								<Check class="w-4 h-4 text-green-500" />
							{:else}
								<Copy class="w-4 h-4" />
							{/if}
						</button>
					</div>
				</div>

				<div>
					<p class="text-sm font-medium text-slate-700 mb-1"> Device ID </p>
					<div
						class="flex items-center justify-between p-3 bg-slate-50 border border-slate-200 rounded-lg"
					>
						<code class="text-sm text-slate-500 font-mono break-all">{store.device_id}</code>
						<button
							onclick={() => copyToClipboard(store?.device_id || '')}
							class="text-slate-400 hover:text-slate-600 transition-colors ml-2 shrink-0"
							aria-label="Copy Device ID"
						>
							{#if copied === store.device_id}
								<Check class="w-4 h-4 text-green-500" />
							{:else}
								<Copy class="w-4 h-4" />
							{/if}
						</button>
					</div>
				</div>
			</div>

			<div class="mt-8 flex items-center justify-end gap-3">
				<button
					onclick={() => loadStore()}
					class="px-4 py-2 text-slate-600 font-medium hover:text-slate-900 transition-colors cursor-pointer"
				>
					Cancel
				</button>
				<button
					onclick={handleSave}
					disabled={loading || !name.trim()}
					class="inline-flex items-center gap-2 bg-coral-500 hover:bg-coral-600 text-white font-semibold px-6 py-2 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-sm shadow-coral-500/20 cursor-pointer"
				>
					{#if loading}
						<div
							class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"
						></div>
					{:else}
						<Save class="w-4 h-4" />
					{/if}
					Save Changes
				</button>
			</div>

			{#if error}
				<div class="mt-4 p-3 bg-red-50 text-red-600 rounded-lg text-sm">
					{error}
				</div>
			{/if}
			{#if success}
				<div class="mt-4 p-3 bg-green-50 text-green-600 rounded-lg text-sm">
					{success}
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex items-center justify-center py-12">
			<div
				class="w-8 h-8 border-4 border-coral-200 border-t-coral-500 rounded-full animate-spin"
			></div>
		</div>
	{/if}
</div>
