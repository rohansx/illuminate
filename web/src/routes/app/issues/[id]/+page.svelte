<script lang="ts">
	import { api, type Issue } from '$lib/api';
	import { onMount } from 'svelte';
	import { page } from '$app/state';

	let issue = $state<Issue | null>(null);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			issue = await api.getIssue(page.params.id);
		} catch (e: any) {
			error = e.message || 'Failed to load issue';
		} finally {
			loading = false;
		}
	});

	function difficultyLabel(d: number): string {
		if (d === 1) return 'beginner';
		if (d === 3) return 'advanced';
		return 'intermediate';
	}

	function difficultyColor(d: number): string {
		if (d === 1) return 'var(--green)';
		if (d === 3) return 'var(--red)';
		return 'var(--amber)';
	}
</script>

{#if loading}
	<div class="loading">
		<div class="spinner"></div>
	</div>
{:else if error}
	<div class="error">
		<p>{error}</p>
		<a href="/app/feed">&larr; back to feed</a>
	</div>
{:else if issue}
	<div class="issue-detail">
		<a href="/app/feed" class="back-link">&larr; back to feed</a>

		<div class="issue-header">
			<div class="repo-badge">{issue.repo?.owner}/{issue.repo?.name}</div>
			<h1>{issue.title} <span class="number">#{issue.number}</span></h1>
		</div>

		<div class="meta-row">
			<span class="difficulty" style="color: {difficultyColor(issue.difficulty)}">
				{difficultyLabel(issue.difficulty)}
			</span>
			<span class="time">{issue.time_estimate}</span>
			<span class="comments">{issue.comment_count} comments</span>
			<span class="freshness">{Math.round(issue.freshness_score * 100)}% fresh</span>
		</div>

		{#if issue.labels?.length}
			<div class="labels">
				{#each issue.labels as label}
					<span class="label">{label}</span>
				{/each}
			</div>
		{/if}

		{#if issue.skills?.length}
			<div class="skills">
				<h3>skills needed</h3>
				<div class="skill-list">
					{#each issue.skills as skill}
						<span class="skill">{skill.language}{skill.framework ? ` / ${skill.framework}` : ''}</span>
					{/each}
				</div>
			</div>
		{/if}

		{#if issue.match_reasons?.length}
			<div class="match-info">
				<h3>why this matches you</h3>
				<div class="reasons">
					{#each issue.match_reasons as reason}
						<span class="reason">{reason}</span>
					{/each}
				</div>
			</div>
		{/if}

		<div class="body-section">
			<h3>description</h3>
			<div class="body-content">
				<pre>{issue.body || 'No description provided.'}</pre>
			</div>
		</div>

		<div class="actions">
			<a
				href="https://github.com/{issue.repo?.owner}/{issue.repo?.name}/issues/{issue.number}"
				target="_blank"
				rel="noopener noreferrer"
				class="github-btn"
			>
				open on github &rarr;
			</a>
		</div>
	</div>
{/if}

<style>
	.loading {
		display: flex;
		justify-content: center;
		padding: 3rem;
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--border);
		border-top-color: var(--amber);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }

	.error {
		text-align: center;
		padding: 3rem;
		color: var(--red);
	}

	.error a {
		color: var(--amber);
		text-decoration: none;
		display: inline-block;
		margin-top: 1rem;
	}

	.issue-detail { max-width: 800px; }

	.back-link {
		color: var(--text-muted);
		text-decoration: none;
		font-size: 0.85rem;
		display: inline-block;
		margin-bottom: 1.5rem;
		transition: color 0.15s;
	}

	.back-link:hover { color: var(--amber); }

	.issue-header { margin-bottom: 1rem; }

	.repo-badge {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.issue-header h1 {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text-bright);
		line-height: 1.4;
	}

	.number {
		color: var(--text-dim);
		font-weight: 400;
	}

	.meta-row {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border);
	}

	.labels {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-bottom: 1.5rem;
	}

	.label {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		background: var(--amber-glow);
		color: var(--amber);
		border-radius: 3px;
	}

	.skills, .match-info, .body-section {
		margin-bottom: 1.5rem;
	}

	h3 {
		font-size: 0.85rem;
		color: var(--text-muted);
		font-weight: 500;
		margin-bottom: 0.75rem;
		text-transform: lowercase;
	}

	.skill-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.skill {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		border-radius: 3px;
	}

	.reasons {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.reason {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		background: rgba(74, 222, 128, 0.1);
		color: var(--green);
		border-radius: 3px;
	}

	.body-content {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.25rem;
	}

	.body-content pre {
		white-space: pre-wrap;
		word-break: break-word;
		font-size: 0.85rem;
		color: var(--text);
		line-height: 1.6;
		font-family: var(--font-mono);
	}

	.actions {
		margin-top: 2rem;
	}

	.github-btn {
		display: inline-block;
		background: var(--amber);
		color: var(--bg);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		font-weight: 600;
		padding: 0.6rem 1.25rem;
		border-radius: 4px;
		text-decoration: none;
		transition: all 0.15s;
	}

	.github-btn:hover {
		background: var(--amber-bright);
	}
</style>
