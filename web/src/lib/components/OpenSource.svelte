<script lang="ts">
	import { reveal } from '$lib/actions/reveal';
</script>

<section id="open-source">
	<div class="container">
		<div class="oss-box" use:reveal>
			<p class="section-label"><span>//</span> OPEN SOURCE</p>
			<h2 class="section-title">Built in the open</h2>
			<p class="oss-desc">
				Illuminate is open source under the MIT license. We're building a tool
				to make open source contribution easier — it only makes sense to build
				it the same way. Star the repo, open an issue, or submit a PR.
			</p>

			<div class="oss-actions">
				<a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener" class="btn-star">
					<svg width="16" height="16" viewBox="0 0 16 16" fill="none">
						<path d="M8 1.5l1.85 3.75L14 5.9l-3 2.925.71 4.125L8 10.9l-3.71 1.95L5 8.825 2 5.9l4.15-.65L8 1.5z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
					</svg>
					Star on GitHub
				</a>
				<a href="https://github.com/rohansx/illuminate/blob/main/CONTRIBUTING.md" target="_blank" rel="noopener" class="btn-contrib">Read Contributing Guide →</a>
			</div>

			<div class="oss-stats">
				<div class="stat">
					<span class="stat-value">MIT</span>
					<span class="stat-label">License</span>
				</div>
				<div class="stat-sep"></div>
				<div class="stat">
					<span class="stat-value">SvelteKit</span>
					<span class="stat-label">Frontend</span>
				</div>
				<div class="stat-sep"></div>
				<div class="stat">
					<span class="stat-value">Go</span>
					<span class="stat-label">Backend</span>
				</div>
				<div class="stat-sep"></div>
				<div class="stat">
					<span class="stat-value">Claude</span>
					<span class="stat-label">AI Engine</span>
				</div>
			</div>
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.oss-box {
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 3.5rem;
		background: var(--bg-raised);
		position: relative;
		overflow: hidden;
	}

	.oss-box::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 1px;
		background: linear-gradient(
			90deg,
			transparent,
			var(--amber) 50%,
			transparent
		);
		opacity: 0.4;
	}

	.oss-desc {
		font-size: 0.85rem;
		color: var(--text-muted);
		line-height: 1.8;
		max-width: 540px;
		margin-bottom: 2rem;
	}

	.oss-actions {
		display: flex;
		gap: 1rem;
		margin-bottom: 2.5rem;
		flex-wrap: wrap;
	}

	.btn-star {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.55rem 1.25rem;
		background: var(--amber);
		color: var(--bg);
		font-family: var(--font-mono);
		font-size: 0.78rem;
		font-weight: 600;
		border-radius: 4px;
		transition: background 0.2s ease, transform 0.15s ease;
	}

	.btn-star:hover {
		background: var(--amber-bright);
		color: var(--bg);
		transform: translateY(-1px);
	}

	.btn-contrib {
		display: inline-flex;
		align-items: center;
		padding: 0.55rem 1.25rem;
		font-family: var(--font-mono);
		font-size: 0.78rem;
		color: var(--text-muted);
		font-weight: 500;
		transition: color 0.2s ease;
	}

	.btn-contrib:hover {
		color: var(--text-bright);
	}

	.oss-stats {
		display: flex;
		align-items: center;
		gap: 1.75rem;
		padding-top: 2rem;
		border-top: 1px solid var(--border);
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.stat-value {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.stat-label {
		font-size: 0.68rem;
		color: var(--text-dim);
	}

	.stat-sep {
		width: 1px;
		height: 28px;
		background: var(--border);
	}

	@media (max-width: 640px) {
		.oss-box {
			padding: 2rem;
		}

		.oss-stats {
			flex-wrap: wrap;
			gap: 1.25rem;
		}

		.stat-sep {
			display: none;
		}
	}
</style>
