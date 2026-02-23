<script lang="ts">
	import { ArrowRight, WifiOff, FileCheck, ShieldCheck } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<section id="signup" class="py-20 md:py-28 bg-slate-900 relative overflow-hidden">
	<!-- Background effects -->
	<div class="absolute inset-0">
		<div
			class="absolute top-0 left-1/4 w-[400px] h-[400px] bg-coral-500/6 rounded-full blur-[120px]"
		></div>
		<div
			class="absolute bottom-0 right-1/4 w-[300px] h-[300px] bg-purple-500/6 rounded-full blur-[100px]"
		></div>
	</div>

	<div class="relative max-w-5xl mx-auto px-6">
		<div class="grid md:grid-cols-2 gap-12 md:gap-16 items-center">
			<!-- Left: Copy -->
			<div>
				<h2 class="font-heading text-3xl md:text-4xl font-bold text-white mb-4">
					{$t('signup.title')}
				</h2>
				<p class="text-slate-400 leading-relaxed mb-8">{$t('signup.subtitle')}</p>

				<div class="space-y-4">
					<div class="flex items-center gap-3">
						<div class="w-8 h-8 rounded-lg bg-coral-500/15 flex items-center justify-center shrink-0">
							<WifiOff class="w-4 h-4 text-coral-400" />
						</div>
						<span class="text-sm text-slate-300">{$t('features.offline.title')}</span>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-8 h-8 rounded-lg bg-coral-500/15 flex items-center justify-center shrink-0">
							<FileCheck class="w-4 h-4 text-coral-400" />
						</div>
						<span class="text-sm text-slate-300">{$t('features.verifactu.title')}</span>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-8 h-8 rounded-lg bg-coral-500/15 flex items-center justify-center shrink-0">
							<ShieldCheck class="w-4 h-4 text-coral-400" />
						</div>
						<span class="text-sm text-slate-300">{$t('features.audit.title')}</span>
					</div>
				</div>
			</div>

			<!-- Right: CTA card -->
			<div class="bg-white/5 backdrop-blur-sm border border-white/10 rounded-2xl p-8 md:p-10">
				<div class="text-center">
					<p class="text-white font-heading font-bold text-xl mb-2">{$t('common.trial_days')}</p>
					<p class="text-slate-400 text-sm mb-8">{$t('signup.subtitle')}</p>

					<a
						href="/register"
						class="w-full inline-flex items-center justify-center gap-2 bg-coral-500 hover:bg-coral-600 text-white font-semibold py-3.5 px-8 rounded-xl transition-colors duration-150 cursor-pointer shadow-lg shadow-coral-500/20"
					>
						<span>{$t('signup.cta')}</span>
						<ArrowRight class="w-4 h-4" />
					</a>

					<p class="mt-5 text-xs text-slate-500">
						{$t('signup.has_account')}
						<a href="/login" class="text-coral-400 hover:text-coral-300 font-medium"
							>{$t('signup.login_link')}</a
						>
					</p>
				</div>
			</div>
		</div>
	</div>
</section>
