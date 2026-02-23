<script lang="ts">
	import { page } from '$app/state';
	import StoreLayout from '$lib/components/StoreLayout.svelte';

	let { children } = $props();

	const storeId = Number(page.params.id);
</script>

<StoreLayout {storeId}>
	{@render children()}
</StoreLayout>
