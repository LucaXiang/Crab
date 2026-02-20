<script lang="ts">
	import '../app.css';
	import { initI18n } from '$lib/i18n';
	import { browser } from '$app/environment';

	if (browser) initI18n();

	let { children } = $props();
</script>

{@render children()}
