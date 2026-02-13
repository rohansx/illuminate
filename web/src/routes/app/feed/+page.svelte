<script lang="ts">
	import { api, type Issue, type IssueFeed } from '$lib/api';
	import { onMount } from 'svelte';

	let feed = $state<IssueFeed | null>(null);
	let loading = $state(true);
	let searchQuery = $state('');
	let page = $state(1);

	onMount(() => loadFeed());

	async function loadFeed() {
		loading = true;
		try {
			feed = await api.getFeed(page);
		} catch (e) {
			console.error('Failed to load feed:', e);
		} finally {
			loading = false;
		}
	}

	async function search() {
		if (!searchQuery.trim()) {
			loadFeed();
			return;
		}
		loading = true;
		try {
			feed = await api.searchIssues(searchQuery, 1);
		} catch (e) {
			console.error('Search failed:', e);
		} finally {
			loading = false;
		}
	}

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

<div class="feed-page">
	<header class="feed-header">
		<h1>your feed<span class="cursor">_</span></h1>
		<p class="subtitle">issues matched to your skills and goals</p>
	</header>

	<div class="search-bar">
		<input
			type="text"
			placeholder="search issues..."
			bind:value={searchQuery}
			onkeydown={(e) => e.key === 'Enter' && search()}
		/>
		<button onclick={search}>search</button>
	</div>

	{#if loading}
		<div class="loading">
			<div class="spinner"></div>
		</div>
	{:else if feed && feed.issues.length > 0}
		<div class="issue-list">
			{#each feed.issues as issue (issue.id)}
				<a href="/app/issues/{issue.id}" class="issue-card">
					<div class="issue-top">
						<span class="repo-name">{issue.repo?.owner}/{issue.repo?.name}</span>
						<span class="issue-number">#{issue.number}</span>
					</div>
					<h3 class="issue-title">{issue.title}</h3>
					<div class="issue-meta">
						<span class="difficulty" style="color: {difficultyColor(issue.difficulty)}">
							{difficultyLabel(issue.difficulty)}
						</span>
						<span class="time">{issue.time_estimate}</span>
						<span class="comments">{issue.comment_count} comments</span>
						{#if issue.match_score}
							<span class="score">{Math.round(issue.match_score * 100)}% match</span>
						{/if}
					</div>
					<div class="issue-labels">
						{#each issue.labels?.slice(0, 4) ?? [] as label}
							<span class="label">{label}</span>
						{/each}
					</div>
					{#if issue.match_reasons?.length}
						<div class="match-reasons">
							{#each issue.match_reasons as reason}
								<span class="reason">{reason}</span>
							{/each}
						</div>
					{/if}
				</a>
			{/each}
		</div>

		{#if feed.total_count > feed.per_page}
			<div class="pagination">
				<button disabled={page <= 1} onclick={() => { page--; loadFeed(); }}>prev</button>
				<span class="page-info">page {feed.page} of {Math.ceil(feed.total_count / feed.per_page)}</span>
				<button disabled={page * feed.per_page >= feed.total_count} onclick={() => { page++; loadFeed(); }}>next</button>
			</div>
		{/if}
	{:else}
		<div class="empty">
			<p>no issues found. try adjusting your skills or check back later.</p>
		</div>
	{/if}
</div>

<style>
	.feed-page { max-width: 800px; }

	.feed-header { margin-bottom: 2rem; }

	.feed-header h1 {
		font-size: 1.5rem;
		color: var(--text-bright);
		font-weight: 600;
	}

	.cursor { animation: blink 1s step-end infinite; color: var(--amber); }
	@keyframes blink { 50% { opacity: 0; } }

	.subtitle {
		color: var(--text-muted);
		font-size: 0.85rem;
		margin-top: 0.25rem;
	}

	.search-bar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.search-bar input {
		flex: 1;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		outline: none;
		transition: border-color 0.15s;
	}

	.search-bar input:focus { border-color: var(--amber); }

	.search-bar button {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 0.5rem 1rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.search-bar button:hover { background: var(--amber-glow); }

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

	.issue-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.issue-card {
		display: block;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem 1.25rem;
		text-decoration: none;
		transition: all 0.15s;
	}

	.issue-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
	}

	.issue-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.repo-name {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.issue-number {
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.issue-title {
		font-size: 0.95rem;
		font-weight: 500;
		color: var(--text-bright);
		margin-bottom: 0.5rem;
		line-height: 1.4;
	}

	.issue-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.score {
		color: var(--amber);
		font-weight: 600;
	}

	.issue-labels {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.label {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		background: var(--amber-glow);
		color: var(--amber);
		border-radius: 3px;
	}

	.match-reasons {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.5rem;
	}

	.reason {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		background: rgba(74, 222, 128, 0.1);
		color: var(--green);
		border-radius: 3px;
	}

	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		margin-top: 2rem;
	}

	.pagination button {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.375rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
	}

	.pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.page-info {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.empty {
		text-align: center;
		padding: 3rem;
		color: var(--text-muted);
	}
</style>
