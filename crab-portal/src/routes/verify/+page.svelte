<script lang="ts">
	import { Mail, KeyRound, ArrowRight, RotateCw } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { verifyEmail, resendCode, ApiError } from '$lib/api';
	import AuthLayout from '$lib/components/AuthLayout.svelte';

	let email = $state('');
	let code = $state('');
	let loading = $state(false);
	let resending = $state(false);
	let error = $state('');
	let resendSuccess = $state('');

	// Read email from URL
	if (typeof window !== 'undefined') {
		const params = new URLSearchParams(window.location.search);
		email = params.get('email') ?? '';
	}

	async function handleVerify(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const res = await verifyEmail({ email, code });
			// Redirect to Stripe checkout
			window.location.href = res.checkout_url;
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = $t('auth.error_generic');
			}
		} finally {
			loading = false;
		}
	}

	async function handleResend() {
		error = '';
		resendSuccess = '';
		resending = true;
		try {
			await resendCode(email);
			resendSuccess = $t('verify.resend_success');
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = $t('auth.error_generic');
			}
		} finally {
			resending = false;
		}
	}
</script>

<svelte:head>
	<title>{$t('verify.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('verify.title')} subtitle={$t('verify.subtitle')}>
	{#if error}
		<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
			{error}
		</div>
	{/if}
	{#if resendSuccess}
		<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600">
			{resendSuccess}
		</div>
	{/if}

	<div class="mb-4 p-3 bg-slate-50 border border-slate-200 rounded-lg">
		<div class="flex items-center gap-2 text-sm text-slate-600">
			<Mail class="w-4 h-4 text-slate-400 shrink-0" />
			<span>{$t('verify.sent_to')} <strong class="text-slate-900">{email}</strong></span>
		</div>
	</div>

	<form class="space-y-4" onsubmit={handleVerify}>
		<div>
			<label for="code" class="block text-sm font-medium text-slate-700 mb-1.5"
				>{$t('verify.label_code')}</label
			>
			<div class="relative">
				<KeyRound class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
				<input
					type="text"
					id="code"
					required
					maxlength={6}
					pattern="[0-9]{6}"
					inputmode="numeric"
					bind:value={code}
					placeholder={$t('verify.placeholder_code')}
					class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150 tracking-[0.3em] text-center font-mono text-lg"
				/>
			</div>
		</div>

		<button
			type="submit"
			disabled={loading || code.length !== 6}
			class="w-full bg-coral-500 hover:bg-coral-600 text-white font-semibold py-3 rounded-lg transition-colors duration-150 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed"
		>
			{#if loading}
				<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
				<span>{$t('verify.verifying')}</span>
			{:else}
				<span>{$t('verify.cta')}</span>
				<ArrowRight class="w-4 h-4" />
			{/if}
		</button>

		<div class="text-center">
			<button
				type="button"
				disabled={resending}
				onclick={handleResend}
				class="inline-flex items-center gap-1.5 text-xs text-slate-500 hover:text-coral-500 transition-colors duration-150 cursor-pointer disabled:opacity-50"
			>
				<RotateCw class="w-3.5 h-3.5 {resending ? 'animate-spin' : ''}" />
				<span>{$t('verify.resend')}</span>
			</button>
		</div>
	</form>
</AuthLayout>
