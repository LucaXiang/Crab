<script lang="ts">
	import { goto } from '$app/navigation';
	import { Mail, Lock, Eye, EyeOff, ArrowRight } from 'lucide-svelte';
	import { t, apiErrorMessage } from '$lib/i18n';
	import { register, ApiError } from '$lib/api';
	import AuthLayout from '$lib/components/AuthLayout.svelte';

	let email = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let showPassword = $state(false);
	let loading = $state(false);
	let error = $state('');
	let termsAccepted = $state(false);

	// Plan from URL param (from pricing CTA), hidden from user
	let selectedPlan = $state('basic');
	if (typeof window !== 'undefined') {
		const params = new URLSearchParams(window.location.search);
		const plan = params.get('plan');
		if (plan && ['basic', 'pro'].includes(plan)) {
			selectedPlan = plan;
		}
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;

		if (password !== confirmPassword) {
			error = $t('auth.password_mismatch');
			loading = false;
			return;
		}

		try {
			const res = await register({ email, password, plan: selectedPlan });
			if (res.status === 'verified') {
				// Already verified — redirect to login
				goto('/login?email=' + encodeURIComponent(email) + '&verified=1');
			} else {
				sessionStorage.setItem('redcoral-verify-email', email);
				goto('/verify');
			}
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
	<title>{$t('register.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('register.title')} subtitle={$t('register.subtitle')}>
	{#if error}
		<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
			{error}
		</div>
	{/if}

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

		<div>
			<label for="password" class="block text-sm font-medium text-slate-700 mb-1.5"
				>{$t('auth.label_password')}</label
			>
			<div class="relative">
				<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
				<input
					type={showPassword ? 'text' : 'password'}
					id="password"
					required
					minlength={8}
					bind:value={password}
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
				>{$t('auth.label_password_confirm')}</label
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

		<div class="flex items-start gap-2">
			<input
				type="checkbox"
				id="terms"
				required
				bind:checked={termsAccepted}
				class="mt-0.5 w-4 h-4 accent-coral-500 border-slate-300 rounded cursor-pointer"
			/>
			<label for="terms" class="text-xs text-slate-500 cursor-pointer leading-relaxed">
				{$t('signup.terms_pre')}<a
					href="/terms"
					target="_blank"
					class="text-coral-500 hover:text-coral-600 font-medium underline underline-offset-2"
					>{$t('signup.terms_link')}</a
				>{$t('signup.terms_mid')}<a
					href="/privacy"
					target="_blank"
					class="text-coral-500 hover:text-coral-600 font-medium underline underline-offset-2"
					>{$t('signup.privacy_link')}</a
				>
			</label>
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
				<span>{$t('register.cta')}</span>
				<ArrowRight class="w-4 h-4" />
			{/if}
		</button>

		<p class="text-center text-xs text-slate-400">
			{$t('register.has_account')}
			<a href="/login" class="text-coral-500 hover:text-coral-600 font-medium">{$t('auth.login_link')}</a>
		</p>
	</form>
</AuthLayout>
