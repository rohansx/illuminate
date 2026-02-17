<script lang="ts">
	import { api, type AdminRepoList, type AdminRepoListItem, type Category } from '$lib/api';
	import { onMount } from 'svelte';

	let data = $state<AdminRepoList | null>(null);
	let categories = $state<Category[]>([]);
	let loading = $state(true);
	let page = $state(1);
	let seeding = $state(false);
	let indexing = $state(false);
	let deleting = $state<string | null>(null);
	let editing = $state<string | null>(null);

	// Edit state
	let editTags = $state('');
	let editDifficulty = $state('intermediate');
	let editActivity = $state('active');

	onMount(() => {
		loadRepos();
		loadCategories();
	});

	async function loadRepos() {
		loading = true;
		try {
			data = await api.adminListRepos(page);
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function loadCategories() {
		try {
			categories = await api.adminGetCategories();
		} catch (e) {
			console.error(e);
		}
	}

	async function triggerSeed() {
		seeding = true;
		try {
			await api.adminTriggerSeed();
			alert('Seed job started. Check jobs page for progress.');
		} catch (e: any) {
			alert(e.message);
		} finally {
			seeding = false;
		}
	}

	async function triggerIndex() {
		indexing = true;
		try {
			await api.adminTriggerIndex();
			alert('Index job started. Check jobs page for progress.');
		} catch (e: any) {
			alert(e.message);
		} finally {
			indexing = false;
		}
	}

	async function deleteRepo(repoId: string, repoName: string) {
		if (!confirm(`Delete ${repoName} and all its issues?`)) return;
		deleting = repoId;
		try {
			await api.adminDeleteRepo(repoId);
			await loadRepos();
		} catch (e: any) {
			alert(e.message);
		} finally {
			deleting = null;
		}
	}

	function startEdit(repo: AdminRepoListItem) {
		editing = repo.id;
		editTags = (repo.tags || []).join(', ');
		editDifficulty = repo.difficulty_level || 'intermediate';
		editActivity = repo.activity_status || 'active';
	}

	function cancelEdit() {
		editing = null;
	}

	async function saveMetadata(repo: AdminRepoListItem) {
		try {
			const tags = editTags.split(',').map(t => t.trim()).filter(Boolean);
			await api.adminUpdateRepoMetadata(repo.id, {
				tags,
				difficulty_level: editDifficulty,
				activity_status: editActivity
			});
			editing = null;
			await loadRepos();
		} catch (e: any) {
			alert(e.message);
		}
	}

	async function toggleCategory(repo: AdminRepoListItem, category: Category) {
		const hasCategory = repo.categories?.some(c => c.id === category.id);
		try {
			if (hasCategory) {
				await api.adminRemoveCategory(repo.id, category.id);
			} else {
				await api.adminAssignCategory(repo.id, category.id);
			}
			await loadRepos();
		} catch (e: any) {
			alert(e.message);
		}
	}
</script>

<div class="repos-page">
	<header>
		<a href="/app/admin" class="back">&larr; admin</a>
		<h1>repositories<span class="cursor">_</span></h1>
		{#if data}
			<p class="subtitle">{data.total_count} indexed repositories</p>
		{/if}
		<div class="header-actions">
			<button class="action-btn" onclick={triggerSeed} disabled={seeding}>
				{seeding ? 'seeding...' : 'seed repos'}
			</button>
			<button class="action-btn" onclick={triggerIndex} disabled={indexing}>
				{indexing ? 'indexing...' : 'index issues'}
			</button>
		</div>
	</header>

	{#if loading}
		<div class="loading"><div class="spinner"></div></div>
	{:else if data?.repos?.length}
		<div class="repo-list">
			{#each data.repos as repo (repo.id)}
				<div class="repo-card" class:expanded={editing === repo.id}>
					<div class="repo-header">
						<div class="repo-info">
							<span class="repo-name">{repo.owner}/{repo.name}</span>
							<div class="repo-meta">
								{#if repo.primary_language}
									<span class="lang">{repo.primary_language}</span>
								{/if}
								<span class="stars">{repo.stars.toLocaleString()} ★</span>
								<span class="issue-count">{repo.issue_count} issues</span>
							</div>
						</div>
						<div class="repo-actions">
							{#if editing === repo.id}
								<button class="action-btn save" onclick={() => saveMetadata(repo)}>save</button>
								<button class="action-btn cancel" onclick={cancelEdit}>cancel</button>
							{:else}
								<button class="action-btn edit" onclick={() => startEdit(repo)}>edit</button>
								<button
									class="action-btn delete"
									onclick={() => deleteRepo(repo.id, `${repo.owner}/${repo.name}`)}
									disabled={deleting === repo.id}
								>
									{deleting === repo.id ? '...' : 'delete'}
								</button>
							{/if}
						</div>
					</div>

					{#if editing === repo.id}
						<div class="edit-section">
							<div class="field">
								<label>tags (comma separated)</label>
								<input type="text" bind:value={editTags} placeholder="beginner-friendly, web, mobile" />
							</div>
							<div class="field">
								<label>difficulty</label>
								<select bind:value={editDifficulty}>
									<option value="beginner">beginner</option>
									<option value="intermediate">intermediate</option>
									<option value="advanced">advanced</option>
								</select>
							</div>
							<div class="field">
								<label>activity</label>
								<select bind:value={editActivity}>
									<option value="very-active">very active</option>
									<option value="active">active</option>
									<option value="maintained">maintained</option>
									<option value="slow">slow</option>
								</select>
							</div>
							<div class="field">
								<label>categories</label>
								<div class="category-grid">
									{#each categories as category}
										<button
											class="category-pill"
											class:active={repo.categories?.some(c => c.id === category.id)}
											onclick={() => toggleCategory(repo, category)}
										>
											{category.name}
										</button>
									{/each}
								</div>
							</div>
						</div>
					{:else}
						<div class="taxonomy">
							{#if repo.tags?.length}
								<div class="tag-list">
									{#each repo.tags as tag}
										<span class="tag">{tag}</span>
									{/each}
								</div>
							{/if}
							<div class="badges">
								<span class="badge difficulty">{repo.difficulty_level || 'intermediate'}</span>
								<span class="badge activity">{repo.activity_status || 'active'}</span>
							</div>
							{#if repo.categories?.length}
								<div class="category-list">
									{#each repo.categories as cat}
										<span class="category">{cat.name}</span>
									{/each}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		{#if data.total_count > data.per_page}
			<div class="pagination">
				<button disabled={page <= 1} onclick={() => { page--; loadRepos(); }}>prev</button>
				<span class="page-info">page {data.page} of {Math.ceil(data.total_count / data.per_page)}</span>
				<button disabled={page * data.per_page >= data.total_count} onclick={() => { page++; loadRepos(); }}>next</button>
			</div>
		{/if}
	{:else}
		<div class="empty"><p>no repositories indexed yet. run a seed job to get started.</p></div>
	{/if}
</div>

<style>
	.repos-page { max-width: 900px; }

	header { margin-bottom: 1.5rem; }

	.back {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-decoration: none;
		margin-bottom: 0.5rem;
		display: inline-block;
	}

	.back:hover { color: var(--amber); }

	h1 {
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

	.header-actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.action-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.375rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.action-btn.edit { color: var(--amber); }
	.action-btn.save { color: var(--green); }
	.action-btn.cancel { color: var(--text-muted); }
	.action-btn.delete { color: var(--text-muted); }
	.action-btn.delete:hover:not(:disabled) { color: var(--red); border-color: var(--red); }

	.action-btn:hover:not(:disabled) { background: var(--amber-glow); }
	.action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

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

	.repo-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.repo-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem;
		transition: all 0.2s;
	}

	.repo-card.expanded {
		border-color: var(--amber-dim);
		background: var(--bg-card);
	}

	.repo-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.repo-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.repo-name {
		font-size: 0.95rem;
		color: var(--text-bright);
		font-weight: 600;
	}

	.repo-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.lang {
		color: var(--amber);
		font-weight: 500;
	}

	.repo-actions {
		display: flex;
		gap: 0.5rem;
	}

	.taxonomy {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.tag-list, .category-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.tag {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--text-muted);
	}

	.category {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		background: var(--amber-glow);
		border: 1px solid var(--amber-dim);
		border-radius: 3px;
		color: var(--amber);
	}

	.badges {
		display: flex;
		gap: 0.5rem;
	}

	.badge {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		border-radius: 3px;
		font-weight: 500;
	}

	.badge.difficulty {
		background: rgba(139, 92, 246, 0.1);
		color: rgb(196, 181, 253);
		border: 1px solid rgba(139, 92, 246, 0.3);
	}

	.badge.activity {
		background: rgba(34, 211, 238, 0.1);
		color: rgb(103, 232, 249);
		border: 1px solid rgba(34, 211, 238, 0.3);
	}

	.edit-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.field label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.field input, .field select {
		background: var(--bg);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 0.5rem;
		border-radius: 4px;
	}

	.field input:focus, .field select:focus {
		outline: none;
		border-color: var(--amber);
	}

	.category-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.category-pill {
		font-size: 0.7rem;
		padding: 0.375rem 0.75rem;
		background: var(--bg);
		border: 1px solid var(--border);
		color: var(--text-muted);
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.category-pill:hover {
		border-color: var(--amber-dim);
		color: var(--text);
	}

	.category-pill.active {
		background: var(--amber-glow);
		border-color: var(--amber);
		color: var(--amber);
	}

	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		margin-top: 1.5rem;
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

	.pagination button:disabled { opacity: 0.3; cursor: not-allowed; }

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
