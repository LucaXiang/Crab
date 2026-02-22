<script lang="ts">
	import { Globe, ArrowRight, Menu, X } from 'lucide-svelte';
	import { t, locale, setLang, LANG_LABELS, SUPPORTED_LANGS, type Lang } from '$lib/i18n';

	let menuOpen = $state(false);
	let mobileMenuOpen = $state(false);

	function toggleMenu() {
		menuOpen = !menuOpen;
	}

	function toggleMobileMenu() {
		mobileMenuOpen = !mobileMenuOpen;
	}

	function selectLang(lang: Lang) {
		setLang(lang);
		menuOpen = false;
	}

	function handleClickOutside(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (!target.closest('.lang-switcher')) {
			menuOpen = false;
		}
	}
</script>

<svelte:window onclick={handleClickOutside} />

<nav class="fixed top-0 left-0 right-0 z-50 glass-nav">
	<div class="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
		<a href="/" class="flex items-center gap-2 cursor-pointer">
			<div class="w-8 h-8 bg-coral-500 rounded-lg flex items-center justify-center">
				<svg
					viewBox="0 0 24 24"
					fill="none"
					class="w-4.5 h-4.5 text-white"
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
			<span class="text-lg font-heading font-bold text-slate-900"
				>Red<span class="text-coral-500">Coral</span></span
			>
		</a>

		<div class="hidden md:flex items-center gap-8 text-[13px] font-medium text-slate-500">
			<a href="#features" class="hover:text-slate-900 transition-colors duration-150 cursor-pointer"
				>{$t('nav.features')}</a
			>
			<a href="#pricing" class="hover:text-slate-900 transition-colors duration-150 cursor-pointer"
				>{$t('nav.pricing')}</a
			>
			<a href="#faq" class="hover:text-slate-900 transition-colors duration-150 cursor-pointer"
				>{$t('nav.faq')}</a
			>
		</div>

		<div class="flex items-center gap-2">
			<div class="relative lang-switcher">
				<button
					class="flex items-center gap-1 text-[13px] font-medium text-slate-500 hover:text-slate-900 transition-colors duration-150 cursor-pointer px-2 py-1.5 rounded-lg"
					onclick={toggleMenu}
				>
					<Globe class="w-3.5 h-3.5" />
					<span>{LANG_LABELS[$locale]}</span>
				</button>
				{#if menuOpen}
					<div
						class="absolute right-0 top-full mt-1 bg-white rounded-lg shadow-lg shadow-slate-200/50 border border-slate-100 py-1 min-w-20 z-50"
					>
						{#each SUPPORTED_LANGS as lang}
							<button
								onclick={() => selectLang(lang)}
								class="block w-full text-left px-3 py-1.5 text-[13px] text-slate-600 hover:bg-slate-50 hover:text-slate-900 cursor-pointer"
							>
								{LANG_LABELS[lang]}
							</button>
						{/each}
					</div>
				{/if}
			</div>
			<div class="hidden md:flex items-center gap-4">
				<a
					href="/login"
					class="px-4 py-2 text-sm font-medium text-slate-500 hover:text-slate-900 transition-colors"
				>
					{$t('nav.login')}
				</a>
				<a
					href="/register"
					class="px-4 py-2 text-sm font-medium text-white bg-coral-500 hover:bg-coral-600 rounded-lg transition-colors shadow-lg shadow-coral-500/20 min-w-20 text-center"
				>
					{$t('nav.signup')}
				</a>
			</div>

			<!-- Mobile menu button -->
			<button
				class="md:hidden p-2 text-slate-500 hover:text-slate-900 transition-colors"
				onclick={toggleMobileMenu}
				aria-label="Toggle menu"
			>
				{#if mobileMenuOpen}
					<X class="w-6 h-6" />
				{:else}
					<Menu class="w-6 h-6" />
				{/if}
			</button>
		</div>
	</div>

	<!-- Mobile menu -->
	{#if mobileMenuOpen}
		<!-- Backdrop -->
		<div 
			class="md:hidden fixed inset-0 bg-slate-900/20 backdrop-blur-sm z-40"
			onclick={() => (mobileMenuOpen = false)}
			role="button"
			tabindex="0"
			onkeydown={(e) => e.key === 'Escape' && (mobileMenuOpen = false)}
		></div>

		<!-- Menu Panel -->
		<div class="md:hidden fixed top-16 left-0 right-0 bg-white border-b border-slate-100 shadow-xl z-50 animate-in slide-in-from-top-2 duration-200">
			<div class="flex flex-col p-4 gap-2 max-h-[calc(100dvh-4rem)] overflow-y-auto">
				<a
					href="#features"
					class="p-4 text-slate-600 hover:bg-slate-50 hover:text-slate-900 rounded-xl transition-colors font-medium flex items-center justify-between group"
					onclick={() => (mobileMenuOpen = false)}
				>
					<span>{$t('nav.features')}</span>
					<ArrowRight class="w-4 h-4 opacity-0 group-hover:opacity-100 transition-opacity text-slate-400" />
				</a>
				<a
					href="#pricing"
					class="p-4 text-slate-600 hover:bg-slate-50 hover:text-slate-900 rounded-xl transition-colors font-medium flex items-center justify-between group"
					onclick={() => (mobileMenuOpen = false)}
				>
					<span>{$t('nav.pricing')}</span>
					<ArrowRight class="w-4 h-4 opacity-0 group-hover:opacity-100 transition-opacity text-slate-400" />
				</a>
				<a
					href="#faq"
					class="p-4 text-slate-600 hover:bg-slate-50 hover:text-slate-900 rounded-xl transition-colors font-medium flex items-center justify-between group"
					onclick={() => (mobileMenuOpen = false)}
				>
					<span>{$t('nav.faq')}</span>
					<ArrowRight class="w-4 h-4 opacity-0 group-hover:opacity-100 transition-opacity text-slate-400" />
				</a>
				
				<div class="h-px bg-slate-100 my-2"></div>
				
				<a
					href="/login"
					class="p-4 text-slate-600 hover:bg-slate-50 hover:text-slate-900 rounded-xl transition-colors font-medium flex items-center gap-3"
					onclick={() => (mobileMenuOpen = false)}
				>
					<span>{$t('nav.login')}</span>
				</a>
				<a
					href="/register"
					class="p-4 bg-slate-900 text-white hover:bg-slate-800 rounded-xl transition-colors font-semibold flex items-center justify-center gap-2 shadow-lg shadow-slate-900/10"
					onclick={() => (mobileMenuOpen = false)}
				>
					<span>{$t('nav.cta')}</span>
					<ArrowRight class="w-4 h-4" />
				</a>
			</div>
		</div>
	{/if}
</nav>
