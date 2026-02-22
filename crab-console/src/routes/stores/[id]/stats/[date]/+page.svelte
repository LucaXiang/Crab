<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, BarChart3, Calendar } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStoreOverview, ApiError, type StoreOverview } from '$lib/api';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';
	import StoreOverviewDisplay from '$lib/components/StoreOverviewDisplay.svelte';

	const storeId = Number(page.params.id);
	const params = page.params as Record<string, string>;
	const dateStr = params.date || '';

	let overview = $state<StoreOverview | null>(null);
	let loading = $state(true);
	let error = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	function getDateRange(dateString: string): { from: number; to: number } {
		// Assuming dateString is YYYY-MM-DD
		// We want 00:00:00 to 23:59:59.999 of that day
		const start = new Date(dateString + 'T00:00:00');
		const end = new Date(dateString + 'T23:59:59.999');
		return { from: start.getTime(), to: end.getTime() };
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}

		try {
			const { from, to } = getDateRange(dateStr);
			overview = await getStoreOverview(token, storeId, from, to);
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
	<title>{dateStr} — {$t('stats.daily_report')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}/stats" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<div class="flex items-center gap-2">
				<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('stats.daily_report')}</h1>
				<span class="text-slate-300">/</span>
				<span class="text-slate-600 font-medium flex items-center gap-1">
					<Calendar class="w-4 h-4" />
					{dateStr}
				</span>
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
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
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
		{:else if overview}
			<StoreOverviewDisplay {overview} showHeader={false} />
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<BarChart3 class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('stats.no_data')}</p>
			</div>
		{/if}
	</div>
</ConsoleLayout>
