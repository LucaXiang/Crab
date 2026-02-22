<script lang="ts">
	import { t } from '$lib/i18n';
	import { browser } from '$app/environment';

	let visible = $state(false);

	$effect(() => {
		if (browser && !localStorage.getItem('cookie_consent')) {
			visible = true;
		}
	});

	function accept(value: 'all' | 'necessary') {
		localStorage.setItem('cookie_consent', value);
		visible = false;
	}
</script>

{#if visible}
	<div class="fixed bottom-0 inset-x-0 z-50 p-4 md:p-6">
		<div
			class="max-w-3xl mx-auto bg-slate-900 text-white rounded-xl shadow-2xl border border-slate-700/50 p-5 flex flex-col sm:flex-row items-start sm:items-center gap-4"
		>
			<p class="text-sm text-slate-300 flex-1">
				{$t('cookie_banner.message')}
				<a
					href="/cookies"
					class="text-coral-400 hover:underline ml-1"
				>
					{$t('cookie_banner.link')}
				</a>
			</p>
			<div class="flex gap-3 shrink-0">
				<button
					onclick={() => accept('necessary')}
					class="text-sm px-4 py-2 rounded-lg border border-slate-600 text-slate-300 hover:bg-slate-800 transition-colors cursor-pointer"
				>
					{$t('cookie_banner.reject')}
				</button>
				<button
					onclick={() => accept('all')}
					class="text-sm px-4 py-2 rounded-lg bg-coral-500 text-white hover:bg-coral-600 transition-colors cursor-pointer"
				>
					{$t('cookie_banner.accept')}
				</button>
			</div>
		</div>
	</div>
{/if}
