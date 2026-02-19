<script lang="ts">
	import { api, type Repository, type HiringRepoList } from '$lib/api';
	import { onMount } from 'svelte';

	let data = $state<HiringRepoList | null>(null);
	let loading = $state(true);
	let page = $state(1);

	onMount(() => loadHiring());

	async function loadHiring() {
		loading = true;
		try {
			data = await api.getHiringRepos(page, 20);
		} catch (e) {
			console.error('Failed to load hiring repos:', e);
		} finally {
			loading = false;
		}
	}

	const totalPages = $derived(data ? Math.ceil(data.total_count / 20) : 0);
</script>

<div class="hiring-page">
	<header class="header">
		<h1>open source hiring<span class="cursor">_</span></h1>
		<p class="subtitle">projects that are actively looking for contributors and team members</p>
		{#if data && !loading}
			<div class="header-stats">
				<span class="stat-num">{data.total_count}</span>
				<span class="stat-label">hiring repos</span>
			</div>
		{/if}
	</header>

	{#if loading}
		<div class="loading-grid">
			{#each Array(6) as _, i}
				<div class="skeleton-card" style="animation-delay: {i * 80}ms"></div>
			{/each}
		</div>
	{:else if data?.repos?.length}
		<div class="repo-grid">
			{#each data.repos as repo (repo.id)}
				<div class="card">
					<div class="card-top">
						<a
							href="https://github.com/{repo.owner}/{repo.name}"
							target="_blank"
							rel="noopener noreferrer"
							class="card-repo"
						>{repo.owner}/{repo.name}</a>
						<span class="hiring-badge">hiring</span>
					</div>

					<p class="card-desc">{repo.description || 'No description'}</p>

					<div class="card-meta">
						{#if repo.primary_language}
							<span class="meta-lang">
								<span class="lang-dot"></span>
								{repo.primary_language}
							</span>
						{/if}
						<span class="meta-stars">{repo.stars.toLocaleString()} &#9733;</span>
						{#if repo.health_score}
							<span class="meta-health" title="health: {Math.round(repo.health_score * 100)}%">
								{repo.health_score >= 0.8 ? '++' : repo.health_score >= 0.6 ? '+' : '~'}
							</span>
						{/if}
					</div>

					{#if repo.topics?.length}
						<div class="card-topics">
							{#each repo.topics.slice(0, 6) as topic}
								<span class="topic">{topic}</span>
							{/each}
						</div>
					{/if}

					<div class="card-actions">
						<a
							href={repo.hiring_url || `https://github.com/${repo.owner}/${repo.name}`}
							target="_blank"
							rel="noopener noreferrer"
							class="hiring-link"
						>view hiring info &rarr;</a>
						<a
							href="https://github.com/{repo.owner}/{repo.name}/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22"
							target="_blank"
							rel="noopener noreferrer"
							class="issues-link"
						>open issues</a>
					</div>
				</div>
			{/each}
		</div>

		{#if totalPages > 1}
			<nav class="pagination">
				<button
					class="page-btn"
					disabled={page <= 1}
					onclick={() => { page--; loadHiring(); }}
				>&larr; prev</button>
				<span class="page-info">{page} / {totalPages}</span>
				<button
					class="page-btn"
					disabled={page >= totalPages}
					onclick={() => { page++; loadHiring(); }}
				>next &rarr;</button>
			</nav>
		{/if}
	{:else}
		<div class="empty-state">
			<div class="empty-icon">&#9675;</div>
			<p class="empty-title">no hiring repos found</p>
			<p class="empty-sub">repos with hiring signals in their topics, description, or HIRING.md will appear here</p>
		</div>
	{/if}
</div>

<style>
	.hiring-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	.header {
		margin-bottom: 1.75rem;
	}

	.header h1 {
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
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 20px;
		padding: 0.3rem 0.75rem;
		font-size: 0.7rem;
		margin-top: 0.75rem;
	}

	.stat-num { color: var(--amber); font-weight: 700; }
	.stat-label { color: var(--text-dim); }

	/* Loading */
	.loading-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	.skeleton-card {
		height: 200px;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 0.8; }
	}

	/* Grid */
	.repo-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 700px) {
		.repo-grid, .loading-grid { grid-template-columns: 1fr; }
	}

	/* Card */
	.card {
		display: flex;
		flex-direction: column;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.15rem 1.25rem;
		transition: all 0.2s;
	}

	.card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
	}

	.card-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.card-repo {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-bright);
		text-decoration: none;
		transition: color 0.15s;
	}

	.card-repo:hover { color: var(--amber); }

	.hiring-badge {
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		padding: 0.15rem 0.5rem;
		background: rgba(74, 222, 128, 0.12);
		color: #4ade80;
		border: 1px solid rgba(74, 222, 128, 0.3);
		border-radius: 3px;
	}

	.card-desc {
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.55;
		margin-bottom: 0.6rem;
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

	.meta-lang {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}

	.lang-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--amber);
	}

	.meta-stars { color: var(--amber); font-weight: 600; }

	.meta-health {
		font-weight: 700;
		font-family: var(--font-mono);
		color: var(--green);
	}

	.card-topics {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin-bottom: 0.75rem;
	}

	.topic {
		font-size: 0.6rem;
		padding: 0.1rem 0.4rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-dim);
		border-radius: 3px;
	}

	.card-actions {
		margin-top: auto;
		display: flex;
		gap: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.hiring-link {
		font-size: 0.72rem;
		color: #4ade80;
		text-decoration: none;
		font-weight: 500;
		transition: opacity 0.15s;
	}

	.hiring-link:hover { opacity: 0.8; }

	.issues-link {
		font-size: 0.72rem;
		color: var(--text-dim);
		text-decoration: none;
		transition: color 0.15s;
	}

	.issues-link:hover { color: var(--text); }

	/* Pagination */
	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
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

	.page-info {
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	/* Empty */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 4rem 2rem;
		text-align: center;
	}

	.empty-icon { font-size: 2rem; color: var(--text-dim); }
	.empty-title { font-size: 0.9rem; color: var(--text-muted); font-weight: 500; }
	.empty-sub { font-size: 0.75rem; color: var(--text-dim); max-width: 400px; }
</style>
