<script lang="ts">
	import { XCircle, ArrowRight, ArrowLeft } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import AuthLayout from '$lib/components/AuthLayout.svelte';
</script>

<svelte:head>
	<title>{$t('reg_cancel.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('reg_cancel.title')} subtitle="">
	<div class="text-center space-y-6">
		<div class="w-16 h-16 mx-auto bg-amber-100 rounded-full flex items-center justify-center">
			<XCircle class="w-8 h-8 text-amber-600" />
		</div>
		<div class="space-y-2">
			<p class="text-sm text-slate-600">{$t('reg_cancel.message')}</p>
			<p class="text-xs text-slate-400">{$t('reg_cancel.hint')}</p>
		</div>
		<div class="flex flex-col sm:flex-row items-center justify-center gap-3">
			<a
				href="/"
				class="inline-flex items-center gap-1.5 text-sm text-slate-500 hover:text-slate-700 transition-colors duration-150"
			>
				<ArrowLeft class="w-4 h-4" />
				<span>{$t('reg_cancel.back_home')}</span>
			</a>
			<a
				href="https://console.redcoral.app/login"
				class="inline-flex items-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-semibold text-sm px-6 py-2.5 rounded-lg transition-colors duration-150 cursor-pointer"
			>
				<span>{$t('reg_cancel.try_login')}</span>
				<ArrowRight class="w-4 h-4" />
			</a>
		</div>
	</div>
</AuthLayout>
