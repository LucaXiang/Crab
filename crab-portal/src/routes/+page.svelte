<script lang="ts">
	import { t, locale } from '$lib/i18n';
	import Navbar from '$lib/components/Navbar.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Features from '$lib/components/Features.svelte';
	import Pricing from '$lib/components/Pricing.svelte';
	import SignupForm from '$lib/components/SignupForm.svelte';
	import Faq from '$lib/components/Faq.svelte';
	import Cta from '$lib/components/Cta.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<svelte:head>
	<title>RedCoral — Sistema POS Offline-First</title>
	<meta
		name="description"
		content="RedCoral es un sistema POS distribuido para restaurantes, impulsado por edge computing y comunicación mTLS."
	/>
	<meta property="og:title" content="RedCoral — Sistema POS Offline-First" />
	<meta
		property="og:description"
		content="Sistema POS distribuido para restaurantes con edge computing y mTLS."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:url" content="https://redcoral.app" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="RedCoral — Sistema POS Offline-First" />
	<meta
		name="twitter:description"
		content="Sistema POS distribuido para restaurantes con edge computing y mTLS."
	/>
	{@html `<script type="application/ld+json">${JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'SoftwareApplication',
		name: 'RedCoral',
		applicationCategory: 'BusinessApplication',
		operatingSystem: 'Windows, macOS',
		description:
			'Distributed POS system for restaurants powered by edge computing and mTLS communication.',
		offers: {
			'@type': 'AggregateOffer',
			lowPrice: '50',
			highPrice: '79',
			priceCurrency: 'EUR',
			offerCount: '3'
		}
	})}</script>`}
</svelte:head>

<Navbar />
<Hero />
<Features />
<Pricing />
<SignupForm />
<Faq />
<Cta />
<Footer />
