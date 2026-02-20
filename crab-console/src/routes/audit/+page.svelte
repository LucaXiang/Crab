<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { ScrollText } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getAuditLog, ApiError, type AuditEntry } from '$lib/api';
	import { formatDateTime } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let entries = $state<AuditEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let currentPage = $state(1);
	let hasMore = $state(true);
	let loadingMore = $state(false);

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	async function loadEntries(reset = false) {
		if (reset) { currentPage = 1; entries = []; hasMore = true; }
		try {
			const batch = await getAuditLog(token, currentPage, 20);
			if (reset) entries = batch; else entries = [...entries, ...batch];
			hasMore = batch.length === 20;
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		}
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }
		await loadEntries(true);
		loading = false;
	});

	async function loadMore() {
		loadingMore = true;
		currentPage++;
		await loadEntries();
		loadingMore = false;
	}

	function actionLabel(action: string): string {
		const key = `audit.${action}`;
		const translated = $t(key);
		return translated !== key ? translated : action;
	}
</script>

<svelte:head>
	<title>{$t('audit.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<h1 class="font-heading text-xl font-bold text-slate-900">{$t('audit.title')}</h1>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if entries.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<ScrollText class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('audit.empty')}</p>
			</div>
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-slate-100 text-left">
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('audit.action')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('audit.detail')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('audit.ip')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('audit.date')}</th>
						</tr>
					</thead>
					<tbody>
						{#each entries as entry}
							<tr class="border-b border-slate-50">
								<td class="px-4 py-2.5 font-medium text-slate-900">{actionLabel(entry.action)}</td>
								<td class="px-4 py-2.5 text-slate-500 text-xs max-w-xs truncate">
									{entry.detail ? JSON.stringify(entry.detail) : '—'}
								</td>
								<td class="px-4 py-2.5 text-slate-500 font-mono text-xs">{entry.ip_address ?? '—'}</td>
								<td class="px-4 py-2.5 text-slate-500">{formatDateTime(entry.created_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			{#if hasMore}
				<div class="text-center">
					<button
						onclick={loadMore}
						disabled={loadingMore}
						class="px-4 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-600 hover:bg-slate-50 cursor-pointer disabled:opacity-50"
					>
						{loadingMore ? $t('auth.loading') : $t('audit.load_more')}
					</button>
				</div>
			{/if}
		{/if}
	</div>
</ConsoleLayout>
