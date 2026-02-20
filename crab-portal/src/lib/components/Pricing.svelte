<script lang="ts">
	import { Check, ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';

	let isAnnual = $state(true);

	function toggleBilling() {
		isAnnual = !isAnnual;
	}
</script>

<section id="pricing" class="py-24 md:py-32 bg-slate-50">
	<div class="max-w-6xl mx-auto px-6">
		<div class="text-center mb-14">
			<p class="text-coral-500 text-sm font-semibold tracking-wide uppercase mb-3">
				{$t('nav.pricing')}
			</p>
			<h2 class="font-heading text-3xl md:text-4xl font-bold text-slate-900 mb-4">
				{$t('pricing.title')}
			</h2>
			<p class="text-slate-500">{$t('pricing.subtitle')}</p>

			<div class="flex items-center justify-center gap-3 mt-8">
				<span
					class="text-sm font-medium transition-colors duration-150"
					class:text-slate-400={isAnnual}
					class:text-slate-900={!isAnnual}>{$t('pricing.monthly')}</span
				>
				<button
					onclick={toggleBilling}
					class="relative w-12 h-6 rounded-full cursor-pointer transition-colors duration-150"
					class:bg-coral-500={isAnnual}
					class:bg-slate-300={!isAnnual}
					aria-label="Toggle billing"
				>
					<span
						class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform duration-200"
						class:translate-x-6={isAnnual}
						class:translate-x-0={!isAnnual}
					></span>
				</button>
				<span
					class="text-sm font-medium transition-colors duration-150"
					class:text-slate-900={isAnnual}
					class:text-slate-400={!isAnnual}
				>
					{$t('pricing.annual')}
					<span class="text-coral-500 text-xs font-bold ml-1">{$t('pricing.discount')}</span>
				</span>
			</div>
		</div>

		<div class="grid md:grid-cols-3 gap-5 max-w-5xl mx-auto">
			<!-- Starter -->
			<div
				class="bg-white border border-slate-200 rounded-2xl p-7 flex flex-col hover:border-slate-300 transition-colors duration-150"
			>
				<div class="mb-6">
					<h3 class="font-heading font-bold text-slate-900 mb-1">
						{$t('pricing.starter.name')}
					</h3>
					<p class="text-sm text-slate-500">{$t('pricing.starter.desc')}</p>
				</div>
				<div class="mb-6">
					<div class="flex items-baseline gap-1">
						<span class="text-4xl font-heading font-bold text-slate-900"
							>&euro;{isAnnual ? 31 : 39}</span
						>
						<span class="text-slate-400 text-sm">{$t('common.per_month')}</span>
					</div>
					{#if isAnnual}
						<p class="text-xs text-slate-400 mt-1">
							{$t('pricing.billed_annually')}, &euro;375{$t('common.per_year')}
						</p>
					{/if}
				</div>
				<ul class="space-y-3 mb-8 flex-1">
					{#each ['pricing.starter.f1', 'pricing.starter.f2', 'pricing.starter.f3', 'pricing.starter.f4'] as key}
						<li class="flex items-center gap-2.5 text-sm text-slate-600">
							<Check class="w-4 h-4 text-slate-400 shrink-0" />
							<span>{$t(key)}</span>
						</li>
					{/each}
				</ul>
				<a
					href="/register?plan=basic"
					class="block text-center bg-slate-100 hover:bg-slate-200 text-slate-700 font-semibold text-sm py-3 rounded-lg transition-colors duration-150 cursor-pointer"
					>{$t('pricing.starter.cta')}</a
				>
			</div>

			<!-- Professional -->
			<div class="bg-white rounded-2xl p-7 flex flex-col pricing-popular relative">
				<div class="absolute -top-3 left-1/2 -translate-x-1/2">
					<span
						class="bg-coral-500 text-white text-xs font-semibold px-3 py-1 rounded-full shadow-md shadow-coral-500/20"
						>{$t('pricing.popular')}</span
					>
				</div>
				<div class="mb-6">
					<h3 class="font-heading font-bold text-slate-900 mb-1">
						{$t('pricing.pro.name')}
					</h3>
					<p class="text-sm text-slate-500">{$t('pricing.pro.desc')}</p>
				</div>
				<div class="mb-6">
					<div class="flex items-baseline gap-1">
						<span class="text-4xl font-heading font-bold text-coral-500"
							>&euro;{isAnnual ? 55 : 69}</span
						>
						<span class="text-coral-400 text-sm">{$t('common.per_month')}</span>
					</div>
					{#if isAnnual}
						<p class="text-xs text-slate-400 mt-1">
							{$t('pricing.billed_annually')}, &euro;660{$t('common.per_year')}
						</p>
					{/if}
				</div>
				<ul class="space-y-3 mb-8 flex-1">
					{#each ['pricing.pro.f1', 'pricing.pro.f2', 'pricing.pro.f3', 'pricing.pro.f4', 'pricing.pro.f5'] as key}
						<li class="flex items-center gap-2.5 text-sm text-slate-600">
							<Check class="w-4 h-4 text-coral-500 shrink-0" />
							<span>{$t(key)}</span>
						</li>
					{/each}
				</ul>
				<a
					href="/register?plan=pro"
					class="flex items-center justify-center gap-1.5 bg-coral-500 hover:bg-coral-600 text-white font-semibold text-sm py-3 rounded-lg transition-colors duration-150 cursor-pointer"
				>
					<span>{$t('pricing.pro.cta')}</span>
					<ArrowRight class="w-3.5 h-3.5" />
				</a>
			</div>

			<!-- Enterprise -->
			<div
				class="bg-white border border-slate-200 rounded-2xl p-7 flex flex-col hover:border-slate-300 transition-colors duration-150"
			>
				<div class="mb-6">
					<h3 class="font-heading font-bold text-slate-900 mb-1">
						{$t('pricing.enterprise.name')}
					</h3>
					<p class="text-sm text-slate-500">{$t('pricing.enterprise.desc')}</p>
				</div>
				<div class="mb-6">
					<div class="flex items-baseline gap-1">
						<span class="text-4xl font-heading font-bold text-slate-900"
							>{$t('common.custom')}</span
						>
					</div>
					<p class="text-xs text-slate-400 mt-1">{$t('pricing.enterprise.price_note')}</p>
				</div>
				<ul class="space-y-3 mb-8 flex-1">
					{#each ['pricing.enterprise.f1', 'pricing.enterprise.f2', 'pricing.enterprise.f3', 'pricing.enterprise.f4', 'pricing.enterprise.f5'] as key}
						<li class="flex items-center gap-2.5 text-sm text-slate-600">
							<Check class="w-4 h-4 text-slate-400 shrink-0" />
							<span>{$t(key)}</span>
						</li>
					{/each}
				</ul>
				<a
					href="/register?plan=enterprise"
					class="block text-center bg-slate-900 hover:bg-slate-800 text-white font-semibold text-sm py-3 rounded-lg transition-colors duration-150 cursor-pointer"
					>{$t('pricing.enterprise.cta')}</a
				>
			</div>
		</div>
	</div>
</section>
