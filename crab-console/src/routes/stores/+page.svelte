<script lang="ts">
	import { goto } from '$app/navigation';
	// Stores listing redirects to dashboard — stores are shown there
	import { onMount } from 'svelte';
	onMount(() => goto('/'));
</script>
