<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		const umamiId = import.meta.env.VITE_UMAMI_WEBSITE_ID;
		if (umamiId) {
			const script = document.createElement('script');
			script.defer = true;
			script.src = 'https://cloud.umami.is/script.js';
			script.dataset.websiteId = umamiId;
			document.head.appendChild(script);
		}
	});
</script>

{@render children()}
