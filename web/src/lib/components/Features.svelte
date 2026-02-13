<script lang="ts">
	import { reveal } from '$lib/actions/reveal';

	const features = [
		{
			tag: 'match',
			title: 'Smart Issue Matching',
			desc: 'Multi-factor ranking based on your skills, goals, and time. Skill match, repo health, freshness, and competition — not just labels.'
		},
		{
			tag: 'dive',
			title: 'AI Deep Dives',
			desc: 'Five-section mentor experience: project overview, issue context, suggested approach, questions to ask, and red flags. Powered by Claude.'
		},
		{
			tag: 'track',
			title: 'Contribution Tracker',
			desc: 'Auto-detects merged PRs to external repos. Lines added, files changed, skills demonstrated — all recorded without lifting a finger.'
		},
		{
			tag: 'share',
			title: 'Shareable Portfolio',
			desc: 'Public profile at illuminate.sh/you with timeline, project views, and stats. Embed an SVG widget in your GitHub README.'
		},
		{
			tag: 'grow',
			title: 'Growth Engine',
			desc: 'Six progression levels from Explorer to Luminary. Skill radar visualization. AI-suggested next steps that push you forward.'
		},
		{
			tag: 'watch',
			title: 'Watchlist & Alerts',
			desc: 'Save issues. Get notified when maintainers respond, when issues get claimed, or when new matches land in your feed.'
		}
	];
</script>

<section id="features">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> FEATURES</p>
		<h2 class="section-title" use:reveal>Everything you need,<br />nothing you don't</h2>

		<div class="feature-grid">
			{#each features as feature, i}
				<div class="feature-card" use:reveal class:reveal-d1={i === 0} class:reveal-d2={i === 1} class:reveal-d3={i === 2} class:reveal-d4={i === 3} class:reveal-d5={i === 4} class:reveal-d6={i === 5}>
					<div class="feature-tag">[{feature.tag}]</div>
					<h3>{feature.title}</h3>
					<p>{feature.desc}</p>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.feature-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1px;
		margin-top: 3.5rem;
		background: var(--border);
		border: 1px solid var(--border);
		border-radius: 6px;
		overflow: hidden;
	}

	.feature-card {
		padding: 2rem;
		background: var(--bg);
		transition: background 0.2s ease;
	}

	.feature-card:hover {
		background: var(--bg-raised);
	}

	.feature-tag {
		font-size: 0.72rem;
		color: var(--amber);
		font-weight: 500;
		margin-bottom: 1rem;
		opacity: 0.7;
	}

	h3 {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.75rem;
	}

	p {
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.75;
	}

	@media (max-width: 900px) {
		.feature-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (max-width: 540px) {
		.feature-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
