<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { initI18n } from '$lib/i18n';

	let { children } = $props();

	onMount(() => {
		initI18n();
	});
</script>

{@render children()}
