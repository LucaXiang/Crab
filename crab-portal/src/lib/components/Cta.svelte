<script lang="ts">
	import { ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<section class="bg-slate-900 py-20 md:py-28 relative overflow-hidden grain">
	<div class="absolute inset-0">
		<div
			class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[300px] bg-coral-500/8 rounded-full blur-[100px]"
		></div>
	</div>
	<div class="relative max-w-3xl mx-auto px-6 text-center">
		<h2 class="font-heading text-3xl md:text-4xl font-bold text-white mb-4">
			{$t('cta.title')}
		</h2>
		<p class="text-slate-400 mb-10 max-w-xl mx-auto">{$t('cta.subtitle')}</p>
		<a
			href="/register"
			class="inline-flex items-center gap-2 bg-coral-500 hover:bg-coral-600 text-white font-semibold px-8 py-3.5 rounded-xl transition-colors duration-150 cursor-pointer"
		>
			<span>{$t('cta.button')}</span>
			<ArrowRight class="w-4 h-4" />
		</a>
	</div>
</section>
