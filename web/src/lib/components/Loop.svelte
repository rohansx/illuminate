<script lang="ts">
	import { reveal } from '$lib/actions/reveal';

	const steps = [
		{
			num: '01',
			name: 'DISCOVER',
			desc: 'AI matches issues to your skills, goals, and available time. No more scrolling through irrelevant lists.',
			color: 'var(--amber)'
		},
		{
			num: '02',
			name: 'UNDERSTAND',
			desc: 'AI breaks down the project, explains the issue in context, and suggests a step-by-step approach.',
			color: 'var(--amber)'
		},
		{
			num: '03',
			name: 'CONTRIBUTE',
			desc: 'Get smart questions to ask maintainers, PR templates, and etiquette coaching before you write code.',
			color: 'var(--amber)'
		},
		{
			num: '04',
			name: 'TRACK',
			desc: 'Every merged PR is auto-detected. Build a shareable portfolio of your open source impact.',
			color: 'var(--amber)'
		},
		{
			num: '05',
			name: 'GROW',
			desc: 'Level up from Explorer to Luminary. Get AI-suggested next steps that push your skills forward.',
			color: 'var(--amber)'
		}
	];
</script>

<section id="loop">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> THE SOLUTION</p>
		<h2 class="section-title" use:reveal>A complete contribution cycle</h2>
		<p class="section-desc" use:reveal>
			Illuminate connects the full loop that no other tool covers.
			From finding the issue to showcasing your work.
		</p>

		<div class="steps">
			{#each steps as step, i}
				<div class="step" use:reveal class:reveal-d1={i === 0} class:reveal-d2={i === 1} class:reveal-d3={i === 2} class:reveal-d4={i === 3} class:reveal-d5={i === 4}>
					<div class="step-marker">
						<span class="step-num">{step.num}</span>
						<div class="step-line"></div>
					</div>
					<div class="step-content">
						<h3>{step.name}</h3>
						<p>{step.desc}</p>
					</div>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.steps {
		margin-top: 3.5rem;
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.step {
		display: flex;
		gap: 1.75rem;
		align-items: flex-start;
	}

	.step-marker {
		display: flex;
		flex-direction: column;
		align-items: center;
		flex-shrink: 0;
		width: 40px;
	}

	.step-num {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--amber);
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid rgba(245, 158, 11, 0.25);
		border-radius: 50%;
		background: rgba(245, 158, 11, 0.05);
		flex-shrink: 0;
	}

	.step-line {
		width: 1px;
		height: 100%;
		min-height: 2rem;
		background: linear-gradient(to bottom, var(--border-bright), transparent);
	}

	.step:last-child .step-line {
		display: none;
	}

	.step-content {
		padding-bottom: 2.5rem;
	}

	.step:last-child .step-content {
		padding-bottom: 0;
	}

	h3 {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--text-bright);
		letter-spacing: 0.1em;
		margin-bottom: 0.5rem;
	}

	.step-content p {
		font-size: 0.8rem;
		color: var(--text-muted);
		line-height: 1.75;
		max-width: 480px;
	}
</style>
