<script lang="ts">
	import { t } from '$lib/i18n';
	import { Twitter, Github, Linkedin, Facebook } from 'lucide-svelte';
</script>

<footer class="bg-slate-950 text-white py-14 border-t border-slate-900">
	<div class="max-w-6xl mx-auto px-6">
		<div class="grid md:grid-cols-4 gap-12 mb-16">
			<div class="col-span-1 md:col-span-1">
				<div class="flex items-center gap-2 mb-6">
					<div class="w-8 h-8 bg-coral-500 rounded-lg flex items-center justify-center shadow-lg shadow-coral-500/20">
						<svg
							viewBox="0 0 24 24"
							fill="none"
							class="w-5 h-5 text-white"
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
					<span class="text-lg font-heading font-bold"
						>Red<span class="text-coral-500">Coral</span></span
					>
				</div>
				<p class="text-slate-400 text-sm leading-relaxed mb-6">{$t('footer.tagline')}</p>
				<div class="flex gap-4">
					<a href="https://twitter.com" class="text-slate-500 hover:text-white transition-colors duration-200" aria-label="Twitter">
						<Twitter class="w-5 h-5" />
					</a>
					<a href="https://github.com" class="text-slate-500 hover:text-white transition-colors duration-200" aria-label="GitHub">
						<Github class="w-5 h-5" />
					</a>
					<a href="https://linkedin.com" class="text-slate-500 hover:text-white transition-colors duration-200" aria-label="LinkedIn">
						<Linkedin class="w-5 h-5" />
					</a>
				</div>
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
		<div class="border-t border-slate-800/50 pt-8 flex flex-col md:flex-row items-center justify-between gap-4">
			<div class="flex flex-col md:flex-row items-center gap-4">
				<p class="text-slate-500 text-sm">
					&copy; {new Date().getFullYear()} RedCoral. All rights reserved.
				</p>
			</div>
			<div class="flex items-center gap-6">
				<span class="flex items-center gap-2 text-sm text-slate-500">
					<span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
					All systems operational
				</span>
			</div>
		</div>
	</div>
</footer>
