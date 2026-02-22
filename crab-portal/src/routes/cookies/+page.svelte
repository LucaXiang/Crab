<script lang="ts">
	import { t } from '$lib/i18n';
	import LegalLayout from '$lib/components/LegalLayout.svelte';
</script>

<svelte:head>
	<title>{$t('cookies.page_title')} — RedCoral</title>
	<meta name="description" content={$t('cookies.title')} />
</svelte:head>

<LegalLayout title={$t('cookies.title')} lastUpdated={$t('cookies.last_updated')}>
	{@html $t('cookies.content')}
</LegalLayout>
