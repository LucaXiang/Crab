<script lang="ts">
	import { browser } from '$app/environment';

	if (browser) {
		window.location.href = 'https://console.redcoral.app/forgot-password';
	}
</script>

<svelte:head>
	<meta http-equiv="refresh" content="0;url=https://console.redcoral.app/forgot-password" />
</svelte:head>

<p class="p-8 text-center text-slate-500">Redirigiendo a console...</p>
