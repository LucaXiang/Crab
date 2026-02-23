<script lang="ts">
	import { ShoppingCart, CreditCard, Scissors, ClipboardList, Crown } from 'lucide-svelte';
	import { t } from '$lib/i18n';

	const showcaseItems = [
		{
			icon: ShoppingCart,
			titleKey: 'showcase.ordering.title',
			descKey: 'showcase.ordering.desc',
			image: '/screenshots/ordering.png',
			accent: 'coral'
		},
		{
			icon: CreditCard,
			titleKey: 'showcase.payment.title',
			descKey: 'showcase.payment.desc',
			image: '/screenshots/payment.png',
			accent: 'blue'
		},
		{
			icon: Scissors,
			titleKey: 'showcase.split.title',
			descKey: 'showcase.split.desc',
			image: '/screenshots/split.png',
			accent: 'purple'
		},
		{
			icon: ClipboardList,
			titleKey: 'showcase.audit.title',
			descKey: 'showcase.audit.desc',
			image: '/screenshots/audit.png',
			accent: 'amber'
		},
		{
			icon: Crown,
			titleKey: 'showcase.loyalty.title',
			descKey: 'showcase.loyalty.desc',
			image: '/screenshots/loyalty.png',
			accent: 'emerald'
		}
	];

	let activeIndex = $state(0);
	let sectionEl: HTMLElement | undefined = $state(undefined);

	function handleScroll() {
		if (!sectionEl) return;
		const rect = sectionEl.getBoundingClientRect();
		const sectionHeight = sectionEl.offsetHeight;
		const viewportHeight = window.innerHeight;

		// Calculate scroll progress within the section
		const scrolled = -rect.top;
		const scrollableHeight = sectionHeight - viewportHeight;
		if (scrollableHeight <= 0) return;

		const progress = Math.max(0, Math.min(1, scrolled / scrollableHeight));
		const newIndex = Math.min(
			showcaseItems.length - 1,
			Math.floor(progress * showcaseItems.length)
		);
		activeIndex = newIndex;
	}
</script>

<svelte:window onscroll={handleScroll} />

<section
	bind:this={sectionEl}
	class="relative bg-slate-900"
	style="height: {100 + showcaseItems.length * 80}vh;"
>
	<div class="sticky top-0 h-screen overflow-hidden">
		<!-- Background glow -->
		<div class="absolute inset-0">
			<div
				class="absolute top-1/3 right-1/4 w-[500px] h-[500px] bg-coral-500/5 rounded-full blur-[150px] transition-all duration-700"
			></div>
		</div>

		<div class="relative h-full max-w-7xl mx-auto px-6 flex items-center">
			<div class="grid lg:grid-cols-2 gap-12 lg:gap-16 items-center w-full">
				<!-- Left: Text content -->
				<div class="space-y-8">
					<div>
						<p class="text-coral-500 text-sm font-semibold tracking-wide uppercase mb-4">
							{$t('showcase.label')}
						</p>
						<h2 class="font-heading text-3xl md:text-4xl font-bold text-white mb-4">
							{$t('showcase.title')}
						</h2>
						<p class="text-slate-400 leading-relaxed">{$t('showcase.subtitle')}</p>
					</div>

					<!-- Feature list -->
					<div class="space-y-2">
						{#each showcaseItems as item, i}
							<button
								onclick={() => (activeIndex = i)}
								class="w-full text-left p-4 rounded-xl transition-all duration-300 cursor-pointer group {activeIndex === i
									? 'bg-white/8 border border-white/10'
									: 'bg-transparent border border-transparent hover:bg-white/4'}"
							>
								<div class="flex items-start gap-4">
									<div
										class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 transition-all duration-300 {activeIndex === i
											? 'bg-coral-500/20 text-coral-400'
											: 'bg-white/5 text-slate-500 group-hover:text-slate-400'}"
									>
										<item.icon class="w-5 h-5" />
									</div>
									<div class="flex-1 min-w-0">
										<h3
											class="text-sm font-semibold transition-colors duration-300 {activeIndex === i
												? 'text-white'
												: 'text-slate-400 group-hover:text-slate-300'}"
										>
											{$t(item.titleKey)}
										</h3>
										{#if activeIndex === i}
											<p class="text-sm text-slate-400 mt-1 leading-relaxed">
												{$t(item.descKey)}
											</p>
										{/if}
									</div>
									<div
										class="w-1.5 h-1.5 rounded-full mt-2 shrink-0 transition-all duration-300 {activeIndex === i
											? 'bg-coral-400'
											: 'bg-slate-700'}"
									></div>
								</div>
							</button>
						{/each}
					</div>

					<!-- Progress dots (mobile) -->
					<div class="flex gap-2 lg:hidden justify-center">
						{#each showcaseItems as _, i}
							<button
								onclick={() => (activeIndex = i)}
								aria-label="Go to slide {i + 1}"
								class="w-2 h-2 rounded-full transition-all duration-300 cursor-pointer {activeIndex === i
									? 'bg-coral-500 w-6'
									: 'bg-slate-600'}"
							></button>
						{/each}
					</div>
				</div>

				<!-- Right: Screenshot -->
				<div class="relative hidden lg:block">
					<div class="relative rounded-2xl overflow-hidden bg-slate-800 border border-white/10 shadow-2xl shadow-black/50 aspect-[4/3]">
						{#each showcaseItems as item, i}
							<div
								class="absolute inset-0 transition-all duration-500 {activeIndex === i
									? 'opacity-100 scale-100'
									: 'opacity-0 scale-95'}"
							>
								<img
									src={item.image}
									alt={$t(item.titleKey)}
									class="w-full h-full object-cover object-top"
									onerror={(e) => {
										const target = e.currentTarget as HTMLImageElement;
										target.style.display = 'none';
										const parent = target.parentElement;
										if (parent && !parent.querySelector('.placeholder')) {
											const div = document.createElement('div');
											div.className = 'placeholder absolute inset-0 flex items-center justify-center bg-slate-800/50';
											div.innerHTML = `<div class="text-center"><div class="text-slate-600 text-6xl mb-4">📸</div><p class="text-slate-500 text-sm">Screenshot coming soon</p></div>`;
											parent.appendChild(div);
										}
									}}
								/>
							</div>
						{/each}

						<!-- Window chrome overlay -->
						<div class="absolute top-0 inset-x-0 h-8 bg-slate-900/80 backdrop-blur-sm flex items-center px-3 gap-1.5 z-10">
							<div class="w-2.5 h-2.5 rounded-full bg-red-500/50"></div>
							<div class="w-2.5 h-2.5 rounded-full bg-amber-500/50"></div>
							<div class="w-2.5 h-2.5 rounded-full bg-emerald-500/50"></div>
						</div>
					</div>

					<!-- Step indicator -->
					<div class="absolute -bottom-4 left-1/2 -translate-x-1/2 bg-slate-800/90 backdrop-blur-md border border-white/10 px-4 py-2 rounded-full">
						<span class="text-xs text-slate-400">
							<span class="text-coral-400 font-semibold">{activeIndex + 1}</span>
							/ {showcaseItems.length}
						</span>
					</div>
				</div>
			</div>
		</div>
	</div>
</section>
