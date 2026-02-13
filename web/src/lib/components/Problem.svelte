<script lang="ts">
	import { reveal } from '$lib/actions/reveal';

	const problems = [
		{
			id: '01',
			title: 'Discovery Overload',
			desc: '420M repos on GitHub. Zero personalization. You\'re scrolling through C++ compiler issues when you write Python.'
		},
		{
			id: '02',
			title: 'The "Now What?" Wall',
			desc: 'You found an issue. The repo has 2,000 files and no architecture guide. You close the tab.'
		},
		{
			id: '03',
			title: 'Fear of the Wrong Thing',
			desc: 'Unwritten social norms. Fear of dumb questions. You never comment. The issue gets claimed by someone else.'
		},
		{
			id: '04',
			title: 'Invisible Contributions',
			desc: 'Green squares show activity, not impact. Nobody knows you fixed a critical bug in FastAPI.'
		}
	];
</script>

<section id="problem">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> THE PROBLEM</p>
		<h2 class="section-title" use:reveal>Open source is broken<br />for contributors</h2>
		<p class="section-desc" use:reveal>
			Everyone says "contribute to open source." Nobody tells you how.
			Existing tools dump thousands of issues on you with zero context.
		</p>

		<div class="problem-grid">
			{#each problems as problem, i}
				<div class="problem-card" use:reveal class:reveal-d1={i === 0} class:reveal-d2={i === 1} class:reveal-d3={i === 2} class:reveal-d4={i === 3}>
					<span class="problem-id">{problem.id}</span>
					<h3>{problem.title}</h3>
					<p>{problem.desc}</p>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.problem-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1px;
		margin-top: 3.5rem;
		background: var(--border);
		border: 1px solid var(--border);
		border-radius: 6px;
		overflow: hidden;
	}

	.problem-card {
		padding: 2rem;
		background: var(--bg);
		position: relative;
	}

	.problem-card:hover {
		background: var(--bg-raised);
	}

	.problem-id {
		font-size: 0.7rem;
		color: var(--amber);
		font-weight: 600;
		display: block;
		margin-bottom: 0.75rem;
		opacity: 0.6;
	}

	h3 {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.65rem;
	}

	p {
		font-size: 0.8rem;
		color: var(--text-muted);
		line-height: 1.7;
	}

	@media (max-width: 640px) {
		.problem-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
