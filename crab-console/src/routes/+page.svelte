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
		ArrowRight,
		Sparkles,
		DollarSign,
		ShoppingBag,
		Users,
		TrendingUp,
		BarChart3,
		Download,
		Upload,
		ShieldCheck,
		FileKey,
		MapPin,
		Phone
	} from 'lucide-svelte';
	import { t, apiErrorMessage } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getProfile,
		getStores,
		getTenantOverview,
		createBillingPortal,
		createCheckout,
		uploadP12,
		ApiError,
		type ProfileResponse,
		type StoreDetail,
		type StoreOverview
	} from '$lib/api';
	import { formatDate, formatCurrency, timeAgo } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let profile = $state<ProfileResponse | null>(null);
	let stores = $state<StoreDetail[]>([]);
	let overview = $state<StoreOverview | null>(null);
	let loading = $state(true);
	let error = $state('');
	let billingLoading = $state(false);
	let checkoutLoading = $state('');

	// P12 upload state
	let onboardStep = $state<'p12' | 'plan'>('p12');
	let p12File = $state<File | null>(null);
	let p12Password = $state('');
	let p12Uploading = $state(false);
	let p12Error = $state('');
	let p12Uploaded = $state(false);
	let p12Subject = $state('');
	let p12Expires = $state<number | null>(null);

	let isAnnual = $state(true);

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	let needsOnboarding = $derived(
		profile !== null && profile.profile.status === 'verified' && !profile.subscription
	);

	let isCanceled = $derived(
		profile !== null && profile.subscription?.status === 'canceled'
	);

	function getTodayRange(): { from: number; to: number } {
		const now = new Date();
		const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		return { from: start.getTime(), to: now.getTime() + 60000 };
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			const profileRes = await getProfile(token);
			profile = profileRes;

			// Initialize P12 state from profile
			if (profileRes.p12?.has_p12) {
				p12Uploaded = true;
				p12Subject = profileRes.p12.subject ?? '';
				p12Expires = profileRes.p12.expires_at;
				onboardStep = 'plan';
			}

			if (profileRes.subscription && profileRes.subscription.status !== 'canceled') {
				const { from, to } = getTodayRange();
				const [storeList, ov] = await Promise.all([
					getStores(token),
					getTenantOverview(token, from, to)
				]);
				stores = storeList;
				overview = ov;
			}
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				clearAuth();
				goto('/login');
				return;
			}
			error = err instanceof ApiError ? apiErrorMessage($t, err.code, err.message) : $t('auth.error_generic');
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
			error = err instanceof ApiError ? apiErrorMessage($t, err.code, err.message) : $t('auth.error_generic');
		} finally {
			billingLoading = false;
		}
	}

	function handleP12FileChange(e: Event) {
		const input = e.target as HTMLInputElement;
		p12File = input.files?.[0] ?? null;
	}

	async function handleP12Upload() {
		if (!p12File) return;
		p12Uploading = true;
		p12Error = '';
		try {
			const res = await uploadP12(token, p12File, p12Password);
			p12Uploaded = true;
			p12Subject = res.common_name;
			p12Expires = res.expires_at;
			onboardStep = 'plan';
		} catch (err) {
			p12Error = err instanceof ApiError ? apiErrorMessage($t, err.code, err.message) : $t('auth.error_generic');
		} finally {
			p12Uploading = false;
		}
	}

	async function handleChoosePlan(planBase: string) {
		const plan = isAnnual ? `${planBase}_yearly` : planBase;
		checkoutLoading = planBase;
		error = '';
		try {
			const res = await createCheckout(token, plan);
			window.location.href = res.checkout_url;
		} catch (err) {
			error = err instanceof ApiError ? apiErrorMessage($t, err.code, err.message) : $t('auth.error_generic');
		} finally {
			checkoutLoading = '';
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
		{:else if needsOnboarding}
			<!-- Onboarding step indicator -->
			<div class="flex items-center justify-center gap-4 mb-6">
				<button
					onclick={() => { if (p12Uploaded) onboardStep = 'p12'; }}
					class="flex items-center gap-2 text-sm font-medium {onboardStep === 'p12' ? 'text-coral-600' : p12Uploaded ? 'text-green-600' : 'text-slate-400'}"
				>
					<span class="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold {onboardStep === 'p12' ? 'bg-coral-100 text-coral-600' : p12Uploaded ? 'bg-green-100 text-green-600' : 'bg-slate-100 text-slate-400'}">
						{#if p12Uploaded && onboardStep !== 'p12'}
							<CheckCircle class="w-4 h-4" />
						{:else}
							1
						{/if}
					</span>
					{$t('onboard.step_certificate')}
				</button>
				<div class="w-8 h-px {p12Uploaded ? 'bg-green-300' : 'bg-slate-200'}"></div>
				<span class="flex items-center gap-2 text-sm font-medium {onboardStep === 'plan' ? 'text-coral-600' : 'text-slate-400'}">
					<span class="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold {onboardStep === 'plan' ? 'bg-coral-100 text-coral-600' : 'bg-slate-100 text-slate-400'}">2</span>
					{$t('onboard.step_plan')}
				</span>
			</div>

			{#if onboardStep === 'p12'}
				<!-- Step 1: P12 upload -->
				<div class="text-center mb-6">
					<div class="w-14 h-14 bg-coral-100 rounded-2xl flex items-center justify-center mx-auto mb-4">
						<FileKey class="w-7 h-7 text-coral-500" />
					</div>
					<h1 class="font-heading text-2xl font-bold text-slate-900 mb-2">
						{$t('onboard.p12_title')}
					</h1>
					<p class="text-sm text-slate-500 max-w-md mx-auto">{$t('onboard.p12_subtitle')}</p>
				</div>

				<div class="max-w-md mx-auto">
					{#if p12Uploaded}
						<!-- Already uploaded -->
						<div class="bg-green-50 border border-green-200 rounded-xl p-5 mb-4">
							<div class="flex items-center gap-3 mb-3">
								<ShieldCheck class="w-6 h-6 text-green-600" />
								<span class="font-semibold text-green-800">{$t('onboard.p12_uploaded')}</span>
							</div>
							<div class="space-y-1 text-sm text-green-700">
								<p><span class="font-medium">{$t('onboard.p12_subject')}:</span> {p12Subject}</p>
								{#if p12Expires}
									<p><span class="font-medium">{$t('onboard.p12_expires')}:</span> {formatDate(p12Expires * 1000)}</p>
								{/if}
							</div>
						</div>
						<div class="flex gap-3">
							<button
								onclick={() => { p12Uploaded = false; p12File = null; p12Password = ''; }}
								class="flex-1 py-3 bg-slate-100 hover:bg-slate-200 text-slate-700 font-semibold rounded-lg transition-colors cursor-pointer"
							>
								{$t('onboard.p12_change')}
							</button>
							<button
								onclick={() => { onboardStep = 'plan'; }}
								class="flex-1 py-3 bg-coral-500 hover:bg-coral-600 text-white font-semibold rounded-lg transition-colors cursor-pointer flex items-center justify-center gap-2"
							>
								{$t('onboard.p12_continue')}
								<ArrowRight class="w-4 h-4" />
							</button>
						</div>
					{:else}
						<!-- Upload form -->
						<div class="bg-white rounded-2xl border border-slate-200 p-6 space-y-4">
							{#if p12Error}
								<div class="p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
									{p12Error}
								</div>
							{/if}

							<div>
								<label for="p12-file" class="block text-sm font-medium text-slate-700 mb-1.5">
									{$t('onboard.p12_label')}
								</label>
								<input
									id="p12-file"
									type="file"
									accept=".p12,.pfx"
									onchange={handleP12FileChange}
									class="w-full text-sm text-slate-600 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-coral-50 file:text-coral-600 hover:file:bg-coral-100 file:cursor-pointer"
								/>
							</div>

							<div>
								<label for="p12-password" class="block text-sm font-medium text-slate-700 mb-1.5">
									{$t('onboard.p12_password')}
								</label>
								<input
									id="p12-password"
									type="password"
									bind:value={p12Password}
									placeholder={$t('onboard.p12_password_placeholder')}
									class="w-full px-3 py-2.5 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent"
								/>
							</div>

							<button
								onclick={handleP12Upload}
								disabled={!p12File || p12Uploading}
								class="w-full py-3 bg-coral-500 hover:bg-coral-600 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
							>
								{#if p12Uploading}
									<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
										<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
										<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
									</svg>
									{$t('onboard.p12_uploading')}
								{:else}
									<Upload class="w-4 h-4" />
									{$t('onboard.p12_upload')}
								{/if}
							</button>

							<p class="text-xs text-slate-400 text-center">{$t('onboard.p12_skip_info')}</p>
						</div>
					{/if}
				</div>
			{:else}
				<!-- Step 2: Choose plan -->
				<div class="text-center mb-2">
					<div
						class="w-14 h-14 bg-coral-100 rounded-2xl flex items-center justify-center mx-auto mb-4"
					>
						<Sparkles class="w-7 h-7 text-coral-500" />
					</div>
					<h1 class="font-heading text-2xl font-bold text-slate-900 mb-2">
						{$t('onboard.title')}
					</h1>
					<p class="text-sm text-slate-500 max-w-md mx-auto">{$t('onboard.subtitle')}</p>
				</div>

				{#if error}
					<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600 max-w-3xl mx-auto mb-4">
						{error}
					</div>
				{/if}

				<div class="flex items-center justify-center gap-3 mb-8">
					<span class="text-sm font-medium transition-colors {isAnnual ? 'text-slate-400' : 'text-slate-900'}">
						{$t('onboard.monthly')}
					</span>
					<button
						aria-label="Toggle annual billing"
						onclick={() => (isAnnual = !isAnnual)}
						class="relative w-12 h-6 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-coral-500 focus:ring-offset-2 {isAnnual
							? 'bg-coral-500'
							: 'bg-slate-300'}"
					>
						<span
							class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform {isAnnual
								? 'translate-x-6'
								: 'translate-x-0'}"
						></span>
					</button>
					<span class="text-sm font-medium transition-colors {isAnnual ? 'text-slate-900' : 'text-slate-400'}">
						{$t('onboard.yearly')}
						<span class="text-coral-500 text-xs font-bold ml-1">{$t('onboard.save_20')}</span>
					</span>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-3xl mx-auto">
					<!-- Basic -->
					<div class="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col">
						<h3 class="font-heading font-bold text-lg text-slate-900">Basic</h3>
						<p class="text-sm text-slate-500 mt-1">{$t('onboard.basic_desc')}</p>
						<div class="mt-4 mb-6">
							<span class="text-3xl font-bold text-slate-900">&euro;{isAnnual ? '31' : '39'}</span>
							<span class="text-sm text-slate-500">/{$t('onboard.month')}</span>
							{#if isAnnual}
								<p class="text-xs text-slate-400 mt-1">{$t('onboard.billed_yearly')}</p>
							{/if}
						</div>
						<ul class="space-y-2 text-sm text-slate-600 mb-6 flex-1">
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 1 {$t('onboard.location')}
							</li>
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 5 {$t('onboard.devices')}
							</li>
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
								{$t('onboard.cloud_sync')}
							</li>
						</ul>
						<button
							onclick={() => handleChoosePlan('basic')}
							disabled={checkoutLoading !== ''}
							class="w-full py-3 bg-slate-800 hover:bg-slate-900 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
						>
							{#if checkoutLoading === 'basic'}
								<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
								</svg>
							{/if}
							{$t('onboard.choose')}
						</button>
					</div>

					<!-- Pro -->
					<div class="bg-white rounded-2xl border-2 border-coral-500 p-6 flex flex-col relative">
						<span class="absolute -top-3 left-6 bg-coral-500 text-white text-xs font-bold px-3 py-0.5 rounded-full">{$t('onboard.popular')}</span>
						<h3 class="font-heading font-bold text-lg text-slate-900">Pro</h3>
						<p class="text-sm text-slate-500 mt-1">{$t('onboard.pro_desc')}</p>
						<div class="mt-4 mb-6">
							<span class="text-3xl font-bold text-slate-900">&euro;{isAnnual ? '55' : '69'}</span>
							<span class="text-sm text-slate-500">/{$t('onboard.month')}</span>
							{#if isAnnual}
								<p class="text-xs text-slate-400 mt-1">{$t('onboard.billed_yearly')}</p>
							{/if}
						</div>
						<ul class="space-y-2 text-sm text-slate-600 mb-6 flex-1">
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 3 {$t('onboard.location')}
							</li>
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" /> 10 {$t('onboard.devices')}
							</li>
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
								{$t('onboard.cloud_sync')}
							</li>
							<li class="flex items-center gap-2">
								<CheckCircle class="w-4 h-4 text-green-500 shrink-0" />
								{$t('onboard.priority_support')}
							</li>
						</ul>
						<button
							onclick={() => handleChoosePlan('pro')}
							disabled={checkoutLoading !== ''}
							class="w-full py-3 bg-coral-500 hover:bg-coral-600 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
						>
							{#if checkoutLoading === 'pro'}
								<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
									<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
									<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
								</svg>
							{/if}
							{$t('onboard.choose')}
						</button>
					</div>
				</div>
			{/if}
		{:else if isCanceled && profile}
			<!-- Canceled subscription -->
			<div class="bg-white rounded-2xl border border-red-200 p-6">
				<div class="flex items-start justify-between">
					<div>
						<h2 class="font-heading font-bold text-lg text-slate-900 mb-1">
							{$t('dash.subscription')}
						</h2>
						<p class="text-sm text-slate-500">{profile.profile.email}</p>
					</div>
					<span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium text-red-600 bg-red-50">
						<XCircle class="w-3.5 h-3.5" />
						{$t('dash.cancelled')}
					</span>
				</div>
				<div class="mt-4 grid grid-cols-2 gap-4">
					<div>
						<p class="text-xs text-slate-400 mb-0.5">{$t('dash.plan')}</p>
						<p class="text-sm font-semibold text-slate-900 capitalize">
							{profile?.subscription?.plan}
						</p>
					</div>
				</div>
				<div class="mt-4">
					<button
						onclick={handleBillingPortal}
						disabled={billingLoading}
						class="inline-flex items-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
					>
						<CreditCard class="w-4 h-4" />
						<span>{$t('dash.manage_billing')}</span>
					</button>
				</div>
			</div>
		{:else if profile?.subscription}
			<!-- Tenant-wide KPI summary (all stores combined, today) -->
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<BarChart3 class="w-5 h-5 text-coral-500" />
						<h2 class="font-heading font-bold text-lg text-slate-900">{$t('stats.today_summary')}</h2>
						<span class="text-xs text-slate-400 bg-slate-100 px-2 py-0.5 rounded-full">
							{$t('stats.all_stores')}
						</span>
					</div>
					<span class="text-sm text-slate-400">{new Date().toLocaleDateString()}</span>
				</div>

				<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="w-8 h-8 bg-coral-100 rounded-lg flex items-center justify-center mb-2">
							<DollarSign class="w-4 h-4 text-coral-600" />
						</div>
						<p class="text-lg font-bold text-slate-900">{formatCurrency(overview?.revenue ?? 0)}</p>
						<p class="text-xs text-slate-400">{$t('stats.total_sales')}</p>
					</div>
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="w-8 h-8 bg-green-100 rounded-lg flex items-center justify-center mb-2">
							<ShoppingBag class="w-4 h-4 text-green-600" />
						</div>
						<p class="text-lg font-bold text-slate-900">{overview?.orders ?? 0}</p>
						<p class="text-xs text-slate-400">{$t('stats.completed_orders')}</p>
					</div>
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="w-8 h-8 bg-blue-100 rounded-lg flex items-center justify-center mb-2">
							<Users class="w-4 h-4 text-blue-600" />
						</div>
						<p class="text-lg font-bold text-slate-900">{overview?.guests ?? 0}</p>
						<p class="text-xs text-slate-400">{$t('stats.guests')}</p>
					</div>
					<div class="bg-white rounded-xl border border-slate-200 p-4">
						<div class="w-8 h-8 bg-purple-100 rounded-lg flex items-center justify-center mb-2">
							<TrendingUp class="w-4 h-4 text-purple-600" />
						</div>
						<p class="text-lg font-bold text-slate-900">{formatCurrency(overview?.average_order_value ?? 0)}</p>
						<p class="text-xs text-slate-400">{$t('stats.average_order')}</p>
					</div>
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
								class="flex flex-col md:flex-row md:items-center justify-between p-4 bg-slate-50 rounded-xl border border-slate-100 hover:border-slate-200 transition-colors duration-150 gap-3 md:gap-0"
							>
								<div class="flex items-center gap-3">
									<div class="w-10 h-10 bg-coral-100 rounded-lg flex items-center justify-center shrink-0">
										<Server class="w-5 h-5 text-coral-600" />
									</div>
									<div class="min-w-0">
										<p class="text-sm font-medium text-slate-900 truncate">
											{store.name ?? store.store_info?.name ?? `Store #${store.id}`}
										</p>
										<p class="text-xs text-slate-400 truncate mb-1">
											ID: {store.device_id.slice(0, 12)}...
										</p>
										{#if store.address}
											<div class="flex items-center gap-1 text-xs text-slate-500 truncate mb-0.5">
												<MapPin class="w-3 h-3 text-slate-400" />
												<span>{store.address}</span>
											</div>
										{/if}
										{#if store.phone}
											<div class="flex items-center gap-1 text-xs text-slate-500 truncate">
												<Phone class="w-3 h-3 text-slate-400" />
												<span>{store.phone}</span>
											</div>
										{/if}
									</div>
								</div>
								<div class="flex items-center justify-between md:justify-end gap-3 w-full md:w-auto pl-13 md:pl-0">
									<div class="text-right">
										<div class="inline-flex items-center gap-1 text-xs text-slate-500">
											<Clock class="w-3.5 h-3.5" />
											<span
												>{$t('dash.last_sync')}: {store.last_sync_at
													? timeAgo(store.last_sync_at)
													: $t('dash.never')}</span
											>
										</div>
									</div>
									<ArrowRight class="w-4 h-4 text-slate-400" />
								</div>
							</a>
						{/each}
					</div>
				{/if}
			</div>


			<!-- Download App -->
			<div class="bg-white rounded-2xl border border-slate-200 p-6">
				<div class="flex flex-col md:flex-row md:items-center justify-between gap-4 md:gap-0">
					<div class="flex items-center gap-3">
						<div class="w-10 h-10 bg-coral-100 rounded-lg flex items-center justify-center shrink-0">
							<Download class="w-5 h-5 text-coral-600" />
						</div>
						<div>
							<h3 class="text-sm font-semibold text-slate-900">{$t('dash.download_app')}</h3>
							<p class="text-xs text-slate-400">{$t('dash.download_desc')}</p>
						</div>
					</div>
					<a
						href="https://auth.redcoral.app/api/download/latest"
						class="inline-flex items-center justify-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-medium text-sm px-4 py-2 rounded-lg transition-colors w-full md:w-auto"
					>
						<Download class="w-4 h-4" />
						{$t('dash.download_windows')}
					</a>
				</div>
			</div>
		{/if}
	</div>
</ConsoleLayout>
