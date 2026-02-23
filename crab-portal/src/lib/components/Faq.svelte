<script lang="ts">
	import { ChevronDown, AlertTriangle } from 'lucide-svelte';
	import { t } from '$lib/i18n';

	const faqs = [
		{ q: 'faq.q1', a: 'faq.a1' },
		{ q: 'faq.q2', a: 'faq.a2' },
		{ q: 'faq.q3', a: 'faq.a3' },
		{ q: 'faq.q4', a: 'faq.a4' },
		{ q: 'faq.q5', a: 'faq.a5' }
	];
</script>

<section id="faq" class="py-24 md:py-32 bg-slate-50">
	<div class="max-w-3xl mx-auto px-6">
		<div class="mb-14">
			<p class="text-coral-500 text-sm font-semibold tracking-wide uppercase mb-3">FAQ</p>
			<h2 class="font-heading text-3xl md:text-4xl font-bold text-slate-900 mb-4">
				{$t('faq.title')}
			</h2>
			<p class="text-slate-500 leading-relaxed">{$t('faq.subtitle')}</p>
		</div>

		<!-- Anxiety banner -->
		<div class="mb-10 bg-amber-50 border border-amber-200/60 rounded-xl p-5 flex gap-4">
			<div class="shrink-0 mt-0.5">
				<AlertTriangle class="w-5 h-5 text-amber-500" />
			</div>
			<div>
				<p class="text-sm font-semibold text-amber-800 mb-1">{$t('faq.anxiety_title')}</p>
				<p class="text-sm text-amber-700/80 leading-relaxed">{$t('faq.anxiety_desc')}</p>
			</div>
		</div>

		<div class="bg-white rounded-2xl border border-slate-200 shadow-sm divide-y divide-slate-100">
			{#each faqs as faq}
				<details class="group">
					<summary
						class="flex items-center justify-between px-6 py-5 cursor-pointer list-none text-[15px] font-medium text-slate-900 hover:text-coral-500 transition-colors duration-150"
					>
						<span>{$t(faq.q)}</span>
						<ChevronDown
							class="w-4 h-4 text-slate-400 shrink-0 ml-4 group-open:rotate-180 transition-transform duration-200"
						/>
					</summary>
					<div class="px-6 pb-5 text-sm text-slate-500 leading-relaxed">{$t(faq.a)}</div>
				</details>
			{/each}
		</div>
	</div>
</section>
