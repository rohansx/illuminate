<script lang="ts">
	import Nav from '$lib/components/Nav.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Problem from '$lib/components/Problem.svelte';
	import Loop from '$lib/components/Loop.svelte';
	import Features from '$lib/components/Features.svelte';
	import HowItWorks from '$lib/components/HowItWorks.svelte';
	import Docs from '$lib/components/Docs.svelte';
	import OpenSource from '$lib/components/OpenSource.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<Nav />
<main>
	<Hero />
	<Problem />
	<Loop />
	<Features />
	<HowItWorks />
	<Docs />
	<OpenSource />
</main>
<Footer />
