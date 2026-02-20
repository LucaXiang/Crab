<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		CreditCard,
		Server,
		Clock,
		CheckCircle,
		AlertTriangle,
		XCircle,
		ArrowRight
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getProfile, getStores, createBillingPortal, ApiError, type ProfileResponse, type StoreDetail } from '$lib/api';
	import { formatDate, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let profile = $state<ProfileResponse | null>(null);
	let stores = $state<StoreDetail[]>([]);
	let loading = $state(true);
	let error = $state('');
	let billingLoading = $state(false);

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
			const [profileRes, storesRes] = await Promise.all([getProfile(token), getStores(token)]);
			profile = profileRes;
			stores = storesRes;
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

	async function handleBillingPortal() {
		billingLoading = true;
		try {
			const res = await createBillingPortal(token);
			window.location.href = res.url;
		} catch (err) {
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			billingLoading = false;
		}
	}

	function statusLabel(status: string): string {
		switch (status) {
			case 'active': return $t('dash.active');
			case 'suspended': return $t('dash.suspended');
			default: return $t('dash.cancelled');
		}
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'active': return 'text-green-600 bg-green-50';
			case 'suspended': return 'text-amber-600 bg-amber-50';
			default: return 'text-red-600 bg-red-50';
		}
	}
</script>

<svelte:head>
	<title>{$t('dash.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout email={profile?.profile.email ?? ''}>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if profile}
			<!-- Subscription Card -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<div class="flex items-start justify-between">
					<div>
						<h2 class="font-heading font-bold text-lg text-slate-900 mb-1">{$t('dash.subscription')}</h2>
						<p class="text-sm text-slate-500">{profile.profile.email}</p>
					</div>
					<span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium {statusColor(profile.subscription.status)}">
						{#if profile.subscription.status === 'active'}
							<CheckCircle class="w-3.5 h-3.5" />
						{:else if profile.subscription.status === 'suspended'}
							<AlertTriangle class="w-3.5 h-3.5" />
						{:else}
							<XCircle class="w-3.5 h-3.5" />
						{/if}
						{statusLabel(profile.subscription.status)}
					</span>
				</div>
				<div class="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.plan')}</p>
						<p class="text-sm font-semibold text-slate-900 capitalize">{profile.subscription.plan}</p>
					</div>
					{#if profile.subscription.current_period_end}
						<div>
							<p class="text-xs text-slate-400 mb-0.5">{$t('dash.next_billing')}</p>
							<p class="text-sm font-semibold text-slate-900">{formatDate(profile.subscription.current_period_end * 1000)}</p>
						</div>
					{/if}
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.quota_servers')}</p>
						<p class="text-sm font-semibold text-slate-900">{stores.length} / {profile.subscription.max_edge_servers}</p>
					</div>
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.stores_count')}</p>
						<p class="text-sm font-semibold text-slate-900">{stores.length}</p>
					</div>
				</div>
				<div class="mt-4">
					<button
						onclick={handleBillingPortal}
						disabled={billingLoading}
						class="inline-flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
					>
						<CreditCard class="w-4 h-4" />
						<span>{$t('dash.manage_billing')}</span>
					</button>
				</div>
			</div>

			<!-- Stores -->
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
