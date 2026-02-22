<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { AlertTriangle, ArrowLeft, Home } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<svelte:head>
	<title>{page.status === 404 ? $t('error.not_found') : $t('error.title')} — RedCoral</title>
</svelte:head>

<div class="min-h-screen bg-slate-50 flex items-center justify-center px-6">
	<div class="max-w-md w-full text-center space-y-6">
		<div class="w-16 h-16 bg-red-50 rounded-2xl flex items-center justify-center mx-auto">
			<AlertTriangle class="w-8 h-8 text-red-400" />
		</div>

		<div>
			<p class="text-5xl font-heading font-bold text-slate-900 mb-2">{page.status}</p>
			{#if page.status === 404}
				<p class="text-sm text-slate-500">{$t('error.not_found_desc')}</p>
			{:else}
				<p class="text-sm text-slate-500">{page.error?.message ?? $t('error.generic_desc')}</p>
			{/if}
		</div>

		<div class="flex items-center justify-center gap-3">
			<button
				onclick={() => history.back()}
				class="inline-flex items-center gap-1.5 px-4 py-2 bg-white border border-slate-200 rounded-lg text-sm font-medium text-slate-600 hover:bg-slate-50 transition-colors cursor-pointer"
			>
				<ArrowLeft class="w-4 h-4" />
				{$t('error.go_back')}
			</button>
			<button
				onclick={() => goto('/')}
				class="inline-flex items-center gap-1.5 px-4 py-2 bg-coral-500 hover:bg-coral-600 text-white rounded-lg text-sm font-medium transition-colors cursor-pointer"
			>
				<Home class="w-4 h-4" />
				{$t('error.go_home')}
			</button>
		</div>
	</div>
</div>
