<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { setAuth } from '$lib/auth';

	onMount(() => {
		const params = $page.url.searchParams;
		const token = params.get('token');
		const tenantId = params.get('tenant_id');

		if (token && tenantId) {
			setAuth(token, tenantId);
			goto('/');
		} else {
			// No credentials — redirect to portal login
			window.location.href = 'https://redcoral.app/login';
		}
	});
</script>

<div class="flex items-center justify-center min-h-screen">
	<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
		<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
		<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
	</svg>
</div>
