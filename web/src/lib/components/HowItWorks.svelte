<script lang="ts">
	import { reveal } from '$lib/actions/reveal';

	const steps = [
		{
			num: '1',
			title: 'Connect GitHub',
			desc: 'Sign in with GitHub. Illuminate analyzes your repos to understand your languages, frameworks, and experience level.',
			cmd: '$ gh auth login --scope read:user,public_repo'
		},
		{
			num: '2',
			title: 'Get Matched',
			desc: 'Your personalized feed surfaces issues ranked by skill match, repo health, and growth potential. Filter by language, difficulty, or time.',
			cmd: '$ illuminate match --lang python --difficulty beginner'
		},
		{
			num: '3',
			title: 'Start Contributing',
			desc: 'Click into an issue for an AI deep dive. Get the approach, ask the right questions, submit your PR, and watch your portfolio grow.',
			cmd: '$ illuminate dive fastapi/fastapi#4521'
		}
	];
</script>

<section id="get-started">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> GET STARTED</p>
		<h2 class="section-title" use:reveal>Three steps to your<br />first contribution</h2>

		<div class="steps-row">
			{#each steps as step, i}
				<div class="step-card" use:reveal class:reveal-d1={i === 0} class:reveal-d2={i === 1} class:reveal-d3={i === 2}>
					<span class="step-num">{step.num}</span>
					<h3>{step.title}</h3>
					<p>{step.desc}</p>
					<code class="step-cmd">{step.cmd}</code>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.steps-row {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1.5rem;
		margin-top: 3.5rem;
	}

	.step-card {
		padding: 2rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--bg);
		transition: border-color 0.25s ease, background 0.25s ease;
	}

	.step-card:hover {
		border-color: var(--border-bright);
		background: var(--bg-raised);
	}

	.step-num {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--amber);
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-radius: 50%;
		background: rgba(245, 158, 11, 0.06);
		margin-bottom: 1.25rem;
	}

	h3 {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.65rem;
	}

	p {
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.75;
		margin-bottom: 1.25rem;
	}

	.step-cmd {
		display: block;
		font-family: var(--font-mono);
		font-size: 0.68rem;
		color: var(--text-dim);
		padding: 0.6rem 0.8rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 4px;
		overflow-x: auto;
		white-space: nowrap;
	}

	@media (max-width: 768px) {
		.steps-row {
			grid-template-columns: 1fr;
		}
	}
</style>
