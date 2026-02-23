<script lang="ts">
	import {
		WifiOff,
		FileCheck,
		Zap,
		ShieldCheck,
		MonitorSmartphone,
		Cloud
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';

	const features = [
		{ icon: WifiOff, titleKey: 'features.offline.title', descKey: 'features.offline.desc' },
		{ icon: FileCheck, titleKey: 'features.verifactu.title', descKey: 'features.verifactu.desc' },
		{ icon: Zap, titleKey: 'features.speed.title', descKey: 'features.speed.desc' },
		{
			icon: ShieldCheck,
			titleKey: 'features.audit.title',
			descKey: 'features.audit.desc'
		},
		{ icon: MonitorSmartphone, titleKey: 'features.multidevice.title', descKey: 'features.multidevice.desc' },
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

		<div class="grid md:grid-cols-2 lg:grid-cols-3 gap-px bg-slate-100 rounded-2xl overflow-hidden border border-slate-100 shadow-sm">
			{#each features as feature}
				<div class="bg-white p-8 group cursor-default hover:bg-slate-50 transition-all duration-300 hover:shadow-lg relative z-0 hover:z-10">
					<div
						class="w-12 h-12 rounded-xl bg-coral-50 flex items-center justify-center mb-6 group-hover:bg-coral-500 group-hover:text-white group-hover:scale-110 transition-all duration-300"
					>
						<feature.icon class="w-6 h-6 text-coral-500 group-hover:text-white transition-colors duration-300" />
					</div>
					<h3 class="text-lg font-heading font-bold text-slate-900 mb-3 group-hover:text-coral-600 transition-colors duration-300">
						{$t(feature.titleKey)}
					</h3>
					<p class="text-sm text-slate-500 leading-relaxed group-hover:text-slate-600 transition-colors duration-300">{$t(feature.descKey)}</p>
				</div>
			{/each}
		</div>
	</div>
</section>
