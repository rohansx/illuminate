<script lang="ts">
	import { categories, allLanguages, allTags, type AwesomeRepo } from '$lib/data/awesome';
	import { api, type StarredRepo } from '$lib/api';

	type Tab = 'curated' | 'awesome-lists' | 'my-stars';

	let activeTab = $state<Tab>('curated');
	let selectedCategory = $state(categories[0]?.id ?? '');
	let searchQuery = $state('');
	let selectedLanguage = $state('');
	let selectedTag = $state('');

	// Starred repos state
	let starredRepos = $state<StarredRepo[]>([]);
	let starredLoading = $state(false);
	let starredLoaded = $state(false);
	let starredError = $state('');

	const activeCategory = $derived(
		categories.find(c => c.id === selectedCategory) ?? categories[0]
	);

	// Awesome-lists: repos with "awesome" in the name across all categories
	const awesomeListRepos = $derived.by(() => {
		const seen = new Set<string>();
		const result: AwesomeRepo[] = [];
		for (const cat of categories) {
			for (const repo of cat.repos) {
				const key = `${repo.owner}/${repo.name}`;
				if (!seen.has(key) && repo.name.toLowerCase().includes('awesome')) {
					seen.add(key);
					result.push(repo);
				}
			}
		}
		return result;
	});

	const filteredRepos = $derived.by(() => {
		let repos = activeCategory?.repos ?? [];
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			repos = repos.filter(r =>
				r.name.toLowerCase().includes(q) ||
				r.owner.toLowerCase().includes(q) ||
				r.description.toLowerCase().includes(q)
			);
		}
		if (selectedLanguage) {
			repos = repos.filter(r => r.language === selectedLanguage);
		}
		if (selectedTag) {
			repos = repos.filter(r => r.tags.includes(selectedTag));
		}
		return repos;
	});

	const filteredAwesomeLists = $derived.by(() => {
		let repos = awesomeListRepos;
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			repos = repos.filter(r =>
				r.name.toLowerCase().includes(q) ||
				r.owner.toLowerCase().includes(q) ||
				r.description.toLowerCase().includes(q)
			);
		}
		return repos;
	});

	const filteredStarred = $derived.by(() => {
		let repos = starredRepos;
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			repos = repos.filter(r =>
				r.name.toLowerCase().includes(q) ||
				r.owner.toLowerCase().includes(q) ||
				(r.description || '').toLowerCase().includes(q)
			);
		}
		if (selectedLanguage) {
			repos = repos.filter(r => r.language === selectedLanguage);
		}
		return repos;
	});

	const hasFilters = $derived(searchQuery.trim() !== '' || selectedLanguage !== '' || selectedTag !== '');

	function clearFilters() {
		searchQuery = '';
		selectedLanguage = '';
		selectedTag = '';
	}

	function formatStars(s: string | number): string {
		if (typeof s === 'number') {
			if (s >= 1000) return (s / 1000).toFixed(s >= 10000 ? 0 : 1) + 'k';
			return String(s);
		}
		return s;
	}

	function switchTab(tab: Tab) {
		activeTab = tab;
		if (tab === 'my-stars' && !starredLoaded && !starredLoading) {
			loadStarred();
		}
	}

	async function loadStarred() {
		starredLoading = true;
		starredError = '';
		try {
			const res = await api.getStarredRepos(1, 100);
			starredRepos = res.repos ?? [];
			starredLoaded = true;
		} catch (e: any) {
			starredError = e.message || 'failed to load starred repos';
		} finally {
			starredLoading = false;
		}
	}
</script>

<div class="awesome-page">
	<header class="header">
		<h1>awesome repos<span class="cursor">_</span></h1>
		<p class="subtitle">curated open-source projects by category</p>
	</header>

	<!-- Tab Bar -->
	<div class="tab-bar">
		<button class="tab" class:tab-active={activeTab === 'curated'} onclick={() => switchTab('curated')}>
			<span class="tab-icon">&lt;/&gt;</span> Curated
		</button>
		<button class="tab" class:tab-active={activeTab === 'awesome-lists'} onclick={() => switchTab('awesome-lists')}>
			<span class="tab-icon">&#9733;</span> Awesome Lists
			<span class="tab-badge">{awesomeListRepos.length}</span>
		</button>
		<button class="tab" class:tab-active={activeTab === 'my-stars'} onclick={() => switchTab('my-stars')}>
			<span class="tab-icon">&#9829;</span> My Stars
			{#if starredLoaded}
				<span class="tab-badge">{starredRepos.length}</span>
			{/if}
		</button>
	</div>

	<!-- Search + Filters -->
	<div class="controls">
		<div class="search-field">
			<span class="search-icon">&#8981;</span>
			<input
				type="text"
				placeholder="search repos..."
				bind:value={searchQuery}
			/>
			{#if searchQuery}
				<button class="search-clear" onclick={() => searchQuery = ''}>&times;</button>
			{/if}
		</div>
		{#if activeTab !== 'awesome-lists'}
			<select class="filter-select" bind:value={selectedLanguage}>
				<option value="">all languages</option>
				{#each allLanguages as lang}
					<option value={lang}>{lang}</option>
				{/each}
			</select>
		{/if}
		{#if activeTab === 'curated'}
			<select class="filter-select" bind:value={selectedTag}>
				<option value="">all tags</option>
				{#each allTags as tag}
					<option value={tag}>{tag}</option>
				{/each}
			</select>
		{/if}
		{#if hasFilters}
			<button class="clear-btn" onclick={clearFilters}>clear</button>
		{/if}
	</div>

	<!-- TAB: Curated -->
	{#if activeTab === 'curated'}
		<div class="layout">
			<nav class="sidebar">
				{#each categories as cat}
					<button
						class="cat-btn"
						class:cat-active={selectedCategory === cat.id}
						onclick={() => selectedCategory = cat.id}
					>
						<span class="cat-icon">{cat.icon}</span>
						<span class="cat-label">{cat.label}</span>
						<span class="cat-count">{cat.repos.length}</span>
					</button>
				{/each}
			</nav>

			<div class="main">
				<div class="section-header">
					<span class="section-icon">{activeCategory?.icon}</span>
					<span class="section-title">{activeCategory?.label}</span>
					<span class="section-count">{filteredRepos.length} repos</span>
				</div>

				{#if filteredRepos.length > 0}
					<div class="repo-grid">
						{#each filteredRepos as repo (repo.owner + '/' + repo.name)}
							<a
								href="https://github.com/{repo.owner}/{repo.name}"
								target="_blank"
								rel="noopener noreferrer"
								class="repo-card"
							>
								<div class="repo-top">
									<span class="repo-name">{repo.owner}<span class="repo-sep">/</span>{repo.name}</span>
									<span class="repo-stars">{formatStars(repo.stars)} &#9733;</span>
								</div>
								<p class="repo-desc">{repo.description}</p>
								<div class="repo-meta">
									<span class="repo-lang">
										<span class="lang-dot"></span>
										{repo.language}
									</span>
									{#each repo.tags as tag}
										<span class="repo-tag">{tag}</span>
									{/each}
								</div>
							</a>
						{/each}
					</div>
				{:else}
					<div class="empty-state">
						<p class="empty-title">no repos match your filters</p>
						<p class="empty-sub">try changing the category or clearing filters</p>
						{#if hasFilters}
							<button class="clear-btn" onclick={clearFilters}>clear filters</button>
						{/if}
					</div>
				{/if}
			</div>
		</div>

	<!-- TAB: Awesome Lists -->
	{:else if activeTab === 'awesome-lists'}
		<div class="main">
			<div class="section-header">
				<span class="section-icon">&#9733;</span>
				<span class="section-title">Awesome Lists</span>
				<span class="section-count">{filteredAwesomeLists.length} repos</span>
			</div>

			{#if filteredAwesomeLists.length > 0}
				<div class="repo-grid">
					{#each filteredAwesomeLists as repo (repo.owner + '/' + repo.name)}
						<a
							href="https://github.com/{repo.owner}/{repo.name}"
							target="_blank"
							rel="noopener noreferrer"
							class="repo-card"
						>
							<div class="repo-top">
								<span class="repo-name">{repo.owner}<span class="repo-sep">/</span>{repo.name}</span>
								<span class="repo-stars">{formatStars(repo.stars)} &#9733;</span>
							</div>
							<p class="repo-desc">{repo.description}</p>
							<div class="repo-meta">
								<span class="repo-lang">
									<span class="lang-dot"></span>
									{repo.language}
								</span>
								{#each repo.tags as tag}
									<span class="repo-tag">{tag}</span>
								{/each}
							</div>
						</a>
					{/each}
				</div>
			{:else}
				<div class="empty-state">
					<p class="empty-title">no awesome lists found</p>
					<p class="empty-sub">try a different search term</p>
				</div>
			{/if}
		</div>

	<!-- TAB: My Stars -->
	{:else if activeTab === 'my-stars'}
		<div class="main">
			<div class="section-header">
				<span class="section-icon">&#9829;</span>
				<span class="section-title">My Starred Repos</span>
				{#if starredLoaded}
					<span class="section-count">{filteredStarred.length} repos</span>
				{/if}
			</div>

			{#if starredLoading}
				<div class="loading-state">
					<div class="spinner"></div>
					<p>loading your starred repos from github...</p>
				</div>
			{:else if starredError}
				<div class="empty-state">
					<p class="empty-title">failed to load starred repos</p>
					<p class="empty-sub">{starredError}</p>
					<button class="clear-btn" onclick={loadStarred}>retry</button>
				</div>
			{:else if filteredStarred.length > 0}
				<div class="repo-grid">
					{#each filteredStarred as repo (repo.owner + '/' + repo.name)}
						<a
							href={repo.html_url}
							target="_blank"
							rel="noopener noreferrer"
							class="repo-card"
						>
							<div class="repo-top">
								<span class="repo-name">{repo.owner}<span class="repo-sep">/</span>{repo.name}</span>
								<span class="repo-stars">{formatStars(repo.stargazers_count)} &#9733;</span>
							</div>
							<p class="repo-desc">{repo.description || 'No description'}</p>
							<div class="repo-meta">
								{#if repo.language}
									<span class="repo-lang">
										<span class="lang-dot"></span>
										{repo.language}
									</span>
								{/if}
								{#if repo.topics}
									{#each repo.topics.slice(0, 4) as topic}
										<span class="repo-tag">{topic}</span>
									{/each}
								{/if}
							</div>
						</a>
					{/each}
				</div>
			{:else if starredLoaded}
				<div class="empty-state">
					<p class="empty-title">{searchQuery ? 'no starred repos match your search' : 'no starred repos yet'}</p>
					<p class="empty-sub">{searchQuery ? 'try a different search term' : 'star repos on github and they\'ll show up here'}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.awesome-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* ── Header ── */
	.header {
		margin-bottom: 1.5rem;
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

	/* ── Tab Bar ── */
	.tab-bar {
		display: flex;
		gap: 0.25rem;
		margin-bottom: 1.25rem;
		border-bottom: 1px solid var(--border);
		padding-bottom: 0;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.78rem;
		cursor: pointer;
		transition: all 0.15s;
		margin-bottom: -1px;
	}

	.tab:hover {
		color: var(--text);
	}

	.tab-active {
		color: var(--amber);
		border-bottom-color: var(--amber);
		font-weight: 500;
	}

	.tab-icon {
		font-size: 0.75rem;
	}

	.tab-badge {
		font-size: 0.62rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text-dim);
		padding: 0.05rem 0.35rem;
		border-radius: 8px;
	}

	.tab-active .tab-badge {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	/* ── Controls ── */
	.controls {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		align-items: center;
	}

	.search-field {
		flex: 1;
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 0.75rem;
		font-size: 0.9rem;
		color: var(--text-dim);
		pointer-events: none;
	}

	.search-field input {
		width: 100%;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.6rem 2rem 0.6rem 2.25rem;
		border-radius: 4px;
		outline: none;
		transition: border-color 0.15s;
	}

	.search-field input::placeholder { color: var(--text-dim); }
	.search-field input:focus { border-color: var(--amber-dim); }

	.search-clear {
		position: absolute;
		right: 0.5rem;
		background: none;
		border: none;
		color: var(--text-dim);
		font-size: 1.1rem;
		cursor: pointer;
		padding: 0.2rem 0.4rem;
		line-height: 1;
	}

	.search-clear:hover { color: var(--text); }

	.filter-select {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.55rem 0.75rem;
		border-radius: 4px;
		outline: none;
		cursor: pointer;
		transition: border-color 0.15s;
		min-width: 120px;
	}

	.filter-select:focus { border-color: var(--amber-dim); }

	.clear-btn {
		background: none;
		border: 1px solid var(--border);
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.55rem 0.75rem;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.clear-btn:hover {
		border-color: var(--red);
		color: var(--red);
	}

	/* ── Layout ── */
	.layout {
		display: grid;
		grid-template-columns: 200px 1fr;
		gap: 1.5rem;
	}

	@media (max-width: 768px) {
		.layout {
			grid-template-columns: 1fr;
		}

		.sidebar {
			display: flex;
			flex-wrap: wrap;
			gap: 0.3rem;
		}

		.controls {
			flex-wrap: wrap;
		}

		.filter-select {
			min-width: 0;
			flex: 1;
		}
	}

	/* ── Sidebar ── */
	.sidebar {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.cat-btn {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.45rem 0.65rem;
		background: none;
		border: 1px solid transparent;
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
		text-align: left;
	}

	.cat-btn:hover {
		background: var(--bg-raised);
		color: var(--text);
	}

	.cat-active {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		color: var(--amber);
		font-weight: 500;
	}

	.cat-icon {
		font-weight: 700;
		font-size: 0.7rem;
		width: 20px;
		text-align: center;
		flex-shrink: 0;
	}

	.cat-label {
		flex: 1;
		white-space: nowrap;
	}

	.cat-count {
		font-size: 0.65rem;
		color: var(--text-dim);
		flex-shrink: 0;
	}

	.cat-active .cat-count {
		color: var(--amber);
	}

	/* ── Main ── */
	.section-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	.section-icon {
		font-weight: 700;
		font-size: 0.8rem;
		color: var(--amber);
	}

	.section-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.section-count {
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-left: auto;
	}

	/* ── Repo Grid ── */
	.repo-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 900px) {
		.repo-grid { grid-template-columns: 1fr; }
	}

	.repo-card {
		display: flex;
		flex-direction: column;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem 1.15rem;
		text-decoration: none;
		transition: all 0.2s;
		animation: cardIn 0.3s ease both;
	}

	@keyframes cardIn {
		from { opacity: 0; transform: translateY(6px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.repo-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
		transform: translateY(-1px);
	}

	/* ── Repo Card Contents ── */
	.repo-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.45rem;
	}

	.repo-name {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-bright);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.repo-card:hover .repo-name {
		color: var(--amber);
	}

	.repo-sep {
		color: var(--text-dim);
		margin: 0 0.05rem;
	}

	.repo-stars {
		font-size: 0.7rem;
		color: var(--amber);
		font-weight: 600;
		flex-shrink: 0;
		margin-left: 0.5rem;
	}

	.repo-desc {
		font-size: 0.72rem;
		color: var(--text-muted);
		line-height: 1.55;
		margin-bottom: 0.6rem;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.repo-meta {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin-top: auto;
	}

	.repo-lang {
		font-size: 0.68rem;
		color: var(--text-muted);
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

	.repo-tag {
		font-size: 0.6rem;
		padding: 0.1rem 0.4rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-dim);
		border-radius: 3px;
	}

	/* ── Loading ── */
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 4rem 2rem;
		color: var(--text-muted);
		font-size: 0.8rem;
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

	/* ── Empty ── */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 4rem 2rem;
		text-align: center;
	}

	.empty-title {
		font-size: 0.9rem;
		color: var(--text-muted);
	}

	.empty-sub {
		font-size: 0.75rem;
		color: var(--text-dim);
	}
</style>
