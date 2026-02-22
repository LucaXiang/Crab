<script lang="ts">
	import { ArrowRight, ChevronDown, Download } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<section class="hero-bg dot-grid pt-28 pb-20 md:pt-40 md:pb-32 overflow-hidden">
	<div class="relative z-10 max-w-6xl mx-auto px-6">
		<div class="max-w-3xl">
			<div
				class="inline-flex items-center gap-2 bg-white/[0.06] border border-white/[0.08] text-slate-400 text-xs font-medium tracking-wide uppercase px-3 py-1.5 rounded-full mb-8"
			>
				<span class="w-1.5 h-1.5 bg-emerald-400 rounded-full glow-dot"></span>
				<span>{$t('hero.badge')}</span>
			</div>

			<h1
				class="font-heading text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-extrabold text-white leading-[1.08] tracking-tight mb-6"
			>
				{$t('hero.title_pre')}<span class="text-coral-400">{$t('hero.title_highlight')}</span
				>{$t('hero.title_post')}
			</h1>

			<p
				class="text-base md:text-lg text-slate-400 leading-relaxed mb-10 max-w-xl"
			>
				{$t('hero.subtitle')}
			</p>

			<div class="flex flex-col sm:flex-row items-start gap-3">
				<a
					href="/register"
					class="inline-flex items-center gap-2 bg-coral-500 hover:bg-coral-600 text-white font-semibold px-7 py-3.5 rounded-xl transition-colors duration-150 cursor-pointer shadow-lg shadow-coral-500/20"
				>
					<span>{$t('hero.cta_primary')}</span>
					<ArrowRight class="w-4 h-4" />
				</a>
				<a
					href="https://auth.redcoral.app/api/download/latest"
					class="inline-flex items-center gap-2 bg-white/[0.08] hover:bg-white/[0.14] text-white font-semibold px-7 py-3.5 rounded-xl border border-white/[0.1] transition-colors duration-150 cursor-pointer"
				>
					<Download class="w-4 h-4" />
					<span>{$t('hero.cta_download')}</span>
				</a>
				<a
					href="#features"
					class="inline-flex items-center gap-2 text-slate-400 hover:text-white font-medium px-4 py-3.5 transition-colors duration-150 cursor-pointer"
				>
					<span>{$t('hero.cta_secondary')}</span>
					<ChevronDown class="w-4 h-4" />
				</a>
			</div>
		</div>

		<!-- Stats bar -->
		<div
			class="mt-20 flex flex-wrap items-center gap-8 md:gap-12 border-t border-white/[0.06] pt-8"
		>
			<div>
				<div class="text-2xl md:text-3xl font-heading font-bold text-white">{$t('hero.stat_latency_value')}</div>
				<div class="text-xs text-slate-500 mt-1">{$t('hero.stat_latency')}</div>
			</div>
			<div class="w-px h-8 bg-white/[0.06] hidden md:block"></div>
			<div>
				<div class="text-2xl md:text-3xl font-heading font-bold text-white">{$t('hero.stat_uptime_value')}</div>
				<div class="text-xs text-slate-500 mt-1">{$t('hero.stat_uptime')}</div>
			</div>
			<div class="w-px h-8 bg-white/[0.06] hidden md:block"></div>
			<div>
				<div class="text-2xl md:text-3xl font-heading font-bold text-white">{$t('hero.stat_security_value')}</div>
				<div class="text-xs text-slate-500 mt-1">{$t('hero.stat_security')}</div>
			</div>
		</div>
	</div>
</section>
