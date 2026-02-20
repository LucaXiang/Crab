<script lang="ts">
	import { t } from '$lib/i18n';

	let { children, title, subtitle = '' } = $props<{
		children: import('svelte').Snippet;
		title: string;
		subtitle?: string;
	}>();
</script>

<div class="min-h-screen bg-slate-50 flex flex-col">
	<nav class="h-16 flex items-center px-6 bg-white border-b border-slate-100">
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
	</nav>

	<div class="flex-1 flex items-center justify-center px-6 py-12">
		<div class="w-full max-w-md">
			<div class="text-center mb-8">
				<h1 class="font-heading text-2xl md:text-3xl font-bold text-slate-900 mb-2">
					{title}
				</h1>
				{#if subtitle}
					<p class="text-sm text-slate-500">{subtitle}</p>
				{/if}
			</div>
			<div class="bg-white rounded-2xl border border-slate-200 p-8 shadow-sm">
				{@render children()}
			</div>
		</div>
	</div>
</div>
