<script lang="ts">
	import { goto } from '$app/navigation';
	import { KeyRound, Lock, Eye, EyeOff, ArrowRight } from 'lucide-svelte';
	import { t, apiErrorMessage } from '$lib/i18n';
	import { resetPassword, ApiError } from '$lib/api';
	import AuthLayout from '$lib/components/AuthLayout.svelte';

	let email = $state('');
	let code = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let showPassword = $state(false);
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	// Read email from sessionStorage (not URL)
	if (typeof window !== 'undefined') {
		email = sessionStorage.getItem('redcoral-reset-email') ?? '';
	}

	function handleCodeInput(e: Event) {
		const input = e.target as HTMLInputElement;
		code = input.value.replace(/\D/g, '').slice(0, 6);
		input.value = code;
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;

		if (newPassword !== confirmPassword) {
			error = $t('auth.password_mismatch');
			loading = false;
			return;
		}

		try {
			await resetPassword({ email, code, new_password: newPassword });
			sessionStorage.removeItem('redcoral-reset-email');
			success = true;
			setTimeout(() => goto('/login'), 2000);
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
	<title>{$t('reset.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('reset.title')} subtitle={$t('reset.subtitle')}>
	{#if error}
		<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
			{error}
		</div>
	{/if}

	{#if success}
		<div class="text-center space-y-4">
			<div class="w-12 h-12 mx-auto bg-green-100 rounded-full flex items-center justify-center">
				<Lock class="w-6 h-6 text-green-600" />
			</div>
			<p class="text-sm text-slate-600">{$t('reset.success')}</p>
			<p class="text-xs text-slate-400">{$t('reset.redirecting')}</p>
		</div>
	{:else}
		<form class="space-y-4" onsubmit={handleSubmit}>
			{#if !email}
				<div>
					<label for="email" class="block text-sm font-medium text-slate-700 mb-1.5"
						>{$t('auth.label_email')}</label
					>
					<input
						type="email"
						id="email"
						required
						bind:value={email}
						placeholder={$t('auth.placeholder_email')}
						class="w-full px-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
					/>
				</div>
			{/if}

			<div>
				<label for="code" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('reset.label_code')}</label
				>
				<div class="relative">
					<KeyRound class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
					<input
						type="text"
						id="code"
						required
						maxlength={6}
						inputmode="numeric"
						autocomplete="one-time-code"
						value={code}
						oninput={handleCodeInput}
						placeholder="000000"
						class="w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150 tracking-[0.3em] text-center font-mono text-lg"
					/>
				</div>
			</div>

			<div>
				<label for="new-password" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('reset.label_new_password')}</label
				>
				<div class="relative">
					<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
					<input
						type={showPassword ? 'text' : 'password'}
						id="new-password"
						required
						minlength={8}
						bind:value={newPassword}
						placeholder={$t('auth.placeholder_password')}
						class="w-full pl-10 pr-10 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
					/>
					<button
						type="button"
						onclick={() => (showPassword = !showPassword)}
						class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 cursor-pointer transition-colors duration-150"
					>
						{#if showPassword}
							<EyeOff class="w-4 h-4" />
						{:else}
							<Eye class="w-4 h-4" />
						{/if}
					</button>
				</div>
			</div>

			<div>
				<label for="confirm-password" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('auth.password_confirm')}</label
				>
				<div class="relative">
					<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
					<input
						type={showPassword ? 'text' : 'password'}
						id="confirm-password"
						required
						minlength={8}
						bind:value={confirmPassword}
						placeholder={$t('auth.placeholder_password')}
						class="w-full pl-10 pr-10 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500 transition-all duration-150"
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
					<span>{$t('auth.loading')}</span>
				{:else}
					<span>{$t('reset.cta')}</span>
					<ArrowRight class="w-4 h-4" />
				{/if}
			</button>
		</form>
	{/if}
</AuthLayout>
