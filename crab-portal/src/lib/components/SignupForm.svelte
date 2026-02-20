<script lang="ts">
	import { ArrowRight } from 'lucide-svelte';
	import { t } from '$lib/i18n';
</script>

<section id="signup" class="py-24 md:py-32 bg-white">
	<div class="max-w-6xl mx-auto px-6">
		<div class="max-w-md mx-auto text-center">
			<h2 class="font-heading text-2xl md:text-3xl font-bold text-slate-900 mb-2">
				{$t('signup.title')}
			</h2>
			<p class="text-sm text-slate-500 mb-8">{$t('signup.subtitle')}</p>

			<a
				href="/register"
				class="inline-flex items-center gap-2 bg-coral-500 hover:bg-coral-600 text-white font-semibold py-3 px-8 rounded-lg transition-colors duration-150 cursor-pointer"
			>
				<span>{$t('signup.cta')}</span>
				<ArrowRight class="w-4 h-4" />
			</a>

			<p class="mt-4 text-xs text-slate-400">
				{$t('signup.has_account')}
				<a href="/login" class="text-coral-500 hover:text-coral-600 font-medium"
					>{$t('signup.login_link')}</a
				>
			</p>
		</div>
	</div>
</section>
