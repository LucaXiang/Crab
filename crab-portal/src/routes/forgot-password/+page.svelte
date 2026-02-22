<script lang="ts">
	import { Mail, ArrowRight, ArrowLeft } from 'lucide-svelte';
	import { t, apiErrorMessage } from '$lib/i18n';
	import { forgotPassword, ApiError } from '$lib/api';
	import AuthLayout from '$lib/components/AuthLayout.svelte';

	let email = $state('');
	let loading = $state(false);
	let sent = $state(false);
	let error = $state('');

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			await forgotPassword(email);
			sent = true;
		} catch (err) {
			if (err instanceof ApiError) {
				error = apiErrorMessage($t, err.code, err.message);
			} else {
				error = $t('auth.error_generic');
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>{$t('forgot.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('forgot.title')} subtitle={$t('forgot.subtitle')}>
	{#if error}
		<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
			{error}
		</div>
	{/if}

	{#if sent}
		<div class="text-center space-y-4">
			<div class="w-12 h-12 mx-auto bg-green-100 rounded-full flex items-center justify-center">
				<Mail class="w-6 h-6 text-green-600" />
			</div>
			<p class="text-sm text-slate-600">{$t('forgot.sent_message')}</p>
			<button
				onclick={() => { sessionStorage.setItem('redcoral-reset-email', email); window.location.href = '/reset-password'; }}
				class="inline-flex items-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-semibold text-sm px-6 py-2.5 rounded-lg transition-colors duration-150 cursor-pointer"
			>
				<span>{$t('forgot.enter_code')}</span>
				<ArrowRight class="w-4 h-4" />
			</button>
		</div>
	{:else}
		<form class="space-y-4" onsubmit={handleSubmit}>
			<div>
				<label for="email" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('auth.label_email')}</label
				>
				<div class="relative">
					<Mail class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
					<input
						type="email"
						id="email"
						required
						bind:value={email}
						placeholder={$t('auth.placeholder_email')}
						class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
					/>
				</div>
			</div>

			<button
				type="submit"
				disabled={loading}
				class="w-full bg-coral-500 hover:bg-coral-600 text-white font-semibold py-3 rounded-lg transition-colors duration-150 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed"
			>
				{#if loading}
					<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
					</svg>
					<span>{$t('auth.loading')}</span>
				{:else}
					<span>{$t('forgot.cta')}</span>
					<ArrowRight class="w-4 h-4" />
				{/if}
			</button>

			<p class="text-center">
				<a href="/login" class="inline-flex items-center gap-1 text-xs text-slate-500 hover:text-slate-700">
					<ArrowLeft class="w-3.5 h-3.5" />
					<span>{$t('forgot.back_to_login')}</span>
				</a>
			</p>
		</form>
	{/if}
</AuthLayout>
