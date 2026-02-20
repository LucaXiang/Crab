<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		LogOut,
		CreditCard,
		Server,
		Clock,
		Lock,
		Eye,
		EyeOff,
		ArrowRight,
		CheckCircle,
		AlertTriangle,
		XCircle
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getProfile,
		getStores,
		createBillingPortal,
		changePassword,
		ApiError,
		type ProfileResponse,
		type StoreInfo
	} from '$lib/api';

	let profile = $state<ProfileResponse | null>(null);
	let stores = $state<StoreInfo[]>([]);
	let loading = $state(true);
	let error = $state('');

	// Change password state
	let showChangePassword = $state(false);
	let currentPassword = $state('');
	let newPassword = $state('');
	let showCurrentPw = $state(false);
	let showNewPw = $state(false);
	let changePwLoading = $state(false);
	let changePwError = $state('');
	let changePwSuccess = $state('');

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

	function handleLogout() {
		clearAuth();
		goto('/login');
	}

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

	async function handleChangePassword(e: SubmitEvent) {
		e.preventDefault();
		changePwError = '';
		changePwSuccess = '';
		changePwLoading = true;
		try {
			await changePassword(token, currentPassword, newPassword);
			changePwSuccess = $t('dashboard.password_changed');
			currentPassword = '';
			newPassword = '';
			showChangePassword = false;
		} catch (err) {
			changePwError = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			changePwLoading = false;
		}
	}

	function formatDate(ts: number): string {
		return new Date(ts).toLocaleDateString();
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'active':
				return 'text-green-600 bg-green-50';
			case 'suspended':
				return 'text-amber-600 bg-amber-50';
			default:
				return 'text-red-600 bg-red-50';
		}
	}
</script>

<svelte:head>
	<title>{$t('dashboard.page_title')} — RedCoral</title>
</svelte:head>

<div class="min-h-screen bg-slate-50">
	<!-- Header -->
	<nav class="bg-white border-b border-slate-200">
		<div class="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
			<a href="/" class="flex items-center gap-2">
				<div class="w-8 h-8 bg-coral-500 rounded-lg flex items-center justify-center">
					<svg
						viewBox="0 0 24 24"
						fill="none"
						class="w-[18px] h-[18px] text-white"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path
							d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z"
						/>
						<line x1="6" y1="17" x2="18" y2="17" />
					</svg>
				</div>
				<span class="text-lg font-heading font-bold text-slate-900"
					>Red<span class="text-coral-500">Coral</span></span
				>
			</a>
			<button
				onclick={handleLogout}
				class="inline-flex items-center gap-1.5 text-sm text-slate-500 hover:text-slate-700 cursor-pointer transition-colors duration-150"
			>
				<LogOut class="w-4 h-4" />
				<span>{$t('dashboard.logout')}</span>
			</button>
		</div>
	</nav>

	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
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
		{:else if profile}
			<!-- Subscription Card -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<div class="flex items-start justify-between">
					<div>
						<h2 class="font-heading font-bold text-lg text-slate-900 mb-1">
							{$t('dashboard.subscription')}
						</h2>
						<p class="text-sm text-slate-500">{profile.profile.email}</p>
					</div>
					<span
						class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium {statusColor(
							profile.subscription.status
						)}"
					>
						{#if profile.subscription.status === 'active'}
							<CheckCircle class="w-3.5 h-3.5" />
						{:else if profile.subscription.status === 'suspended'}
							<AlertTriangle class="w-3.5 h-3.5" />
						{:else}
							<XCircle class="w-3.5 h-3.5" />
						{/if}
						{profile.subscription.status}
					</span>
				</div>
				<div class="mt-4 grid grid-cols-2 md:grid-cols-3 gap-4">
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dashboard.plan')}</p>
						<p class="text-sm font-semibold text-slate-900 capitalize">
							{profile.subscription.plan}
						</p>
					</div>
					{#if profile.subscription.current_period_end}
						<div>
							<p class="text-xs text-slate-400 mb-0.5">{$t('dashboard.next_billing')}</p>
							<p class="text-sm font-semibold text-slate-900">
								{formatDate(profile.subscription.current_period_end * 1000)}
							</p>
						</div>
					{/if}
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dashboard.stores_count')}</p>
						<p class="text-sm font-semibold text-slate-900">{stores.length}</p>
					</div>
				</div>
				<div class="mt-4 flex gap-3">
					<button
						onclick={handleBillingPortal}
						disabled={billingLoading}
						class="inline-flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
					>
						<CreditCard class="w-4 h-4" />
						<span>{$t('dashboard.manage_billing')}</span>
					</button>
					<button
						onclick={() => (showChangePassword = !showChangePassword)}
						class="inline-flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer"
					>
						<Lock class="w-4 h-4" />
						<span>{$t('dashboard.change_password')}</span>
					</button>
				</div>
			</div>

			<!-- Change Password -->
			{#if showChangePassword}
				<div class="bg-white rounded-2xl border border-slate-200 p-6">
					<h3 class="font-heading font-bold text-slate-900 mb-4">
						{$t('dashboard.change_password')}
					</h3>
					{#if changePwError}
						<div
							class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600"
						>
							{changePwError}
						</div>
					{/if}
					{#if changePwSuccess}
						<div
							class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600"
						>
							{changePwSuccess}
						</div>
					{/if}
					<form class="space-y-3 max-w-sm" onsubmit={handleChangePassword}>
						<div class="relative">
							<input
								type={showCurrentPw ? 'text' : 'password'}
								required
								bind:value={currentPassword}
								placeholder={$t('dashboard.current_password')}
								class="w-full px-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
							/>
							<button
								type="button"
								onclick={() => (showCurrentPw = !showCurrentPw)}
								class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 cursor-pointer"
							>
								{#if showCurrentPw}<EyeOff class="w-4 h-4" />{:else}<Eye
										class="w-4 h-4"
									/>{/if}
							</button>
						</div>
						<div class="relative">
							<input
								type={showNewPw ? 'text' : 'password'}
								required
								minlength={8}
								bind:value={newPassword}
								placeholder={$t('dashboard.new_password')}
								class="w-full px-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
							/>
							<button
								type="button"
								onclick={() => (showNewPw = !showNewPw)}
								class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 cursor-pointer"
							>
								{#if showNewPw}<EyeOff class="w-4 h-4" />{:else}<Eye
										class="w-4 h-4"
									/>{/if}
							</button>
						</div>
						<button
							type="submit"
							disabled={changePwLoading}
							class="bg-coral-500 hover:bg-coral-600 text-white font-semibold text-sm px-4 py-2.5 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
						>
							{changePwLoading ? $t('auth.loading') : $t('dashboard.save_password')}
						</button>
					</form>
				</div>
			{/if}

			<!-- Stores -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<h2 class="font-heading font-bold text-lg text-slate-900 mb-4">
					{$t('dashboard.stores_title')}
				</h2>
				{#if stores.length === 0}
					<div class="text-center py-8">
						<Server class="w-10 h-10 text-slate-300 mx-auto mb-3" />
						<p class="text-sm text-slate-500">{$t('dashboard.no_stores')}</p>
						<p class="text-xs text-slate-400 mt-1">{$t('dashboard.no_stores_hint')}</p>
					</div>
				{:else}
					<div class="space-y-3">
						{#each stores as store}
							<div
								class="flex items-center justify-between p-4 bg-slate-50 rounded-xl border border-slate-100"
							>
								<div class="flex items-center gap-3">
									<div
										class="w-10 h-10 bg-coral-100 rounded-lg flex items-center justify-center"
									>
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
								<div class="text-right">
									<div
										class="inline-flex items-center gap-1 text-xs text-slate-500"
									>
										<Clock class="w-3.5 h-3.5" />
										<span>{$t('dashboard.last_sync')}: {formatDate(store.last_sync_at)}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
