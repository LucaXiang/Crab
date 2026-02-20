<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import {
		LayoutDashboard,
		Store,
		ScrollText,
		Settings,
		LogOut,
		Globe,
		ChevronDown,
		Menu,
		X
	} from 'lucide-svelte';
	import { t, locale, setLang, SUPPORTED_LANGS, LANG_LABELS, type Lang } from '$lib/i18n';
	import { clearAuth } from '$lib/auth';

	let { children, email = '' }: { children: any; email?: string } = $props();

	let mobileOpen = $state(false);
	let langOpen = $state(false);

	const navItems = [
		{ key: 'nav.dashboard', href: '/', icon: LayoutDashboard },
		{ key: 'nav.stores', href: '/stores', icon: Store, match: '/stores' },
		{ key: 'nav.audit', href: '/audit', icon: ScrollText },
		{ key: 'nav.settings', href: '/settings', icon: Settings }
	];

	function isActive(href: string, match?: string): boolean {
		const path = page.url.pathname;
		if (match) return path.startsWith(match);
		return path === href;
	}

	function handleLogout() {
		clearAuth();
		goto('/login');
	}
</script>

<div class="flex h-screen">
	<!-- Sidebar (desktop) -->
	<aside class="hidden md:flex md:w-60 flex-col bg-white border-r border-slate-200">
		<!-- Logo -->
		<div class="h-16 flex items-center px-5 border-b border-slate-100">
			<a href="/" class="flex items-center gap-2">
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
		</div>

		<!-- Nav links -->
		<nav class="flex-1 px-3 py-4 space-y-1">
			{#each navItems as item}
				<a
					href={item.href}
					class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors duration-150 {isActive(
						item.href,
						item.match
					)
						? 'bg-coral-50 text-coral-600'
						: 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
				>
					<item.icon class="w-[18px] h-[18px]" />
					<span>{$t(item.key)}</span>
				</a>
			{/each}
		</nav>

		<!-- Footer -->
		<div class="px-3 pb-4 space-y-2">
			<!-- Language -->
			<div class="relative">
				<button
					onclick={() => (langOpen = !langOpen)}
					class="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-slate-500 hover:bg-slate-50 cursor-pointer"
				>
					<Globe class="w-4 h-4" />
					<span>{LANG_LABELS[$locale]}</span>
					<ChevronDown class="w-3 h-3 ml-auto" />
				</button>
				{#if langOpen}
					<div class="absolute bottom-full left-0 mb-1 w-full bg-white border border-slate-200 rounded-lg shadow-lg py-1 z-10">
						{#each SUPPORTED_LANGS as lang}
							<button
								onclick={() => {
									setLang(lang as Lang);
									langOpen = false;
								}}
								class="block w-full text-left px-3 py-1.5 text-sm hover:bg-slate-50 cursor-pointer {$locale === lang ? 'text-coral-500 font-medium' : 'text-slate-600'}"
							>
								{LANG_LABELS[lang as Lang]}
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<!-- User + logout -->
			<div class="flex items-center gap-2 px-3 py-2">
				<div class="flex-1 min-w-0">
					<p class="text-xs text-slate-400 truncate">{email}</p>
				</div>
				<button
					onclick={handleLogout}
					class="text-slate-400 hover:text-slate-600 cursor-pointer"
					title={$t('nav.logout')}
				>
					<LogOut class="w-4 h-4" />
				</button>
			</div>
		</div>
	</aside>

	<!-- Mobile header + content -->
	<div class="flex-1 flex flex-col min-w-0">
		<!-- Mobile header -->
		<header class="md:hidden h-14 flex items-center justify-between px-4 bg-white border-b border-slate-200">
			<a href="/" class="flex items-center gap-2">
				<div class="w-7 h-7 bg-coral-500 rounded-lg flex items-center justify-center">
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
				<span class="text-base font-heading font-bold text-slate-900"
					>Red<span class="text-coral-500">Coral</span></span
				>
			</a>
			<button
				onclick={() => (mobileOpen = !mobileOpen)}
				class="text-slate-600 cursor-pointer"
			>
				{#if mobileOpen}
					<X class="w-5 h-5" />
				{:else}
					<Menu class="w-5 h-5" />
				{/if}
			</button>
		</header>

		<!-- Mobile nav overlay -->
		{#if mobileOpen}
			<div class="md:hidden absolute inset-x-0 top-14 bg-white border-b border-slate-200 shadow-lg z-50 px-4 py-3 space-y-1">
				{#each navItems as item}
					<a
						href={item.href}
						onclick={() => (mobileOpen = false)}
						class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium {isActive(
							item.href,
							item.match
						)
							? 'bg-coral-50 text-coral-600'
							: 'text-slate-600'}"
					>
						<item.icon class="w-[18px] h-[18px]" />
						<span>{$t(item.key)}</span>
					</a>
				{/each}
				<button
					onclick={handleLogout}
					class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium text-slate-600 w-full cursor-pointer"
				>
					<LogOut class="w-[18px] h-[18px]" />
					<span>{$t('nav.logout')}</span>
				</button>
			</div>
		{/if}

		<!-- Main content -->
		<main class="flex-1 overflow-y-auto">
			{@render children()}
		</main>
	</div>
</div>
