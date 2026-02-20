<script lang="ts">
	import { ChevronDown } from 'lucide-svelte';
	import { t } from '$lib/i18n';

	const faqs = [
		{ q: 'faq.q1', a: 'faq.a1' },
		{ q: 'faq.q2', a: 'faq.a2' },
		{ q: 'faq.q3', a: 'faq.a3' },
		{ q: 'faq.q4', a: 'faq.a4' },
		{ q: 'faq.q5', a: 'faq.a5' }
	];
</script>

<section id="faq" class="py-24 md:py-32 bg-white">
	<div class="max-w-2xl mx-auto px-6">
		<div class="mb-12">
			<p class="text-coral-500 text-sm font-semibold tracking-wide uppercase mb-3">FAQ</p>
			<h2 class="font-heading text-3xl md:text-4xl font-bold text-slate-900">
				{$t('faq.title')}
			</h2>
		</div>

		<div class="divide-y divide-slate-100">
			{#each faqs as faq}
				<details class="group">
					<summary
						class="flex items-center justify-between py-5 cursor-pointer list-none text-[15px] font-medium text-slate-900 hover:text-coral-500 transition-colors duration-150"
					>
						<span>{$t(faq.q)}</span>
						<ChevronDown
							class="w-4 h-4 text-slate-400 shrink-0 ml-4 group-open:rotate-180 transition-transform duration-200"
						/>
					</summary>
					<div class="pb-5 text-sm text-slate-500 leading-relaxed">{$t(faq.a)}</div>
				</details>
			{/each}
		</div>
	</div>
</section>
