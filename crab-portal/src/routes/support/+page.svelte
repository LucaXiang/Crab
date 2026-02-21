<script lang="ts">
	import { t } from '$lib/i18n';
	import { Mail, MessageCircle, FileText, ArrowLeft } from 'lucide-svelte';
	import AuthLayout from '$lib/components/AuthLayout.svelte';
</script>

<svelte:head>
	<title>{$t('support.page_title')} — RedCoral</title>
</svelte:head>

<AuthLayout title={$t('support.title')} subtitle={$t('support.subtitle')}>
	<div class="space-y-4">
		<a
			href="mailto:soporte@redcoral.app"
			class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 hover:border-coral-300 hover:bg-coral-50/30 transition-all duration-150 group"
		>
			<div
				class="w-10 h-10 rounded-lg bg-coral-50 text-coral-500 flex items-center justify-center group-hover:bg-coral-100 transition-colors"
			>
				<Mail class="w-5 h-5" />
			</div>
			<div>
				<p class="text-sm font-semibold text-slate-900">{$t('support.email_title')}</p>
				<p class="text-xs text-slate-500">soporte@redcoral.app</p>
			</div>
		</a>

		<a
			href="https://wa.me/34600000000"
			target="_blank"
			rel="noopener noreferrer"
			class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 hover:border-emerald-300 hover:bg-emerald-50/30 transition-all duration-150 group"
		>
			<div
				class="w-10 h-10 rounded-lg bg-emerald-50 text-emerald-500 flex items-center justify-center group-hover:bg-emerald-100 transition-colors"
			>
				<MessageCircle class="w-5 h-5" />
			</div>
			<div>
				<p class="text-sm font-semibold text-slate-900">{$t('support.whatsapp_title')}</p>
				<p class="text-xs text-slate-500">{$t('support.whatsapp_desc')}</p>
			</div>
		</a>

		<a
			href="/#faq"
			class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 hover:border-blue-300 hover:bg-blue-50/30 transition-all duration-150 group"
		>
			<div
				class="w-10 h-10 rounded-lg bg-blue-50 text-blue-500 flex items-center justify-center group-hover:bg-blue-100 transition-colors"
			>
				<FileText class="w-5 h-5" />
			</div>
			<div>
				<p class="text-sm font-semibold text-slate-900">{$t('support.faq_title')}</p>
				<p class="text-xs text-slate-500">{$t('support.faq_desc')}</p>
			</div>
		</a>
	</div>

	<div class="mt-6 text-center">
		<a
			href="/"
			class="inline-flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-600 transition-colors"
		>
			<ArrowLeft class="w-3.5 h-3.5" />
			{$t('support.back_home')}
		</a>
	</div>
</AuthLayout>
