<script lang="ts">
	import { goto } from '$app/navigation';
	import { Mail, Lock, ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { login, ApiError } from '$lib/api';
	import { setAuth, isAuthenticated } from '$lib/auth';
	import { onMount } from 'svelte';

	let email = $state('');
	let password = $state('');
	let loading = $state(false);
	let error = $state('');

	onMount(() => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (authenticated) goto('/');
	});

	async function handleLogin(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const res = await login(email, password);
			setAuth(res.token, res.tenant_id);
			goto('/');
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				error = $t('auth.error_invalid');
			} else {
				error = err instanceof ApiError ? err.message : $t('auth.error_generic');
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>{$t('auth.login_title')} — RedCoral Console</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-slate-50 px-4">
	<div class="w-full max-w-sm">
		<!-- Logo -->
		<div class="text-center mb-8">
			<div
				class="w-12 h-12 bg-coral-500 rounded-xl flex items-center justify-center mx-auto mb-4"
			>
				<svg
					viewBox="0 0 24 24"
					fill="none"
					class="w-6 h-6 text-white"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z"
					/>
					<line x1="6" y1="17" x2="18" y2="17" />
				</svg>
			</div>
			<h1 class="font-heading text-2xl font-bold text-slate-900">{$t('auth.login_title')}</h1>
			<p class="text-sm text-slate-500 mt-1">{$t('auth.login_subtitle')}</p>
		</div>

		{#if error}
			<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">
				{error}
			</div>
		{/if}

		<form class="space-y-4" onsubmit={handleLogin}>
			<div>
				<label for="email" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('auth.email')}</label
				>
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

			<div>
				<label for="password" class="block text-sm font-medium text-slate-700 mb-1.5"
					>{$t('auth.password')}</label
				>
				<div class="relative">
					<Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
					<input
						type="password"
						id="password"
						required
						bind:value={password}
						placeholder={$t('auth.password')}
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
					<svg
						class="animate-spin w-4 h-4"
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
					<span>{$t('auth.loading')}</span>
				{:else}
					<span>{$t('auth.login_cta')}</span>
					<ArrowRight class="w-4 h-4" />
				{/if}
			</button>
		</form>

			<div class="text-center mt-4">
			<a href="/forgot-password" class="text-sm text-slate-500 hover:text-coral-500 transition-colors">{$t('auth.forgot')}</a>
		</div>

		<p class="text-center text-sm text-slate-500 mt-4">
			{$t('auth.no_account')}
			<a href="https://redcoral.app/register" class="text-coral-500 hover:text-coral-600 font-medium"
				>{$t('auth.register')}</a
			>
		</p>
	</div>
</div>
