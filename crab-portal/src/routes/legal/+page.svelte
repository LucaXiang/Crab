<script lang="ts">
	import { t } from '$lib/i18n';
	import LegalLayout from '$lib/components/LegalLayout.svelte';
</script>

<svelte:head>
	<title>{$t('legal.page_title')} — RedCoral</title>
	<meta name="description" content={$t('legal.title')} />
</svelte:head>

<LegalLayout title={$t('legal.title')} lastUpdated={$t('legal.last_updated')}>
	{@html $t('legal.content')}
</LegalLayout>
