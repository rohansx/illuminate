<script lang="ts">
	import { reveal } from '$lib/actions/reveal';
</script>

<section class="hero" id="hero">
	<div class="hero-glow"></div>
	<div class="container">
		<div class="hero-content">
			<p class="hero-pre" use:reveal>
				<span class="prompt">&gt;</span> your open source copilot<span class="cursor">_</span>
			</p>

			<h1 use:reveal class="reveal-d1">
				<span class="glow">Light</span> the path to<br />open source
			</h1>

			<p class="hero-desc" use:reveal class:reveal-d2={true}>
				Discover issues matched to your skills. Get AI-powered guidance
				before your first line of code. Build a portfolio that proves
				your impact.
			</p>

			<div class="hero-actions" use:reveal class:reveal-d3={true}>
				<a href="/login" class="btn-primary">Get Started — Free</a>
				<a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener" class="btn-outline">
					View on GitHub <span class="arrow">→</span>
				</a>
			</div>
		</div>

		<div class="terminal" use:reveal class:reveal-d4={true}>
			<div class="terminal-bar">
				<div class="terminal-dots">
					<span class="dot dot-red"></span>
					<span class="dot dot-yellow"></span>
					<span class="dot dot-green"></span>
				</div>
				<span class="terminal-title">illuminate — match</span>
			</div>
			<div class="terminal-body">
				<div class="line">
					<span class="prompt-char">$</span>
					<span class="cmd">illuminate match</span>
				</div>
				<div class="line blank"></div>
				<div class="line">
					<span class="dim">  Scanning profile...</span>
					<span class="green"> done ✓</span>
				</div>
				<div class="line">
					<span class="dim">  Skills:</span>
					<span class="text-bright"> Python, TypeScript, React, Go</span>
				</div>
				<div class="line">
					<span class="dim">  Matched:</span>
					<span class="amber"> 23 issues</span>
					<span class="dim"> across 140 repos</span>
				</div>
				<div class="line blank"></div>
				<div class="line">
					<span class="border-line">  ┌─ #1 ──────────────────────── </span><span class="amber">94% match</span><span class="border-line"> ─┐</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="text-bright"> fastapi/fastapi</span>
					<span class="dim"> #4521</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="text-white"> Fix OAuth2 token refresh logic</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="tag">Python</span>
					<span class="tag tag-label">good-first-issue</span>
					<span class="dim">~2hrs</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  └───────────────────────────────────────┘</span>
				</div>
				<div class="line blank"></div>
				<div class="line">
					<span class="border-line">  ┌─ #2 ──────────────────────── </span><span class="amber">89% match</span><span class="border-line"> ─┐</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="text-bright"> sveltejs/kit</span>
					<span class="dim"> #9182</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="text-white"> Add preload hints for CSS modules</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  │</span>
					<span class="tag">TypeScript</span>
					<span class="tag tag-label">enhancement</span>
					<span class="dim">~3hrs</span>
					<span class="border-line" style="float:right">│</span>
				</div>
				<div class="line">
					<span class="border-line">  └───────────────────────────────────────┘</span>
				</div>
				<div class="line blank"></div>
				<div class="line">
					<span class="prompt-char">$</span>
					<span class="cursor-blink">█</span>
				</div>
			</div>
		</div>
	</div>
</section>

<style>
	.hero {
		padding-top: 10rem;
		padding-bottom: 5rem;
		position: relative;
		overflow: hidden;
	}

	.hero-glow {
		position: absolute;
		top: -300px;
		left: 50%;
		transform: translateX(-50%);
		width: 800px;
		height: 700px;
		background: radial-gradient(
			ellipse at center,
			rgba(245, 158, 11, 0.07) 0%,
			rgba(245, 158, 11, 0.03) 30%,
			transparent 70%
		);
		pointer-events: none;
	}

	.container {
		position: relative;
	}

	.hero-content {
		text-align: center;
		margin-bottom: 4rem;
	}

	.hero-pre {
		font-size: 0.85rem;
		color: var(--text-muted);
		margin-bottom: 1.75rem;
		font-weight: 400;
	}

	.prompt {
		color: var(--amber);
		margin-right: 0.3rem;
	}

	.cursor {
		color: var(--amber);
		animation: blink 1.2s step-end infinite;
	}

	h1 {
		font-size: clamp(2rem, 6vw, 3.5rem);
		font-weight: 700;
		color: var(--text-bright);
		line-height: 1.15;
		letter-spacing: -0.03em;
		margin-bottom: 1.5rem;
	}

	.glow {
		color: var(--amber-bright);
		text-shadow:
			0 0 30px rgba(251, 191, 36, 0.35),
			0 0 80px rgba(251, 191, 36, 0.15);
	}

	.hero-desc {
		font-size: 0.9rem;
		color: var(--text-muted);
		max-width: 540px;
		margin: 0 auto 2.5rem;
		line-height: 1.8;
	}

	.hero-actions {
		display: flex;
		gap: 1rem;
		justify-content: center;
		flex-wrap: wrap;
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		padding: 0.65rem 1.75rem;
		background: var(--amber);
		color: var(--bg);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		border-radius: 4px;
		letter-spacing: 0.01em;
		transition: background 0.2s ease, transform 0.15s ease, box-shadow 0.2s ease;
	}

	.btn-primary:hover {
		background: var(--amber-bright);
		color: var(--bg);
		transform: translateY(-1px);
		box-shadow: 0 0 20px var(--amber-glow-strong), 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.btn-outline {
		display: inline-flex;
		align-items: center;
		padding: 0.65rem 1.75rem;
		background: transparent;
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 500;
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		transition: border-color 0.2s ease, color 0.2s ease, transform 0.15s ease;
	}

	.btn-outline:hover {
		border-color: var(--text-dim);
		color: var(--text-bright);
		transform: translateY(-1px);
	}

	.arrow {
		margin-left: 0.4rem;
		transition: transform 0.2s ease;
	}

	.btn-outline:hover .arrow {
		transform: translateX(2px);
	}

	/* Terminal mock */
	.terminal {
		max-width: 620px;
		margin: 0 auto;
		border: 1px solid var(--border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--bg-raised);
		box-shadow:
			0 0 40px var(--amber-glow),
			0 20px 60px rgba(0, 0, 0, 0.5);
	}

	.terminal-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.65rem 1rem;
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
	.dot-green { background: var(--green); }

	.terminal-title {
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-left: auto;
		margin-right: auto;
	}

	.terminal-body {
		padding: 1.25rem 1.25rem;
		font-size: 0.78rem;
		line-height: 1.65;
		overflow-x: auto;
	}

	.line {
		white-space: pre;
		min-height: 1.65em;
	}

	.line.blank {
		min-height: 0.8em;
	}

	.prompt-char {
		color: var(--green);
		margin-right: 0.5rem;
	}

	.cmd {
		color: var(--text-bright);
		font-weight: 600;
	}

	.dim { color: var(--text-dim); }
	.green { color: var(--green); }
	.amber { color: var(--amber); font-weight: 600; }
	.text-bright { color: var(--text-bright); }
	.text-white { color: var(--text); }
	.border-line { color: var(--border-bright); }

	.tag {
		display: inline-block;
		padding: 0 0.4rem;
		font-size: 0.68rem;
		border-radius: 3px;
		background: rgba(245, 158, 11, 0.1);
		color: var(--amber);
		border: 1px solid rgba(245, 158, 11, 0.2);
		margin-right: 0.3rem;
	}

	.tag-label {
		background: rgba(74, 222, 128, 0.08);
		color: var(--green);
		border-color: rgba(74, 222, 128, 0.2);
	}

	.cursor-blink {
		color: var(--amber);
		animation: blink 1.2s step-end infinite;
	}

	@media (max-width: 768px) {
		.hero {
			padding-top: 7rem;
		}

		.terminal-body {
			font-size: 0.68rem;
		}
	}
</style>
