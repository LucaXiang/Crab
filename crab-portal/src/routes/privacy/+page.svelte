<script lang="ts">
	import { t } from '$lib/i18n';
	import LegalLayout from '$lib/components/LegalLayout.svelte';
</script>

<svelte:head>
	<title>{$t('privacy.page_title')} — RedCoral</title>
	<meta name="description" content={$t('privacy.title')} />
</svelte:head>

<LegalLayout title={$t('privacy.title')} lastUpdated={$t('privacy.last_updated')}>
	{@html $t('privacy.content')}
</LegalLayout>
