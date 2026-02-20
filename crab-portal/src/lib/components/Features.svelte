<script lang="ts">
	import {
		WifiOff,
		ShieldCheck,
		Zap,
		MonitorSmartphone,
		Printer,
		Cloud
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';

	const features = [
		{ icon: WifiOff, titleKey: 'features.offline.title', descKey: 'features.offline.desc' },
		{ icon: ShieldCheck, titleKey: 'features.security.title', descKey: 'features.security.desc' },
		{ icon: Zap, titleKey: 'features.speed.title', descKey: 'features.speed.desc' },
		{
			icon: MonitorSmartphone,
			titleKey: 'features.multidevice.title',
			descKey: 'features.multidevice.desc'
		},
		{ icon: Printer, titleKey: 'features.printer.title', descKey: 'features.printer.desc' },
		{ icon: Cloud, titleKey: 'features.cloud.title', descKey: 'features.cloud.desc' }
	];
</script>

<section id="features" class="py-24 md:py-32 bg-white">
	<div class="max-w-6xl mx-auto px-6">
		<div class="max-w-2xl mb-16">
			<p class="text-coral-500 text-sm font-semibold tracking-wide uppercase mb-3">
				{$t('nav.features')}
			</p>
			<h2 class="font-heading text-3xl md:text-4xl font-bold text-slate-900 mb-4">
				{$t('features.title')}
			</h2>
			<p class="text-slate-500 leading-relaxed">{$t('features.subtitle')}</p>
		</div>

		<div class="grid md:grid-cols-2 lg:grid-cols-3 gap-px bg-slate-100 rounded-2xl overflow-hidden border border-slate-100">
			{#each features as feature}
				<div class="bg-white p-8 group cursor-default hover:bg-slate-50/80 transition-colors duration-150">
					<div
						class="w-10 h-10 rounded-lg bg-coral-50 flex items-center justify-center mb-5 group-hover:bg-coral-100 transition-colors duration-150"
					>
						<feature.icon class="w-5 h-5 text-coral-500" />
					</div>
					<h3 class="text-[15px] font-heading font-bold text-slate-900 mb-2">
						{$t(feature.titleKey)}
					</h3>
					<p class="text-sm text-slate-500 leading-relaxed">{$t(feature.descKey)}</p>
				</div>
			{/each}
		</div>
	</div>
</section>
