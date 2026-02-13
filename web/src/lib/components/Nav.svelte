<script lang="ts">
	let scrolled = $state(false);
	let menuOpen = $state(false);

	function handleScroll() {
		scrolled = window.scrollY > 20;
	}

	$effect(() => {
		window.addEventListener('scroll', handleScroll, { passive: true });
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<nav class:scrolled>
	<div class="nav-inner">
		<a href="/" class="logo">
			<span class="logo-name">illuminate</span><span class="logo-dot">.sh</span>
		</a>

		<button class="menu-toggle" onclick={() => menuOpen = !menuOpen} aria-label="Toggle menu">
			{#if menuOpen}✕{:else}≡{/if}
		</button>

		<div class="nav-links" class:open={menuOpen}>
			<a href="#features" onclick={() => menuOpen = false}>Features</a>
			<a href="/awesome" onclick={() => menuOpen = false}>Awesome</a>
			<a href="#docs" onclick={() => menuOpen = false}>Docs</a>
			<a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener" class="github-icon" aria-label="GitHub">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
			</a>
			<a href="/login" class="nav-cta" onclick={() => menuOpen = false}>Get Started</a>
		</div>
	</div>
</nav>

<style>
	nav {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 1000;
		padding: 0 1.5rem;
		transition: background 0.3s ease, border-color 0.3s ease;
		border-bottom: 1px solid transparent;
	}

	nav.scrolled {
		background: rgba(9, 9, 11, 0.9);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border-bottom-color: var(--border);
	}

	.nav-inner {
		max-width: var(--content-width);
		margin: 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 64px;
	}

	.logo {
		font-size: 1.05rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--text-bright);
		transition: none;
	}

	.logo:hover {
		color: var(--text-bright);
	}

	.logo-name {
		color: var(--amber);
	}

	.logo-dot {
		color: var(--text-dim);
		font-weight: 400;
	}

	.nav-links {
		display: flex;
		align-items: center;
		gap: 2rem;
	}

	.nav-links a {
		font-size: 0.8rem;
		color: var(--text-muted);
		transition: color 0.2s ease;
		font-weight: 500;
	}

	.nav-links a:hover {
		color: var(--text-bright);
	}

	.github-icon {
		display: flex;
		align-items: center;
	}

	.nav-cta {
		padding: 0.45rem 1.1rem;
		background: var(--amber);
		color: var(--bg) !important;
		border-radius: 4px;
		font-weight: 600;
		font-size: 0.75rem !important;
		letter-spacing: 0.02em;
		transition: background 0.2s ease, transform 0.15s ease !important;
	}

	.nav-cta:hover {
		background: var(--amber-bright) !important;
		color: var(--bg) !important;
		transform: translateY(-1px);
	}

	.menu-toggle {
		display: none;
		background: none;
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 1.25rem;
		cursor: pointer;
		width: 40px;
		height: 40px;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
	}

	@media (max-width: 768px) {
		.menu-toggle {
			display: flex;
		}

		.nav-links {
			position: fixed;
			top: 64px;
			left: 0;
			right: 0;
			background: rgba(9, 9, 11, 0.97);
			backdrop-filter: blur(12px);
			flex-direction: column;
			padding: 1.5rem;
			gap: 1rem;
			border-bottom: 1px solid var(--border);
			transform: translateY(-100%);
			opacity: 0;
			pointer-events: none;
			transition: transform 0.3s ease, opacity 0.3s ease;
		}

		.nav-links.open {
			transform: translateY(0);
			opacity: 1;
			pointer-events: all;
		}

		.nav-links a {
			font-size: 0.9rem;
			padding: 0.5rem 0;
		}
	}
</style>
