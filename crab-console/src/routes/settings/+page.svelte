<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Lock, Mail, CreditCard, Check } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getProfile,
		updateProfile,
		changePassword,
		changeEmail,
		confirmEmailChange,
		createBillingPortal,
		ApiError,
		type ProfileResponse
	} from '$lib/api';
	import ConsoleLayout from '$lib/components/ConsoleLayout.svelte';

	let profile = $state<ProfileResponse | null>(null);
	let loading = $state(true);
	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	// Profile
	let profileName = $state('');
	let profileSaving = $state(false);
	let profileMsg = $state('');

	// Password
	let currentPassword = $state('');
	let newPassword = $state('');
	let pwSaving = $state(false);
	let pwMsg = $state('');
	let pwError = $state('');

	// Email
	let emailPassword = $state('');
	let newEmail = $state('');
	let emailCode = $state('');
	let emailStep = $state<'form' | 'verify'>('form');
	let emailSaving = $state(false);
	let emailMsg = $state('');
	let emailError = $state('');

	// Billing
	let billingLoading = $state(false);

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) {
			goto('/login');
			return;
		}
		try {
			profile = await getProfile(token);
			profileName = profile.profile.name ?? '';
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				clearAuth();
				goto('/login');
			}
		} finally {
			loading = false;
		}
	});

	async function handleSaveProfile() {
		profileSaving = true;
		profileMsg = '';
		try {
			await updateProfile(token, profileName);
			profileMsg = $t('settings.saved');
		} catch {
			// ignore
		} finally {
			profileSaving = false;
		}
	}

	async function handleChangePassword(e: SubmitEvent) {
		e.preventDefault();
		pwSaving = true;
		pwMsg = '';
		pwError = '';
		try {
			await changePassword(token, currentPassword, newPassword);
			pwMsg = $t('settings.password_changed');
			currentPassword = '';
			newPassword = '';
		} catch (err) {
			pwError = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			pwSaving = false;
		}
	}

	async function handleChangeEmail(e: SubmitEvent) {
		e.preventDefault();
		emailSaving = true;
		emailError = '';
		emailMsg = '';
		try {
			if (emailStep === 'form') {
				await changeEmail(token, emailPassword, newEmail);
				emailStep = 'verify';
				emailMsg = $t('settings.email_code_sent');
			} else {
				await confirmEmailChange(token, newEmail, emailCode);
				emailMsg = $t('settings.email_changed');
				emailStep = 'form';
				emailPassword = '';
				newEmail = '';
				emailCode = '';
				// Refresh profile
				profile = await getProfile(token);
			}
		} catch (err) {
			emailError = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			emailSaving = false;
		}
	}

	async function handleBillingPortal() {
		billingLoading = true;
		try {
			const res = await createBillingPortal(token);
			window.location.href = res.url;
		} catch {
			// ignore
		} finally {
			billingLoading = false;
		}
	}
</script>

<svelte:head>
	<title>{$t('settings.title')} — RedCoral Console</title>
</svelte:head>

<ConsoleLayout email={profile?.profile.email ?? ''}>
	<div class="max-w-2xl mx-auto px-6 py-8 space-y-8">
		<h1 class="font-heading text-xl font-bold text-slate-900">{$t('settings.title')}</h1>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else}
			<!-- Profile -->
			<section class="bg-white rounded-2xl border border-slate-200 p-6">
				<h2 class="font-heading font-bold text-base text-slate-900 mb-4">{$t('settings.profile')}</h2>
				<div class="space-y-4">
					<div>
						<label for="name" class="block text-sm font-medium text-slate-700 mb-1">{$t('settings.name')}</label>
						<div class="flex gap-3">
							<input
								type="text"
								id="name"
								bind:value={profileName}
								placeholder={$t('settings.name')}
								class="flex-1 px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
							/>
							<button
								onclick={handleSaveProfile}
								disabled={profileSaving}
								class="px-4 py-2 bg-coral-500 hover:bg-coral-600 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-60"
							>
								{$t('settings.save')}
							</button>
						</div>
						{#if profileMsg}
							<p class="text-xs text-green-600 mt-1 flex items-center gap-1"><Check class="w-3 h-3" />{profileMsg}</p>
						{/if}
					</div>
					<div>
						<label class="block text-sm font-medium text-slate-700 mb-1">{$t('settings.email')}</label>
						<p class="text-sm text-slate-600">{profile?.profile.email}</p>
					</div>
				</div>
			</section>

			<!-- Change password -->
			<section class="bg-white rounded-2xl border border-slate-200 p-6">
				<h2 class="font-heading font-bold text-base text-slate-900 mb-4 flex items-center gap-2">
					<Lock class="w-4 h-4 text-slate-500" />
					{$t('settings.change_password')}
				</h2>
				{#if pwMsg}
					<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600">{pwMsg}</div>
				{/if}
				{#if pwError}
					<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{pwError}</div>
				{/if}
				<form class="space-y-3" onsubmit={handleChangePassword}>
					<input
						type="password"
						bind:value={currentPassword}
						placeholder={$t('settings.current_password')}
						required
						class="w-full px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
					/>
					<input
						type="password"
						bind:value={newPassword}
						placeholder={$t('settings.new_password')}
						required
						minlength="8"
						class="w-full px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
					/>
					<button
						type="submit"
						disabled={pwSaving}
						class="px-4 py-2 bg-slate-800 hover:bg-slate-900 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-60"
					>
						{$t('settings.change_password')}
					</button>
				</form>
			</section>

			<!-- Change email -->
			<section class="bg-white rounded-2xl border border-slate-200 p-6">
				<h2 class="font-heading font-bold text-base text-slate-900 mb-4 flex items-center gap-2">
					<Mail class="w-4 h-4 text-slate-500" />
					{$t('settings.change_email')}
				</h2>
				{#if emailMsg}
					<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600">{emailMsg}</div>
				{/if}
				{#if emailError}
					<div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{emailError}</div>
				{/if}
				<form class="space-y-3" onsubmit={handleChangeEmail}>
					{#if emailStep === 'form'}
						<input
							type="password"
							bind:value={emailPassword}
							placeholder={$t('settings.current_password')}
							required
							class="w-full px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
						/>
						<input
							type="email"
							bind:value={newEmail}
							placeholder={$t('settings.new_email')}
							required
							class="w-full px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
						/>
					{:else}
						<p class="text-sm text-slate-600">{$t('settings.email_code_sent')}</p>
						<input
							type="text"
							bind:value={emailCode}
							placeholder={$t('settings.confirm_code')}
							required
							maxlength="6"
							inputmode="numeric"
							class="w-full px-3 py-2 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-coral-500/20 focus:border-coral-500"
						/>
					{/if}
					<button
						type="submit"
						disabled={emailSaving}
						class="px-4 py-2 bg-slate-800 hover:bg-slate-900 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-60"
					>
						{emailStep === 'form' ? $t('settings.change_email') : $t('settings.save')}
					</button>
				</form>
			</section>

			<!-- Billing -->
			{#if profile?.subscription}
				<section class="bg-white rounded-2xl border border-slate-200 p-6">
					<h2 class="font-heading font-bold text-base text-slate-900 mb-4 flex items-center gap-2">
						<CreditCard class="w-4 h-4 text-slate-500" />
						{$t('settings.billing')}
					</h2>
					<button
						onclick={handleBillingPortal}
						disabled={billingLoading}
						class="px-4 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-60"
					>
						{$t('settings.manage_billing')}
					</button>
				</section>
			{/if}
		{/if}
	</div>
</ConsoleLayout>
