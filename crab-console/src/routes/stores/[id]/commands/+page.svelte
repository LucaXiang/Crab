<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ArrowLeft, Terminal, Send } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getCommands, createCommand, ApiError, type CommandRecord } from '$lib/api';
	import { formatDateTime } from '$lib/format';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	const storeId = Number(page.params.id);

	let commands = $state<CommandRecord[]>([]);
	let loading = $state(true);
	let error = $state('');
	let currentPage = $state(1);
	let hasMore = $state(true);
	let loadingMore = $state(false);
	let sendLoading = $state(false);
	let sendType = $state('sync');
	let sendResult = $state('');

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	async function loadCommands(reset = false) {
		if (reset) { currentPage = 1; commands = []; hasMore = true; }
		try {
			const batch = await getCommands(token, storeId, currentPage, 20);
			if (reset) commands = batch; else commands = [...commands, ...batch];
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
		await loadCommands(true);
		loading = false;
	});

	async function loadMore() {
		loadingMore = true;
		currentPage++;
		await loadCommands();
		loadingMore = false;
	}

	async function handleSend() {
		sendLoading = true;
		sendResult = '';
		try {
			const res = await createCommand(token, storeId, sendType);
			sendResult = `Command #${res.command_id} created (ws_queued: ${res.ws_queued})`;
			await loadCommands(true);
		} catch (err) {
			sendResult = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			sendLoading = false;
		}
	}

	function statusBadge(status: string): string {
		switch (status) {
			case 'completed': return 'bg-green-50 text-green-600';
			case 'delivered': return 'bg-blue-50 text-blue-600';
			case 'failed': return 'bg-red-50 text-red-600';
			default: return 'bg-amber-50 text-amber-600';
		}
	}

	function statusLabel(status: string): string {
		switch (status) {
			case 'pending': return $t('commands.pending');
			case 'delivered': return $t('commands.delivered');
			case 'completed': return $t('commands.cmd_completed');
			case 'failed': return $t('commands.failed');
			default: return status;
		}
	}
</script>

<svelte:head>
	<title>{$t('commands.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout>
	<div class="max-w-5xl mx-auto px-6 py-8 space-y-6">
		<div class="flex items-center gap-3">
			<a href="/stores/{storeId}" class="text-slate-400 hover:text-slate-600">
				<ArrowLeft class="w-5 h-5" />
			</a>
			<h1 class="font-heading text-xl font-bold text-slate-900">{$t('commands.title')}</h1>
		</div>

		<!-- Send command -->
		<div class="bg-white rounded-2xl border border-slate-200 p-6">
			<h3 class="font-heading font-bold text-slate-900 mb-3">{$t('commands.send')}</h3>
			<div class="flex items-center gap-3">
				<select
					bind:value={sendType}
					class="px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
				>
					<option value="sync">sync</option>
					<option value="restart">restart</option>
					<option value="get_status">get_status</option>
				</select>
				<button
					onclick={handleSend}
					disabled={sendLoading}
					class="inline-flex items-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-medium text-sm px-4 py-2 rounded-lg cursor-pointer disabled:opacity-50"
				>
					<Send class="w-4 h-4" />
					<span>{$t('commands.send')}</span>
				</button>
			</div>
			{#if sendResult}
				<p class="mt-2 text-sm text-slate-600">{sendResult}</p>
			{/if}
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if commands.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<Terminal class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('commands.empty')}</p>
			</div>
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-slate-100 text-left">
							<th class="px-4 py-3 text-xs font-medium text-slate-400">ID</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('commands.type')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('commands.status')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('commands.created')}</th>
							<th class="px-4 py-3 text-xs font-medium text-slate-400">{$t('commands.executed')}</th>
						</tr>
					</thead>
					<tbody>
						{#each commands as cmd}
							<tr class="border-b border-slate-50">
								<td class="px-4 py-2.5 text-slate-500">#{cmd.id}</td>
								<td class="px-4 py-2.5 font-medium text-slate-900">{cmd.command_type}</td>
								<td class="px-4 py-2.5">
									<span class="inline-flex px-2 py-0.5 rounded-full text-xs font-medium {statusBadge(cmd.status)}">
										{statusLabel(cmd.status)}
									</span>
								</td>
								<td class="px-4 py-2.5 text-slate-500">{formatDateTime(cmd.created_at)}</td>
								<td class="px-4 py-2.5 text-slate-500">{cmd.executed_at ? formatDateTime(cmd.executed_at) : '—'}</td>
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
						{loadingMore ? $t('auth.loading') : $t('commands.load_more')}
					</button>
				</div>
			{/if}
		{/if}
	</div>
</ConsoleLayout>
