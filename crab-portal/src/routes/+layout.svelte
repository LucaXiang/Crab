<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { initI18n } from '$lib/i18n';
	import CookieBanner from '$lib/components/CookieBanner.svelte';

	let { children } = $props();

	onMount(() => {
		initI18n();
	});
</script>

{@render children()}
<CookieBanner />
