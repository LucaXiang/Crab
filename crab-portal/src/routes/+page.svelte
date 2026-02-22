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
	<title>RedCoral — POS para restaurantes que funciona sin internet</title>
	<meta
		name="description"
		content="RedCoral es un sistema POS para restaurantes que funciona sin internet. Seguro, rápido y fácil de usar. Gestiona pedidos, cobros e impresión desde cualquier dispositivo."
	/>
	<meta property="og:title" content="RedCoral — POS para restaurantes que funciona sin internet" />
	<meta
		property="og:description"
		content="Sistema POS para restaurantes que funciona sin internet. Seguro, rápido y fácil de usar."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:url" content="https://redcoral.app" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="RedCoral — POS para restaurantes que funciona sin internet" />
	<meta
		name="twitter:description"
		content="Sistema POS para restaurantes que funciona sin internet. Seguro, rápido y fácil de usar."
	/>
	{@html `<script type="application/ld+json">${JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'SoftwareApplication',
		name: 'RedCoral',
		applicationCategory: 'BusinessApplication',
		operatingSystem: 'Windows',
		description:
			'POS system for restaurants that works offline. Secure, fast, and easy to use.',
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
