<script lang="ts">
	import { Globe, ArrowRight } from 'lucide-svelte';
	import { t, locale, setLang, LANG_LABELS, SUPPORTED_LANGS, type Lang } from '$lib/i18n';

	let menuOpen = $state(false);

	function toggleMenu() {
		menuOpen = !menuOpen;
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
					class="w-[18px] h-[18px] text-white"
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
					class="flex items-center gap-1 text-[13px] font-medium text-slate-400 hover:text-slate-600 transition-colors duration-150 cursor-pointer px-2 py-1.5 rounded-lg"
					onclick={toggleMenu}
				>
					<Globe class="w-3.5 h-3.5" />
					<span>{LANG_LABELS[$locale]}</span>
				</button>
				{#if menuOpen}
					<div
						class="absolute right-0 top-full mt-1 bg-white rounded-lg shadow-lg shadow-slate-200/50 border border-slate-100 py-1 min-w-[80px] z-50"
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
			<a
				href="/login"
				class="hidden sm:inline-flex text-[13px] font-medium text-slate-500 hover:text-slate-900 transition-colors duration-150 cursor-pointer px-3 py-1.5"
				>{$t('nav.login')}</a
			>
			<a
				href="/register"
				class="inline-flex items-center gap-1.5 bg-slate-900 hover:bg-slate-800 text-white text-[13px] font-semibold px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer"
			>
				<span>{$t('nav.cta')}</span>
				<ArrowRight class="w-3.5 h-3.5" />
			</a>
		</div>
	</div>
</nav>
