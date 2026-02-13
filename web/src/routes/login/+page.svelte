<script lang="ts">
	import { api } from '$lib/api';
	import { reveal } from '$lib/actions/reveal';

	let loading = $state(false);

	function handleLogin() {
		loading = true;
		window.location.href = api.loginURL;
	}
</script>

<svelte:head>
	<title>sign in — illuminate</title>
	<meta name="description" content="Sign in with GitHub to get personalized open source issue recommendations." />
</svelte:head>

<div class="login-page">
	<div class="login-glow"></div>

	<a href="/" class="back-link" use:reveal>
		<span class="arrow">&larr;</span> back to home
	</a>

	<div class="login-container" use:reveal>
		<div class="terminal-window">
			<div class="terminal-bar">
				<div class="dots">
					<span class="dot dot-red"></span>
					<span class="dot dot-yellow"></span>
					<span class="dot dot-green"></span>
				</div>
				<span class="terminal-title">illuminate — auth</span>
			</div>

			<div class="terminal-body">
				<div class="line">
					<span class="prompt">$</span>
					<span class="cmd">illuminate auth login</span>
				</div>
				<div class="line blank"></div>
				<div class="line">
					<span class="dim">  Connecting to GitHub...</span>
				</div>
				<div class="line blank"></div>

				<div class="auth-card">
					<div class="logo-mark">
						<span class="logo-text">illuminate</span><span class="logo-dot">.sh</span>
					</div>

					<h1>Sign in to continue</h1>
					<p class="auth-desc">
						Connect your GitHub account to get personalized issue
						recommendations matched to your skills.
					</p>

					<button class="github-btn" onclick={handleLogin} disabled={loading}>
						<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
							<path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
						</svg>
						{#if loading}
							Redirecting...
						{:else}
							Continue with GitHub
						{/if}
					</button>

					<div class="permissions">
						<span class="perm-label">We'll request access to:</span>
						<div class="perm-list">
							<span class="perm-item">
								<span class="check">&#10003;</span> Public profile info
							</span>
							<span class="perm-item">
								<span class="check">&#10003;</span> Repository languages
							</span>
							<span class="perm-item">
								<span class="check">&#10003;</span> Public repo metadata
							</span>
						</div>
					</div>
				</div>

				<div class="line blank"></div>
				<div class="line">
					<span class="dim">  Your data stays private. We never push code or modify repos.</span>
				</div>
				<div class="line">
					<span class="prompt">$</span>
					<span class="cursor-blink">_</span>
				</div>
			</div>
		</div>
	</div>

	<p class="footer-note" use:reveal>
		Open source under <a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener">MIT License</a>
	</p>
</div>

<style>
	.login-page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem 1.5rem;
		position: relative;
	}

	.login-glow {
		position: absolute;
		top: -200px;
		left: 50%;
		transform: translateX(-50%);
		width: 600px;
		height: 500px;
		background: radial-gradient(
			ellipse at center,
			rgba(245, 158, 11, 0.06) 0%,
			rgba(245, 158, 11, 0.02) 40%,
			transparent 70%
		);
		pointer-events: none;
	}

	.back-link {
		position: absolute;
		top: 1.5rem;
		left: 1.5rem;
		color: var(--text-dim);
		font-size: 0.75rem;
		transition: color 0.2s;
	}

	.back-link:hover {
		color: var(--text-muted);
	}

	.back-link .arrow {
		margin-right: 0.3rem;
	}

	.login-container {
		width: 100%;
		max-width: 480px;
	}

	/* Terminal window */
	.terminal-window {
		border: 1px solid var(--border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--bg-raised);
		box-shadow:
			0 0 60px rgba(245, 158, 11, 0.06),
			0 25px 50px rgba(0, 0, 0, 0.4);
	}

	.terminal-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.65rem 1rem;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
	}

	.dots {
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
		padding: 1.25rem;
		font-size: 0.78rem;
		line-height: 1.65;
	}

	.line {
		white-space: pre;
		min-height: 1.65em;
	}

	.line.blank {
		min-height: 0.6em;
	}

	.prompt {
		color: var(--green);
		margin-right: 0.5rem;
	}

	.cmd {
		color: var(--text-bright);
		font-weight: 600;
	}

	.dim { color: var(--text-dim); }

	.cursor-blink {
		color: var(--amber);
		animation: blink 1.2s step-end infinite;
	}

	/* Auth card inside terminal */
	.auth-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.75rem;
		margin: 0.5rem 0;
		text-align: center;
	}

	.logo-mark {
		margin-bottom: 1.25rem;
	}

	.logo-text {
		color: var(--amber);
		font-weight: 700;
		font-size: 1.1rem;
	}

	.logo-dot {
		color: var(--text-dim);
		font-weight: 400;
		font-size: 1.1rem;
	}

	.auth-card h1 {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.5rem;
	}

	.auth-desc {
		font-size: 0.78rem;
		color: var(--text-muted);
		line-height: 1.6;
		margin-bottom: 1.5rem;
	}

	.github-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.7rem 1.5rem;
		background: #fafafa;
		color: #09090b;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		border: none;
		border-radius: 5px;
		cursor: pointer;
		transition: background 0.2s, transform 0.15s, box-shadow 0.2s;
	}

	.github-btn:hover:not(:disabled) {
		background: #ffffff;
		transform: translateY(-1px);
		box-shadow: 0 4px 16px rgba(255, 255, 255, 0.1);
	}

	.github-btn:active:not(:disabled) {
		transform: translateY(0);
	}

	.github-btn:disabled {
		opacity: 0.7;
		cursor: wait;
	}

	.permissions {
		margin-top: 1.25rem;
		text-align: left;
	}

	.perm-label {
		font-size: 0.68rem;
		color: var(--text-dim);
		display: block;
		margin-bottom: 0.5rem;
	}

	.perm-list {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.perm-item {
		font-size: 0.72rem;
		color: var(--text-muted);
	}

	.check {
		color: var(--green);
		margin-right: 0.4rem;
		font-size: 0.7rem;
	}

	/* Footer */
	.footer-note {
		margin-top: 2rem;
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.footer-note a {
		color: var(--text-muted);
	}

	@media (max-width: 520px) {
		.auth-card {
			padding: 1.25rem;
		}

		.terminal-body {
			padding: 1rem;
		}
	}
</style>
