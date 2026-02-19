<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import Nav from '$lib/components/Nav.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Problem from '$lib/components/Problem.svelte';
	import Loop from '$lib/components/Loop.svelte';
	import Features from '$lib/components/Features.svelte';
	import HowItWorks from '$lib/components/HowItWorks.svelte';
	import Docs from '$lib/components/Docs.svelte';
	import OpenSource from '$lib/components/OpenSource.svelte';
	import Footer from '$lib/components/Footer.svelte';

	let isLoggedIn = $state(false);

	onMount(async () => {
		const user = await api.checkAuth();
		if (user) isLoggedIn = true;
	});
</script>

<Nav {isLoggedIn} />
<main>
	<Hero {isLoggedIn} />
	<Problem />
	<Loop />
	<Features />
	<HowItWorks />
	<Docs />
	<OpenSource />
</main>
<Footer />
