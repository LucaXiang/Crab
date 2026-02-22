<script lang="ts">
	import { t } from '$lib/i18n';
	import LegalLayout from '$lib/components/LegalLayout.svelte';
</script>

<svelte:head>
	<title>{$t('terms.page_title')} — RedCoral</title>
	<meta name="description" content={$t('terms.title')} />
</svelte:head>

<LegalLayout title={$t('terms.title')} lastUpdated={$t('terms.last_updated')}>
	{@html $t('terms.content')}
</LegalLayout>
