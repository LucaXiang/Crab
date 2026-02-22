<script lang="ts">
	import { t } from '$lib/i18n';
</script>

<footer class="bg-slate-950 text-white py-14">
	<div class="max-w-6xl mx-auto px-6">
		<div class="grid md:grid-cols-4 gap-10 mb-10">
			<div>
				<div class="flex items-center gap-2 mb-3">
					<div class="w-7 h-7 bg-coral-500 rounded-md flex items-center justify-center">
						<svg
							viewBox="0 0 24 24"
							fill="none"
							class="w-4 h-4 text-white"
							stroke="currentColor"
							stroke-width="2.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z"
							/>
							<line x1="6" y1="17" x2="18" y2="17" />
						</svg>
					</div>
					<span class="text-sm font-heading font-bold"
						>Red<span class="text-coral-400">Coral</span></span
					>
				</div>
				<p class="text-slate-500 text-xs leading-relaxed">{$t('footer.tagline')}</p>
			</div>
			<div>
				<h4 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-4">
					{$t('footer.product')}
				</h4>
				<ul class="space-y-2">
					<li>
						<a
							href="#features"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.features')}</a
						>
					</li>
					<li>
						<a
							href="#pricing"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.pricing')}</a
						>
					</li>
					</ul>
			</div>
			<div>
				<h4 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-4">
					{$t('footer.support')}
				</h4>
				<ul class="space-y-2">
					<li>
						<a
							href="#faq"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.faq')}</a
						>
					</li>
					<li>
						<a
							href="/support"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.docs')}</a
						>
					</li>
					<li>
						<a
							href="/support"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.contact')}</a
						>
					</li>
				</ul>
			</div>
			<div>
				<h4 class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-4">
					{$t('footer.legal')}
				</h4>
				<ul class="space-y-2">
					<li>
						<a
							href="/terms"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.terms')}</a
						>
					</li>
					<li>
						<a
							href="/privacy"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.privacy')}</a
						>
					</li>
					<li>
						<a
							href="/cookies"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.cookies')}</a
						>
					</li>
					<li>
						<a
							href="/legal"
							class="text-sm text-slate-500 hover:text-white transition-colors duration-150 cursor-pointer"
							>{$t('footer.aviso_legal')}</a
						>
					</li>
				</ul>
			</div>
		</div>
		<div class="border-t border-slate-800/60 pt-6 text-center">
			<p class="text-xs text-slate-600">{$t('footer.copyright')} · v{__BUILD_VERSION__}</p>
		</div>
	</div>
</footer>
