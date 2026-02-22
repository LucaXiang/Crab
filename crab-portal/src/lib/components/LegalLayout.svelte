<script lang="ts">
	import { t } from '$lib/i18n';

	let { children, title, lastUpdated } = $props<{
		children: import('svelte').Snippet;
		title: string;
		lastUpdated: string;
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

	<div class="flex-1 px-6 py-12">
		<div class="max-w-3xl mx-auto">
			<h1 class="font-heading text-2xl md:text-3xl font-bold text-slate-900 mb-2">
				{title}
			</h1>
			<p class="text-sm text-slate-500 mb-8">{lastUpdated}</p>

			<div class="bg-white rounded-2xl border border-slate-200 p-8 md:p-10 shadow-sm prose prose-slate max-w-none
				prose-headings:font-heading prose-headings:text-slate-900
				prose-h2:text-lg prose-h2:mt-8 prose-h2:mb-3
				prose-h3:text-base prose-h3:mt-6 prose-h3:mb-2
				prose-p:text-sm prose-p:leading-relaxed prose-p:text-slate-600
				prose-li:text-sm prose-li:text-slate-600
				prose-a:text-coral-500 prose-a:no-underline hover:prose-a:underline
				prose-strong:text-slate-700
				prose-table:text-sm
				prose-th:text-left prose-th:text-slate-700 prose-th:font-semibold prose-th:py-2 prose-th:px-3 prose-th:bg-slate-50
				prose-td:py-2 prose-td:px-3 prose-td:text-slate-600 prose-td:border-t prose-td:border-slate-100">
				{@render children()}
			</div>

			<div class="mt-8 text-center">
				<a
					href="/"
					class="text-sm text-slate-500 hover:text-coral-500 transition-colors duration-150"
				>
					&larr; {$t('legal.back_home')}
				</a>
			</div>
		</div>
	</div>
</div>
