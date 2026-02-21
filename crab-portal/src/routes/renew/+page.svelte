<script lang="ts">
	import { t } from '$lib/i18n';
	import { CreditCard, ArrowRight, ArrowLeft, Shield } from 'lucide-svelte';
	import AuthLayout from '$lib/components/AuthLayout.svelte';
</script>

<svelte:head>
	<title>{$t('renew.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('renew.title')} subtitle={$t('renew.subtitle')}>
	<div class="space-y-5">
		<div class="p-4 bg-amber-50 border border-amber-200 rounded-xl">
			<p class="text-sm text-amber-800 font-medium">{$t('renew.notice')}</p>
		</div>

		<div class="space-y-3">
			<h3 class="text-sm font-semibold text-slate-700">{$t('renew.steps_title')}</h3>
			<ol class="space-y-2.5 text-sm text-slate-600">
				<li class="flex items-start gap-3">
					<span
						class="w-5 h-5 rounded-full bg-coral-100 text-coral-600 text-xs font-bold flex items-center justify-center shrink-0 mt-0.5"
						>1</span
					>
					{$t('renew.step1')}
				</li>
				<li class="flex items-start gap-3">
					<span
						class="w-5 h-5 rounded-full bg-coral-100 text-coral-600 text-xs font-bold flex items-center justify-center shrink-0 mt-0.5"
						>2</span
					>
					{$t('renew.step2')}
				</li>
				<li class="flex items-start gap-3">
					<span
						class="w-5 h-5 rounded-full bg-coral-100 text-coral-600 text-xs font-bold flex items-center justify-center shrink-0 mt-0.5"
						>3</span
					>
					{$t('renew.step3')}
				</li>
			</ol>
		</div>

		<a
			href="https://console.redcoral.app"
			class="w-full bg-coral-500 hover:bg-coral-600 text-white font-semibold py-3 rounded-lg transition-colors duration-150 cursor-pointer flex items-center justify-center gap-2"
		>
			<CreditCard class="w-4 h-4" />
			<span>{$t('renew.cta')}</span>
			<ArrowRight class="w-4 h-4" />
		</a>

		<div class="flex items-center justify-center gap-1.5 text-xs text-slate-400">
			<Shield class="w-3.5 h-3.5" />
			<span>{$t('renew.secure')}</span>
		</div>
	</div>

	<div class="mt-6 text-center">
		<a
			href="/"
			class="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-600 transition-colors"
		>
			<ArrowLeft class="w-3.5 h-3.5" />
			{$t('renew.back_home')}
		</a>
	</div>
</AuthLayout>
