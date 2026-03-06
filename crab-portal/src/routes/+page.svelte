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
	<meta property="og:locale:alternate" content="en_US" />
	<meta property="og:locale:alternate" content="zh_CN" />
	<link rel="canonical" href="https://redcoral.app" />
	<link rel="alternate" hreflang="es" href="https://redcoral.app/?lang=es" />
	<link rel="alternate" hreflang="en" href="https://redcoral.app/?lang=en" />
	<link rel="alternate" hreflang="zh" href="https://redcoral.app/?lang=zh" />
	<link rel="alternate" hreflang="x-default" href="https://redcoral.app" />
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
		url: 'https://redcoral.app',
		description:
			'TPV para hostelería que funciona sin internet. Cumplimiento Verifactu incluido, registro antifraude y sincronización multi-dispositivo.',
		offers: {
			'@type': 'AggregateOffer',
			lowPrice: '31',
			highPrice: '69',
			priceCurrency: 'EUR',
			offerCount: '3'
		}
	})}</script>`}
	{@html `<script type="application/ld+json">${JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'FAQPage',
		mainEntity: [
			{
				'@type': 'Question',
				name: '¿Funciona sin conexión a internet?',
				acceptedAnswer: { '@type': 'Answer', text: 'Sí. RedCoral está diseñado para funcionar sin internet. Todas las funciones principales — pedidos, cobros e impresión — se ejecutan localmente. Cuando vuelve la conexión, todo se sincroniza con la nube automáticamente.' }
			},
			{
				'@type': 'Question',
				name: '¿RedCoral cumple con Verifactu?',
				acceptedAnswer: { '@type': 'Answer', text: 'Sí. RedCoral incluye cumplimiento Verifactu de serie, sin módulos adicionales. Todas las facturas llevan firma digital y los registros son inalterables, cumpliendo con la normativa antifraude vigente.' }
			},
			{
				'@type': 'Question',
				name: '¿Cómo evita RedCoral el fraude interno?',
				acceptedAnswer: { '@type': 'Answer', text: 'Cada operación — ventas, anulaciones, descuentos, cambios de precio — queda registrada de forma inmutable con marca de tiempo y usuario. No se puede borrar ni modificar el historial.' }
			},
			{
				'@type': 'Question',
				name: '¿Necesito hardware especial?',
				acceptedAnswer: { '@type': 'Answer', text: 'No. RedCoral funciona en cualquier PC o portátil con Windows. También es compatible con la mayoría de impresoras térmicas de tickets.' }
			},
			{
				'@type': 'Question',
				name: '¿Puedo cambiar de plan en cualquier momento?',
				acceptedAnswer: { '@type': 'Answer', text: 'Sí. Puedes actualizar o reducir tu plan en cualquier momento. Los cargos se ajustan proporcionalmente.' }
			}
		]
	})}</script>`}
	{@html `<script type="application/ld+json">${JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'Organization',
		name: 'RedCoral',
		url: 'https://redcoral.app',
		logo: 'https://redcoral.app/favicon.svg',
		contactPoint: {
			'@type': 'ContactPoint',
			email: 'support@redcoral.app',
			contactType: 'customer support',
			availableLanguage: ['Spanish', 'English', 'Chinese']
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
