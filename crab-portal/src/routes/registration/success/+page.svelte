<script lang="ts">
	import { CheckCircle, ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import AuthLayout from '$lib/components/AuthLayout.svelte';
</script>

<svelte:head>
	<title>{$t('reg_success.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('reg_success.title')} subtitle="">
	<div class="text-center space-y-6">
		<div class="w-16 h-16 mx-auto bg-green-100 rounded-full flex items-center justify-center">
			<CheckCircle class="w-8 h-8 text-green-600" />
		</div>
		<div class="space-y-2">
			<p class="text-sm text-slate-600">{$t('reg_success.message')}</p>
			<p class="text-xs text-slate-400">{$t('reg_success.next_steps')}</p>
		</div>
		<a
			href="/login"
			class="inline-flex items-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-semibold text-sm px-6 py-3 rounded-lg transition-colors duration-150 cursor-pointer"
		>
			<span>{$t('reg_success.login_cta')}</span>
			<ArrowRight class="w-4 h-4" />
		</a>
	</div>
</AuthLayout>
