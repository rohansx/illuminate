<script lang="ts">
	const year = new Date().getFullYear();
</script>

<footer>
	<div class="container">
		<div class="footer-top">
			<div class="footer-brand">
				<a href="/" class="logo">
					<span class="logo-name">illuminate</span><span class="logo-dot">.sh</span>
				</a>
				<p class="tagline">Light the path to open source.</p>
			</div>

			<div class="footer-links">
				<div class="footer-col">
					<h4>Product</h4>
					<a href="#features">Features</a>
					<a href="/awesome">Awesome</a>
					<a href="#get-started">How It Works</a>
					<a href="/login">Get Started</a>
				</div>
				<div class="footer-col">
					<h4>Docs</h4>
					<a href="https://github.com/rohansx/illuminate/blob/main/docs/architecture.md" target="_blank" rel="noopener">Architecture</a>
					<a href="https://github.com/rohansx/illuminate/blob/main/docs/roadmap.md" target="_blank" rel="noopener">Roadmap</a>
					<a href="https://github.com/rohansx/illuminate/blob/main/docs/product-overview.md" target="_blank" rel="noopener">Product Overview</a>
				</div>
				<div class="footer-col">
					<h4>Community</h4>
					<a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener">GitHub</a>
					<a href="https://github.com/rohansx/illuminate/blob/main/CONTRIBUTING.md" target="_blank" rel="noopener">Contributing</a>
					<a href="https://github.com/rohansx/illuminate/issues" target="_blank" rel="noopener">Issues</a>
				</div>
			</div>
		</div>

		<div class="footer-bottom">
			<p class="copyright">© {year} Illuminate. MIT License.</p>
			<p class="built-by">Built by <a href="https://github.com/rohansharma" target="_blank" rel="noopener">Rohan Sharma</a> and contributors.</p>
		</div>
	</div>
</footer>

<style>
	footer {
		border-top: 1px solid var(--border);
		padding: 4rem 1.5rem 2.5rem;
	}

	.container {
		max-width: var(--content-width);
		margin: 0 auto;
	}

	.footer-top {
		display: flex;
		justify-content: space-between;
		gap: 3rem;
		margin-bottom: 3rem;
	}

	.footer-brand {
		flex-shrink: 0;
	}

	.logo {
		font-size: 1rem;
		font-weight: 700;
		display: inline-block;
		margin-bottom: 0.5rem;
	}

	.logo-name {
		color: var(--amber);
	}

	.logo-dot {
		color: var(--text-dim);
		font-weight: 400;
	}

	.tagline {
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.footer-links {
		display: flex;
		gap: 4rem;
	}

	.footer-col {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	h4 {
		font-size: 0.72rem;
		font-weight: 600;
		color: var(--text-muted);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		margin-bottom: 0.25rem;
	}

	.footer-col a {
		font-size: 0.78rem;
		color: var(--text-dim);
		transition: color 0.2s ease;
	}

	.footer-col a:hover {
		color: var(--text);
	}

	.footer-bottom {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 2rem;
		border-top: 1px solid var(--border);
	}

	.copyright, .built-by {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.built-by a {
		color: var(--text-muted);
	}

	.built-by a:hover {
		color: var(--amber);
	}

	@media (max-width: 768px) {
		.footer-top {
			flex-direction: column;
		}

		.footer-links {
			gap: 2.5rem;
			flex-wrap: wrap;
		}

		.footer-bottom {
			flex-direction: column;
			gap: 0.5rem;
			text-align: center;
		}
	}
</style>
