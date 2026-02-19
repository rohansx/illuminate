<script lang="ts">
	import { api, type Issue, type IssueFeed } from '$lib/api';
	import { onMount } from 'svelte';

	let feed = $state<IssueFeed | null>(null);
	let loading = $state(true);
	let page = $state(1);

	onMount(() => { loadSaved(); });

	async function loadSaved() {
		loading = true;
		try {
			feed = await api.getSavedIssues(page, 20);
		} catch (e) {
			console.error('Failed to load saved issues:', e);
		} finally {
			loading = false;
		}
	}

	async function unsave(issueId: string) {
		try {
			await api.unsaveIssue(issueId);
			// Remove from local list
			if (feed) {
				feed = {
					...feed,
					issues: feed.issues.filter(i => i.id !== issueId),
					total_count: feed.total_count - 1
				};
			}
		} catch (e) {
			console.error('Failed to unsave:', e);
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

	function getLabelColor(label: string): { bg: string; fg: string } {
		const l = label.toLowerCase();
		if (l.includes('good first issue') || l.includes('beginner') || l.includes('easy')) return { bg: 'rgba(74, 222, 128, 0.12)', fg: '#4ade80' };
		if (l.includes('help wanted') || l.includes('contributions')) return { bg: 'rgba(103, 232, 249, 0.12)', fg: '#67e8f9' };
		if (l.includes('bug') || l.includes('fix')) return { bg: 'rgba(248, 113, 113, 0.12)', fg: '#f87171' };
		if (l.includes('type:') || l.includes('documentation') || l.includes('docs') || l.includes('cleanup')) return { bg: 'rgba(96, 165, 250, 0.12)', fg: '#60a5fa' };
		if (l.includes('team-') || l.includes('team:')) return { bg: 'rgba(192, 132, 252, 0.12)', fg: '#c084fc' };
		if (l.startsWith('p0') || l.startsWith('p1')) return { bg: 'rgba(248, 113, 113, 0.12)', fg: '#f87171' };
		if (l.startsWith('p2') || l.startsWith('p3')) return { bg: 'rgba(251, 191, 36, 0.12)', fg: '#fbbf24' };
		if (l.includes('feature') || l.includes('enhancement')) return { bg: 'rgba(74, 222, 128, 0.12)', fg: '#4ade80' };
		return { bg: 'var(--amber-glow)', fg: 'var(--amber)' };
	}

	const totalPages = $derived(feed ? Math.ceil(feed.total_count / feed.per_page) : 0);
</script>

<div class="saved-page">
	<header class="header">
		<div class="header-text">
			<h1>saved issues<span class="cursor">_</span></h1>
			<p class="subtitle">issues you've bookmarked for later</p>
		</div>
		{#if feed && !loading}
			<div class="header-stats">
				<div class="stat-pill">
					<span class="stat-num">{feed.total_count}</span>
					<span class="stat-label">saved</span>
				</div>
			</div>
		{/if}
	</header>

	{#if loading}
		<div class="loading-grid">
			{#each Array(4) as _, i}
				<div class="skeleton-card" style="animation-delay: {i * 80}ms"></div>
			{/each}
		</div>
	{:else if feed?.issues?.length}
		<div class="issue-grid">
			{#each feed.issues as issue, i (issue.id)}
				<div class="card-wrapper" style="animation-delay: {i * 40}ms">
					<a href="/app/issues/{issue.id}" class="card">
						<div class="card-top">
							<span class="card-repo">{issue.repo?.owner}/{issue.repo?.name}</span>
							<span class="card-num">#{issue.number}</span>
						</div>

						<h3 class="card-title">{issue.title}</h3>

						{#if issue.summary}
							<p class="card-summary">{issue.summary}</p>
						{/if}

						<div class="card-meta">
							<span class="meta-difficulty" style="color: {difficultyColor(issue.difficulty)}">
								<span class="meta-dot" style="background: {difficultyColor(issue.difficulty)}"></span>
								{difficultyLabel(issue.difficulty)}
							</span>
							<span class="meta-item"><span class="meta-dot" style="background: #60a5fa"></span>{issue.time_estimate}</span>
							<span class="meta-item"><span class="meta-dot" style="background: #c084fc"></span>{issue.comment_count} comments</span>
						</div>

						{#if issue.labels?.length}
							<div class="card-labels">
								{#each issue.labels.slice(0, 5) as label}
									<span class="label" style="background: {getLabelColor(label).bg}; color: {getLabelColor(label).fg}">{label}</span>
								{/each}
								{#if issue.labels.length > 5}
									<span class="label label-more">+{issue.labels.length - 5}</span>
								{/if}
							</div>
						{/if}
					</a>
					<button
						class="unsave-btn"
						onclick={() => unsave(issue.id)}
						title="remove from saved"
					>
						&#9733;
					</button>
				</div>
			{/each}
		</div>

		{#if totalPages > 1}
			<nav class="pagination">
				<button
					class="page-btn"
					disabled={page <= 1}
					onclick={() => { page--; loadSaved(); }}
				>
					&larr; prev
				</button>

				<div class="page-dots">
					{#each Array(Math.min(totalPages, 7)) as _, i}
						{@const p = totalPages <= 7 ? i + 1 : (page <= 4 ? i + 1 : (page >= totalPages - 3 ? totalPages - 6 + i : page - 3 + i))}
						<button
							class="page-dot"
							class:page-dot-active={p === page}
							onclick={() => { page = p; loadSaved(); }}
						>
							{p}
						</button>
					{/each}
				</div>

				<button
					class="page-btn"
					disabled={page >= totalPages}
					onclick={() => { page++; loadSaved(); }}
				>
					next &rarr;
				</button>
			</nav>
		{/if}
	{:else}
		<div class="empty-state">
			<div class="empty-icon">&#9734;</div>
			<p class="empty-title">no saved issues yet</p>
			<p class="empty-sub">browse the feed and bookmark issues you're interested in</p>
			<a href="/app/feed" class="btn-ghost">&larr; go to feed</a>
		</div>
	{/if}
</div>

<style>
	.saved-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		margin-bottom: 1.75rem;
	}

	.header-text h1 {
		font-size: 1.4rem;
		color: var(--text-bright);
		font-weight: 600;
		line-height: 1;
	}

	.cursor { animation: blink 1s step-end infinite; color: var(--amber); }
	@keyframes blink { 50% { opacity: 0; } }

	.subtitle {
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 0.35rem;
	}

	.header-stats {
		display: flex;
		gap: 0.5rem;
	}

	.stat-pill {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 20px;
		padding: 0.3rem 0.75rem;
		font-size: 0.7rem;
	}

	.stat-num { color: var(--amber); font-weight: 700; }
	.stat-label { color: var(--text-dim); }

	/* Loading */
	.loading-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 700px) { .loading-grid { grid-template-columns: 1fr; } }

	.skeleton-card {
		height: 180px;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.8; } }

	/* Grid */
	.issue-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 700px) { .issue-grid { grid-template-columns: 1fr; } }

	/* Card wrapper */
	.card-wrapper {
		position: relative;
		animation: cardIn 0.35s ease both;
	}

	@keyframes cardIn {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.card {
		display: flex;
		flex-direction: column;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.15rem 1.25rem;
		text-decoration: none;
		transition: all 0.2s;
	}

	.card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
		transform: translateY(-1px);
	}

	.card:hover .card-title { color: var(--amber); }

	.card-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.card-repo {
		font-size: 0.7rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.card-num {
		font-size: 0.7rem;
		color: var(--text-dim);
		flex-shrink: 0;
		margin-left: 0.5rem;
	}

	.card-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
		line-height: 1.45;
		margin-bottom: 0.4rem;
		transition: color 0.15s;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.card-summary {
		font-size: 0.72rem;
		color: var(--text-muted);
		line-height: 1.6;
		margin-bottom: 0.5rem;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.card-meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.7rem;
		color: var(--text-muted);
		margin-bottom: 0.6rem;
	}

	.meta-difficulty {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-weight: 500;
	}

	.meta-dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.meta-item {
		white-space: nowrap;
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}

	.card-labels {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.label {
		font-size: 0.65rem;
		padding: 0.125rem 0.45rem;
		border-radius: 3px;
		white-space: nowrap;
	}

	.label-more {
		background: var(--bg-card);
		color: var(--text-dim);
		border: 1px solid var(--border);
	}

	/* Unsave button */
	.unsave-btn {
		position: absolute;
		top: 0.6rem;
		right: 0.6rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-size: 0.9rem;
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		z-index: 2;
	}

	.unsave-btn:hover {
		border-color: var(--red);
		color: var(--red);
		background: rgba(248, 113, 113, 0.08);
	}

	/* Pagination */
	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 0.5rem;
		margin-top: 2rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border);
	}

	.page-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.35rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.page-btn:hover:not(:disabled) {
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	.page-btn:disabled { opacity: 0.25; cursor: not-allowed; }

	.page-dots { display: flex; gap: 0.25rem; }

	.page-dot {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		border: 1px solid transparent;
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.page-dot:hover {
		color: var(--text);
		border-color: var(--border);
	}

	.page-dot-active {
		color: var(--amber);
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		font-weight: 600;
	}

	/* Empty state */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 4rem 2rem;
		text-align: center;
	}

	.empty-icon {
		font-size: 2.5rem;
		color: var(--text-dim);
		margin-bottom: 0.25rem;
	}

	.empty-title {
		font-size: 0.9rem;
		color: var(--text-muted);
		font-weight: 500;
	}

	.empty-sub {
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.btn-ghost {
		margin-top: 0.5rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.35rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
		text-decoration: none;
	}

	.btn-ghost:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
	}
</style>
