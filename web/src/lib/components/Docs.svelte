<script lang="ts">
	import { reveal } from '$lib/actions/reveal';

	const docs = [
		{
			tag: 'spec',
			title: 'Product Overview',
			desc: 'Full product specification — features, user journeys, data models, AI strategy, monetization, and competitive landscape.',
			href: 'https://github.com/rohansx/illuminate/blob/main/docs/product-overview.md',
			lines: '696 lines'
		},
		{
			tag: 'arch',
			title: 'Architecture',
			desc: 'System design — service breakdown, data flows, background workers, database schema, security model, and infrastructure.',
			href: 'https://github.com/rohansx/illuminate/blob/main/docs/architecture.md',
			lines: '280 lines'
		},
		{
			tag: 'plan',
			title: 'Roadmap',
			desc: 'Phase-by-phase progress tracker — what\'s shipped, what\'s in progress, milestones, key decisions log, and metrics.',
			href: 'https://github.com/rohansx/illuminate/blob/main/docs/roadmap.md',
			lines: '120 lines'
		},
		{
			tag: 'contrib',
			title: 'Contributing',
			desc: 'Dev environment setup, code style, branching strategy, commit conventions, PR process, and issue labels.',
			href: 'https://github.com/rohansx/illuminate/blob/main/CONTRIBUTING.md',
			lines: '220 lines'
		}
	];
</script>

<section id="docs">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> DOCUMENTATION</p>
		<h2 class="section-title" use:reveal>Everything is documented</h2>
		<p class="section-desc" use:reveal>
			We build in the open with full transparency. Every architectural decision,
			every roadmap milestone, every contribution guideline — all public.
		</p>

		<div class="docs-grid">
			{#each docs as doc, i}
				<a
					href={doc.href}
					target="_blank"
					rel="noopener"
					class="doc-card"
					use:reveal
					class:reveal-d1={i === 0}
					class:reveal-d2={i === 1}
					class:reveal-d3={i === 2}
					class:reveal-d4={i === 3}
				>
					<div class="doc-top">
						<span class="doc-tag">[{doc.tag}]</span>
						<span class="doc-lines">{doc.lines}</span>
					</div>
					<h3>{doc.title}</h3>
					<p>{doc.desc}</p>
					<span class="doc-link">Read docs <span class="arrow">→</span></span>
				</a>
			{/each}
		</div>

		<div class="docs-terminal" use:reveal>
			<div class="terminal-bar">
				<div class="terminal-dots">
					<span class="dot dot-red"></span>
					<span class="dot dot-yellow"></span>
					<span class="dot dot-green"></span>
				</div>
				<span class="terminal-title">project structure</span>
			</div>
			<div class="terminal-body">
				<div class="line"><span class="amber">illuminate/</span></div>
				<div class="line">├── <span class="text-bright">README.md</span>              <span class="dim"># SEO-optimized project overview</span></div>
				<div class="line">├── <span class="text-bright">CONTRIBUTING.md</span>        <span class="dim"># How to contribute</span></div>
				<div class="line">├── <span class="text-bright">LICENSE</span>                <span class="dim"># MIT</span></div>
				<div class="line">├── <span class="amber">docs/</span></div>
				<div class="line">│   ├── product-overview.md <span class="dim"># Full product spec</span></div>
				<div class="line">│   ├── architecture.md     <span class="dim"># System design</span></div>
				<div class="line">│   └── roadmap.md          <span class="dim"># Plans + progress</span></div>
				<div class="line">├── <span class="amber">web/</span>                    <span class="dim"># SvelteKit frontend</span></div>
				<div class="line">│   └── src/</div>
				<div class="line">│       ├── lib/components/ <span class="dim"># UI components</span></div>
				<div class="line">│       └── routes/         <span class="dim"># Pages</span></div>
				<div class="line">└── <span class="amber">api/</span>                    <span class="dim"># Go backend (coming soon)</span></div>
			</div>
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.docs-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		margin-top: 3.5rem;
	}

	.doc-card {
		display: flex;
		flex-direction: column;
		padding: 1.75rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--bg);
		transition: border-color 0.25s ease, background 0.25s ease, transform 0.2s ease;
		text-decoration: none;
	}

	.doc-card:hover {
		border-color: rgba(245, 158, 11, 0.25);
		background: var(--bg-raised);
		transform: translateY(-2px);
	}

	.doc-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.doc-tag {
		font-size: 0.7rem;
		color: var(--amber);
		font-weight: 500;
		opacity: 0.6;
	}

	.doc-lines {
		font-size: 0.65rem;
		color: var(--text-dim);
	}

	.doc-card h3 {
		font-size: 0.92rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.6rem;
	}

	.doc-card p {
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.7;
		flex-grow: 1;
		margin-bottom: 1.25rem;
	}

	.doc-link {
		font-size: 0.75rem;
		color: var(--amber);
		font-weight: 500;
		opacity: 0.7;
		transition: opacity 0.2s ease;
	}

	.doc-card:hover .doc-link {
		opacity: 1;
	}

	.arrow {
		display: inline-block;
		transition: transform 0.2s ease;
	}

	.doc-card:hover .arrow {
		transform: translateX(3px);
	}

	/* Project structure terminal */
	.docs-terminal {
		margin-top: 2rem;
		border: 1px solid var(--border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--bg-raised);
	}

	.terminal-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 1rem;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
	}

	.terminal-dots {
		display: flex;
		gap: 6px;
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
	}

	.dot-red { background: #f87171; }
	.dot-yellow { background: var(--amber); }
	.dot-green { background: var(--green, #4ade80); }

	.terminal-title {
		font-size: 0.68rem;
		color: var(--text-dim);
		margin-left: auto;
		margin-right: auto;
	}

	.terminal-body {
		padding: 1.25rem;
		font-size: 0.75rem;
		line-height: 1.7;
	}

	.line {
		white-space: pre;
		color: var(--text-muted);
	}

	.amber { color: var(--amber); font-weight: 600; }
	.dim { color: var(--text-dim); }
	.text-bright { color: var(--text-bright); }

	@media (max-width: 640px) {
		.docs-grid {
			grid-template-columns: 1fr;
		}

		.terminal-body {
			font-size: 0.65rem;
			overflow-x: auto;
		}
	}
</style>
