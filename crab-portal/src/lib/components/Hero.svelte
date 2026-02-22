<script lang="ts">
	import { ArrowRight, ChevronDown, Download } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<section class="hero-bg dot-grid pt-28 pb-20 md:pt-40 md:pb-32 overflow-hidden">
	<div class="relative z-10 max-w-7xl mx-auto px-6">
		<div class="grid lg:grid-cols-2 gap-12 lg:gap-8 items-center">
			<div class="max-w-3xl">
				<div
					class="inline-flex items-center gap-2 bg-white/6 border border-white/8 text-slate-400 text-xs font-medium tracking-wide uppercase px-3 py-1.5 rounded-full mb-8"
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
						class="inline-flex items-center gap-2 bg-white/8 hover:bg-white/14 text-white font-semibold px-7 py-3.5 rounded-xl border border-white/10 transition-colors duration-150 cursor-pointer"
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

			<!-- Hero Visual -->
			<div class="relative hidden lg:block perspective-1000">
				<div class="relative rounded-2xl bg-slate-900 border border-white/10 shadow-2xl shadow-coral-500/10 p-2 transform rotate-y-[-5deg] rotate-x-[5deg] hover:rotate-0 transition-transform duration-700 ease-out preserve-3d">
					<div class="absolute inset-0 bg-linear-to-tr from-coral-500/10 to-purple-500/10 rounded-2xl pointer-events-none"></div>
					<div class="relative bg-slate-950 rounded-xl overflow-hidden aspect-4/3 flex flex-col">
						<!-- Mockup Header -->
						<div class="h-10 border-b border-white/6 flex items-center px-4 gap-2 bg-slate-900/50">
							<div class="flex gap-1.5">
								<div class="w-2.5 h-2.5 rounded-full bg-red-500/50"></div>
								<div class="w-2.5 h-2.5 rounded-full bg-amber-500/50"></div>
								<div class="w-2.5 h-2.5 rounded-full bg-emerald-500/50"></div>
							</div>
							<div class="ml-4 h-4 w-32 rounded-full bg-white/4"></div>
						</div>
						<!-- Mockup Content -->
						<div class="flex-1 p-6 flex gap-6">
							<!-- Sidebar -->
							<div class="w-16 flex flex-col gap-4 border-r border-white/6 pr-4 py-2">
								<div class="w-10 h-10 rounded-xl bg-coral-500/20 mb-4 flex items-center justify-center text-coral-500">
									<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><line x1="3" x2="21" y1="9" y2="9"/><path d="m9 16 2 2 4-4"/></svg>
								</div>
								<div class="w-10 h-10 rounded-xl bg-white/4"></div>
								<div class="w-10 h-10 rounded-xl bg-white/4"></div>
								<div class="w-10 h-10 rounded-xl bg-white/4"></div>
								<div class="mt-auto w-10 h-10 rounded-full bg-white/6"></div>
							</div>
							<!-- Main Area -->
							<div class="flex-1 flex flex-col gap-6">
								<div class="flex gap-4">
									<div class="flex-1 h-28 rounded-2xl bg-linear-to-br from-white/8 to-white/2 border border-white/4 p-4 flex flex-col justify-between">
										<div class="w-8 h-8 rounded-lg bg-emerald-500/20"></div>
										<div class="space-y-2">
											<div class="w-12 h-2 rounded bg-white/10"></div>
											<div class="w-20 h-4 rounded bg-white/20"></div>
										</div>
									</div>
									<div class="flex-1 h-28 rounded-2xl bg-linear-to-br from-white/8 to-white/2 border border-white/4 p-4 flex flex-col justify-between">
										<div class="w-8 h-8 rounded-lg bg-blue-500/20"></div>
										<div class="space-y-2">
											<div class="w-12 h-2 rounded bg-white/10"></div>
											<div class="w-20 h-4 rounded bg-white/20"></div>
										</div>
									</div>
									<div class="flex-1 h-28 rounded-2xl bg-linear-to-br from-white/8 to-white/2 border border-white/4 p-4 flex flex-col justify-between">
										<div class="w-8 h-8 rounded-lg bg-purple-500/20"></div>
										<div class="space-y-2">
											<div class="w-12 h-2 rounded bg-white/10"></div>
											<div class="w-20 h-4 rounded bg-white/20"></div>
										</div>
									</div>
								</div>
								<div class="flex-1 rounded-2xl bg-white/2 border border-white/4 p-1 overflow-hidden">
									<div class="grid grid-cols-3 gap-3 p-3 h-full">
										{#each Array(6) as _}
											<div class="aspect-square rounded-xl bg-white/4 border border-white/2 hover:bg-white/8 transition-colors"></div>
										{/each}
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
				<!-- Floating Elements -->
				<div class="absolute -bottom-6 -left-8 bg-slate-900/90 backdrop-blur-md border border-white/10 p-4 rounded-2xl shadow-2xl shadow-black/50 animate-bounce duration-4000">
					<div class="flex items-center gap-4">
						<div class="w-12 h-12 rounded-xl bg-emerald-500/20 flex items-center justify-center text-emerald-400">
							<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>
						</div>
						<div>
							<div class="text-xs text-slate-400 font-medium uppercase tracking-wider mb-0.5">Total Revenue</div>
							<div class="text-xl font-bold text-white tracking-tight">$12,450.00</div>
						</div>
					</div>
				</div>
				<div class="absolute -top-4 -right-4 bg-slate-900/90 backdrop-blur-md border border-white/10 p-3 rounded-xl shadow-2xl shadow-black/50 animate-pulse duration-5000">
					<div class="flex items-center gap-2">
						<div class="w-2 h-2 rounded-full bg-emerald-400 animate-ping"></div>
						<div class="text-xs font-semibold text-emerald-400">System Online</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Stats bar -->
		<div
			class="mt-24 flex flex-wrap justify-center md:justify-start items-center gap-x-12 gap-y-8 border-t border-white/6 pt-10"
		>
			<div>
				<div class="text-3xl md:text-4xl font-heading font-bold text-white tracking-tight">{$t('hero.stat_latency_value')}</div>
				<div class="text-sm text-slate-500 mt-1 font-medium">{$t('hero.stat_latency')}</div>
			</div>
			<div class="w-px h-10 bg-white/6 hidden md:block"></div>
			<div>
				<div class="text-3xl md:text-4xl font-heading font-bold text-white tracking-tight">{$t('hero.stat_uptime_value')}</div>
				<div class="text-sm text-slate-500 mt-1 font-medium">{$t('hero.stat_uptime')}</div>
			</div>
			<div class="w-px h-10 bg-white/6 hidden md:block"></div>
			<div>
				<div class="text-3xl md:text-4xl font-heading font-bold text-white tracking-tight">{$t('hero.stat_security_value')}</div>
				<div class="text-sm text-slate-500 mt-1 font-medium">{$t('hero.stat_security')}</div>
			</div>
		</div>
	</div>
</section>
