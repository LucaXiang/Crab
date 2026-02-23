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
		ChevronLeft,
		Menu,
		X,
		Radio,
		ShoppingBag,
		BarChart3,
		Package,
		FolderTree,
		Tag,
		SlidersHorizontal,
		Percent,
		Users,
		Map,
		Grid3x3,
		ShieldAlert
	} from 'lucide-svelte';
	import { t, locale, setLang, SUPPORTED_LANGS, LANG_LABELS, type Lang } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import { getStores, ApiError, type StoreDetail } from '$lib/api';
	import { onMount } from 'svelte';

	let { children, storeId }: { children: any; storeId: number } = $props();

	let mobileOpen = $state(false);
	let langOpen = $state(false);
	let storeName = $state('');
	let storeOnline = $state(false);

	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }
		try {
			const stores = await getStores(token);
			const store = stores.find((s) => s.id === storeId);
			if (store) {
				storeName = store.name ?? store.store_info?.name as string ?? `Store #${storeId}`;
				storeOnline = store.is_online;
			}
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); }
		}
	});

	const mainNav = [
		{ key: 'nav.dashboard', href: '/', icon: LayoutDashboard },
		{ key: 'nav.stores', href: '/stores', icon: Store, match: '/stores' },
		{ key: 'nav.audit', href: '/audit', icon: ScrollText },
		{ key: 'nav.settings', href: '/settings', icon: Settings }
	];

	type NavItem = { key: string; href: string; icon: any };
	type NavGroup = { label: string; items: NavItem[] };

	const storeNav: NavGroup[] = [
		{
			label: 'nav.group_operations',
			items: [
				{ key: 'nav.overview', href: `/stores/${storeId}/overview`, icon: BarChart3 },
				{ key: 'nav.live_orders', href: `/stores/${storeId}/live`, icon: Radio },
				{ key: 'nav.orders', href: `/stores/${storeId}/orders`, icon: ShoppingBag },
				{ key: 'nav.daily_report', href: `/stores/${storeId}/stats`, icon: ScrollText }
			]
		},
		{
			label: 'nav.group_catalog',
			items: [
				{ key: 'nav.products', href: `/stores/${storeId}/products`, icon: Package },
				{ key: 'nav.categories', href: `/stores/${storeId}/categories`, icon: FolderTree },
				{ key: 'nav.tags', href: `/stores/${storeId}/tags`, icon: Tag },
				{ key: 'nav.attributes', href: `/stores/${storeId}/attributes`, icon: SlidersHorizontal },
				{ key: 'nav.price_rules', href: `/stores/${storeId}/price-rules`, icon: Percent }
			]
		},
		{
			label: 'nav.group_manage',
			items: [
				{ key: 'nav.employees', href: `/stores/${storeId}/employees`, icon: Users },
				{ key: 'nav.zones', href: `/stores/${storeId}/zones`, icon: Map },
				{ key: 'nav.tables', href: `/stores/${storeId}/tables`, icon: Grid3x3 }
			]
		},
		{
			label: 'nav.group_monitor',
			items: [
				{ key: 'nav.red_flags', href: `/stores/${storeId}/red-flags`, icon: ShieldAlert }
			]
		}
	];

	// Flat list for mobile tabs
	const mobileTabItems: NavItem[] = storeNav.flatMap((g) => g.items);

	function isActive(href: string): boolean {
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}

	function isMainActive(href: string, match?: string): boolean {
		const path = page.url.pathname;
		if (match) return path.startsWith(match);
		return path === href;
	}

	function handleLogout() {
		clearAuth();
		goto('/login');
	}
</script>

<div class="flex h-dvh overflow-hidden bg-slate-50">
	<!-- === Main sidebar (desktop) === -->
	<aside class="hidden md:flex md:w-56 flex-col bg-white border-r border-slate-200 shrink-0">
		<div class="h-14 flex items-center px-4 border-b border-slate-100">
			<a href="/" class="flex items-center gap-2">
				<div class="w-7 h-7 bg-coral-500 rounded-lg flex items-center justify-center">
					<svg viewBox="0 0 24 24" fill="none" class="w-4 h-4 text-white" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z" />
						<line x1="6" y1="17" x2="18" y2="17" />
					</svg>
				</div>
				<span class="text-base font-heading font-bold text-slate-900">Red<span class="text-coral-500">Coral</span></span>
			</a>
		</div>

		<nav class="flex-1 px-3 py-3 space-y-0.5">
			{#each mainNav as item}
				<a
					href={item.href}
					class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors {isMainActive(item.href, item.match)
						? 'bg-coral-50 text-coral-600'
						: 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
				>
					<item.icon class="w-4 h-4" />
					<span>{$t(item.key)}</span>
				</a>
			{/each}
		</nav>

		<div class="px-3 pb-3 space-y-1">
			<div class="relative">
				<button
					onclick={() => (langOpen = !langOpen)}
					class="flex items-center gap-2 w-full px-3 py-1.5 rounded-lg text-xs text-slate-400 hover:bg-slate-50 cursor-pointer"
				>
					<Globe class="w-3.5 h-3.5" />
					<span>{LANG_LABELS[$locale]}</span>
					<ChevronDown class="w-3 h-3 ml-auto" />
				</button>
				{#if langOpen}
					<div class="absolute bottom-full left-0 mb-1 w-full bg-white border border-slate-200 rounded-lg shadow-lg py-1 z-10">
						{#each SUPPORTED_LANGS as lang}
							<button
								onclick={() => { setLang(lang as Lang); langOpen = false; }}
								class="block w-full text-left px-3 py-1.5 text-sm hover:bg-slate-50 cursor-pointer {$locale === lang ? 'text-coral-500 font-medium' : 'text-slate-600'}"
							>
								{LANG_LABELS[lang as Lang]}
							</button>
						{/each}
					</div>
				{/if}
			</div>
			<button
				onclick={handleLogout}
				class="flex items-center gap-2 w-full px-3 py-1.5 rounded-lg text-xs text-slate-400 hover:bg-slate-50 hover:text-slate-600 cursor-pointer"
			>
				<LogOut class="w-3.5 h-3.5" />
				<span>{$t('nav.logout')}</span>
			</button>
			<p class="px-3 text-[10px] text-slate-300">v{__APP_VERSION__} ({__GIT_HASH__})</p>
		</div>
	</aside>

	<!-- === Store secondary sidebar (desktop) === -->
	<aside class="hidden md:flex md:w-48 flex-col bg-white border-r border-slate-100 shrink-0">
		<!-- Store header -->
		<div class="h-14 flex items-center gap-2 px-4 border-b border-slate-100">
			<a href="/" class="text-slate-400 hover:text-slate-600 shrink-0" title={$t('store.back')}>
				<ChevronLeft class="w-4 h-4" />
			</a>
			<div class="min-w-0 flex-1">
				<p class="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</p>
				{#if storeOnline}
					<p class="text-[10px] text-green-500 flex items-center gap-1">
						<span class="w-1.5 h-1.5 bg-green-400 rounded-full inline-block"></span>
						Online
					</p>
				{/if}
			</div>
		</div>

		<!-- Store navigation -->
		<nav class="flex-1 overflow-y-auto px-2 py-3 space-y-4">
			{#each storeNav as group}
				<div>
					<p class="px-2.5 mb-1 text-[10px] font-semibold text-slate-400 uppercase tracking-wider">{$t(group.label)}</p>
					<div class="space-y-0.5">
						{#each group.items as item}
							<a
								href={item.href}
								class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] font-medium transition-colors {isActive(item.href)
									? 'bg-coral-50 text-coral-600'
									: 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
							>
								<item.icon class="w-3.5 h-3.5" />
								<span>{$t(item.key)}</span>
							</a>
						{/each}
					</div>
				</div>
			{/each}

			<!-- Store settings link -->
			<div>
				<a
					href="/stores/{storeId}"
					class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] font-medium transition-colors {page.url.pathname === `/stores/${storeId}`
						? 'bg-coral-50 text-coral-600'
						: 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
				>
					<Settings class="w-3.5 h-3.5" />
					<span>{$t('nav.store_settings')}</span>
				</a>
			</div>
		</nav>
	</aside>

	<!-- === Mobile + Content === -->
	<div class="flex-1 flex flex-col min-w-0 relative">
		<!-- Mobile header -->
		<header class="md:hidden h-12 flex items-center justify-between px-3 bg-white border-b border-slate-200 shrink-0 z-50">
			<div class="flex items-center gap-2 min-w-0">
				<a href="/" class="text-slate-400 hover:text-slate-600 shrink-0">
					<ChevronLeft class="w-5 h-5" />
				</a>
				<span class="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</span>
				{#if storeOnline}
					<span class="w-1.5 h-1.5 bg-green-400 rounded-full shrink-0"></span>
				{/if}
			</div>
			<button
				onclick={() => (mobileOpen = !mobileOpen)}
				class="text-slate-600 cursor-pointer p-1"
			>
				{#if mobileOpen}
					<X class="w-5 h-5" />
				{:else}
					<Menu class="w-5 h-5" />
				{/if}
			</button>
		</header>

		<!-- Mobile navigation tabs (horizontal scroll) -->
		<nav class="md:hidden flex overflow-x-auto bg-white border-b border-slate-100 px-2 gap-1 no-scrollbar shrink-0">
			{#each mobileTabItems as item}
				<a
					href={item.href}
					class="flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium whitespace-nowrap border-b-2 transition-colors {isActive(item.href)
						? 'border-coral-500 text-coral-600'
						: 'border-transparent text-slate-500 hover:text-slate-700'}"
				>
					<item.icon class="w-3.5 h-3.5" />
					<span>{$t(item.key)}</span>
				</a>
			{/each}
			<a
				href="/stores/{storeId}"
				class="flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium whitespace-nowrap border-b-2 transition-colors {page.url.pathname === `/stores/${storeId}`
					? 'border-coral-500 text-coral-600'
					: 'border-transparent text-slate-500 hover:text-slate-700'}"
			>
				<Settings class="w-3.5 h-3.5" />
				<span>{$t('nav.store_settings')}</span>
			</a>
		</nav>

		<!-- Mobile nav overlay (hamburger menu for main nav) -->
		{#if mobileOpen}
			<div
				class="md:hidden fixed inset-0 bg-slate-900/20 backdrop-blur-sm z-40"
				onclick={() => (mobileOpen = false)}
				role="button"
				tabindex="0"
				onkeydown={(e) => e.key === 'Escape' && (mobileOpen = false)}
			></div>
			<div class="md:hidden absolute inset-x-0 top-12 bg-white border-b border-slate-200 shadow-xl z-50 px-3 py-2 space-y-0.5">
				{#each mainNav as item}
					<a
						href={item.href}
						onclick={() => (mobileOpen = false)}
						class="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium {isMainActive(item.href, item.match)
							? 'bg-coral-50 text-coral-600'
							: 'text-slate-600'}"
					>
						<item.icon class="w-5 h-5" />
						<span>{$t(item.key)}</span>
					</a>
				{/each}
				<div class="border-t border-slate-100 my-1 pt-1">
					<button
						onclick={handleLogout}
						class="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium text-slate-600 w-full cursor-pointer hover:bg-slate-50"
					>
						<LogOut class="w-5 h-5" />
						<span>{$t('nav.logout')}</span>
					</button>
				</div>
			</div>
		{/if}

		<!-- Content -->
		<main class="flex-1 overflow-y-auto">
			{@render children()}
		</main>
	</div>
</div>
