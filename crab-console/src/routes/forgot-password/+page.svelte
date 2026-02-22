<script lang="ts">
	import { goto } from '$app/navigation';
	import { Mail, ArrowLeft, ArrowRight, Lock, KeyRound } from 'lucide-svelte';
	import { t, apiErrorMessage } from '$lib/i18n';
	import { forgotPassword, resetPassword, ApiError } from '$lib/api';

	let email = $state('');
	let code = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let loading = $state(false);
	let error = $state('');
	let success = $state('');
	let step = $state<'email' | 'reset'>('email');

	async function handleSendCode(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			await forgotPassword(email);
			success = $t('auth.reset_code_sent');
			step = 'reset';
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

	async function handleReset(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		success = '';
		loading = true;

		if (newPassword !== confirmPassword) {
			error = $t('auth.password_mismatch');
			loading = false;
			return;
		}

		try {
			await resetPassword(email, code, newPassword);
			success = $t('auth.password_reset_success');
			setTimeout(() => goto('/login'), 1500);
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
	<title>{$t('auth.forgot')} — RedCoral Console</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-slate-50 px-4">
	<div class="w-full max-w-sm">
		<div class="text-center mb-8">
			<div class="w-12 h-12 bg-coral-500 rounded-xl flex items-center justify-center mx-auto mb-4">
				<svg viewBox="0 0 24 24" fill="none" class="w-6 h-6 text-white" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
					<path d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z" />
					<line x1="6" y1="17" x2="18" y2="17" />
				</svg>
			</div>
			<h1 class="font-heading text-2xl font-bold text-slate-900">{$t('auth.forgot')}</h1>
			<p class="text-sm text-slate-500 mt-1">{step === 'email' ? $t('auth.forgot_desc') : $t('auth.reset_desc')}</p>
		</div>

		{#if error}
			<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{/if}
		{#if success}
			<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600">{success}</div>
		{/if}

		{#if step === 'email'}
			<form class="space-y-4" onsubmit={handleSendCode}>
				<div>
					<label for="email" class="block text-sm font-medium text-slate-700 mb-1.5">{$t('auth.email')}</label>
					<div class="relative">
						<Mail class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
						<input
							type="email"
							id="email"
							required
							bind:value={email}
							placeholder={$t('auth.email')}
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
						<span>{$t('auth.send_code')}</span>
						<ArrowRight class="w-4 h-4" />
					{/if}
				</button>
			</form>
		{:else}
			<form class="space-y-4" onsubmit={handleReset}>
				<div>
					<label for="code" class="block text-sm font-medium text-slate-700 mb-1.5">{$t('auth.reset_code')}</label>
					<div class="relative">
						<KeyRound class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
						<input
							type="text"
							id="code"
							required
							bind:value={code}
							placeholder="000000"
							class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
						/>
					</div>
				</div>

				<div>
					<label for="new-password" class="block text-sm font-medium text-slate-700 mb-1.5">{$t('settings.new_password')}</label>
					<div class="relative">
						<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
						<input
							type="password"
							id="new-password"
							required
							minlength="8"
							bind:value={newPassword}
							placeholder={$t('settings.new_password')}
							class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
						/>
					</div>
				</div>

				<div>
					<label for="confirm-password" class="block text-sm font-medium text-slate-700 mb-1.5">{$t('auth.password_confirm')}</label>
					<div class="relative">
						<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
						<input
							type="password"
							id="confirm-password"
							required
							minlength="8"
							bind:value={confirmPassword}
							placeholder={$t('auth.password_confirm')}
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
						<span>{$t('auth.reset_password')}</span>
						<ArrowRight class="w-4 h-4" />
					{/if}
				</button>
			</form>
		{/if}

		<p class="text-center text-sm text-slate-500 mt-6">
			<a href="/login" class="inline-flex items-center gap-1 text-coral-500 hover:text-coral-600 font-medium">
				<ArrowLeft class="w-3.5 h-3.5" />
				{$t('auth.back_to_login')}
			</a>
		</p>
	</div>
</div>
