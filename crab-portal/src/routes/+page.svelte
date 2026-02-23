<script lang="ts">
	import { t, locale } from '$lib/i18n';
	import Navbar from '$lib/components/Navbar.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Features from '$lib/components/Features.svelte';
	import Showcase from '$lib/components/Showcase.svelte';
	import Pricing from '$lib/components/Pricing.svelte';
	import SignupForm from '$lib/components/SignupForm.svelte';
	import Faq from '$lib/components/Faq.svelte';
	import Cta from '$lib/components/Cta.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<svelte:head>
	<title>RedCoral — TPV para hostelería con Verifactu incluido</title>
	<meta
		name="description"
		content="RedCoral es un TPV moderno para hostelería que funciona sin internet, cumple con Verifactu y registra cada operación con trazabilidad total. Desde 31 €/mes."
	/>
	<meta property="og:title" content="RedCoral — TPV para hostelería con Verifactu incluido" />
	<meta
		property="og:description"
		content="TPV moderno para hostelería: funciona sin internet, Verifactu incluido, registro antifraude. Desde 31 €/mes."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:url" content="https://redcoral.app" />
	<meta property="og:image" content="https://redcoral.app/og-image.png" />
	<meta property="og:locale" content="es_ES" />
	<link rel="canonical" href="https://redcoral.app" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="RedCoral — TPV para hostelería con Verifactu incluido" />
	<meta
		name="twitter:description"
		content="TPV moderno para hostelería: funciona sin internet, Verifactu incluido, registro antifraude. Desde 31 €/mes."
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
<Showcase />
<Pricing />
<SignupForm />
<Faq />
<Cta />
<Footer />
