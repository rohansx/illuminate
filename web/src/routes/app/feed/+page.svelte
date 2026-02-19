<script lang="ts">
	import { api, type Issue, type IssueFeed, type Category, type User } from '$lib/api';
	import { onMount } from 'svelte';

	let feed = $state<IssueFeed | null>(null);
	let categories = $state<Category[]>([]);
	let loading = $state(true);
	let searchQuery = $state('');
	let page = $state(1);
	let isSearching = $state(false);

	// Import repo
	let showImport = $state(false);
	let importUrl = $state('');
	let importing = $state(false);
	let importResult = $state<string | null>(null);
	let importError = $state('');

	async function doImport() {
		if (!importUrl.trim()) return;
		importing = true;
		importResult = null;
		importError = '';
		try {
			const res = await api.importRepo(importUrl.trim());
			importResult = res.repo;
			importUrl = '';
		} catch (e: any) {
			importError = e.message || 'failed to import';
		} finally {
			importing = false;
		}
	}

	// Filters
	let selectedCategory = $state('');
	let selectedDifficulty = $state(0);
	let languageFilters = $state<Record<string, boolean>>({});

	const allLanguagesSelected = $derived(
		Object.keys(languageFilters).length === 0 || Object.values(languageFilters).every(v => v)
	);
	const activeLanguages = $derived(
		Object.entries(languageFilters).filter(([, active]) => active).map(([lang]) => lang)
	);
	const hasActiveFilters = $derived(selectedCategory !== '' || selectedDifficulty !== 0 || !allLanguagesSelected);

	onMount(async () => {
		const [, cats, me] = await Promise.all([
			loadFeed(),
			api.getCategories().catch(() => [] as Category[]),
			api.getMe().catch(() => null as User | null)
		]);
		categories = cats;
		if (me?.skills?.length) {
			const filters: Record<string, boolean> = {};
			for (const skill of me.skills) {
				filters[skill.language] = true;
			}
			languageFilters = filters;
		}
	});

	async function loadFeed() {
		loading = true;
		isSearching = false;
		try {
			const filters: { difficulty?: number; category?: string; languages?: string[] } = {
				difficulty: selectedDifficulty || undefined,
				category: selectedCategory || undefined
			};
			if (!allLanguagesSelected && activeLanguages.length > 0) {
				filters.languages = activeLanguages;
			}
			feed = await api.getFeed(page, 20, filters);
		} catch (e) {
			console.error('Failed to load feed:', e);
		} finally {
			loading = false;
		}
	}

	async function toggleSave(issueId: string) {
		if (!feed) return;
		const issue = feed.issues.find(i => i.id === issueId);
		if (!issue) return;
		try {
			if (issue.is_saved) {
				await api.unsaveIssue(issueId);
				issue.is_saved = false;
			} else {
				await api.saveIssue(issueId);
				issue.is_saved = true;
			}
			feed = { ...feed, issues: [...feed.issues] };
		} catch (e) {
			console.error('Save toggle failed:', e);
		}
	}

	async function search() {
		if (!searchQuery.trim()) {
			loadFeed();
			return;
		}
		loading = true;
		isSearching = true;
		try {
			feed = await api.searchIssues(searchQuery, 1);
		} catch (e) {
			console.error('Search failed:', e);
		} finally {
			loading = false;
		}
	}

	function clearSearch() {
		searchQuery = '';
		page = 1;
		loadFeed();
	}

	function toggleCategory(slug: string) {
		selectedCategory = selectedCategory === slug ? '' : slug;
		page = 1;
		loadFeed();
	}

	function toggleDifficulty(d: number) {
		selectedDifficulty = selectedDifficulty === d ? 0 : d;
		page = 1;
		loadFeed();
	}

	function toggleLanguage(lang: string) {
		languageFilters = { ...languageFilters, [lang]: !languageFilters[lang] };
		page = 1;
		loadFeed();
	}

	function clearFilters() {
		selectedCategory = '';
		selectedDifficulty = 0;
		// Reset all language filters to true
		const reset: Record<string, boolean> = {};
		for (const lang of Object.keys(languageFilters)) {
			reset[lang] = true;
		}
		languageFilters = reset;
		page = 1;
		loadFeed();
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

	function matchGrade(score: number): string {
		if (score >= 0.8) return 'A';
		if (score >= 0.6) return 'B';
		if (score >= 0.4) return 'C';
		return 'D';
	}

	function healthLabel(score: number): string {
		if (score >= 0.8) return '++';
		if (score >= 0.6) return '+ ';
		if (score >= 0.4) return '~ ';
		return '- ';
	}

	function healthColor(score: number): string {
		if (score >= 0.8) return '#4ade80';
		if (score >= 0.6) return '#fbbf24';
		if (score >= 0.4) return '#fb923c';
		return '#f87171';
	}

	const totalPages = $derived(feed ? Math.ceil(feed.total_count / feed.per_page) : 0);
</script>

<div class="feed-page">
	<!-- Header -->
	<header class="header">
		<div class="header-top">
			<div class="header-text">
				<h1>your feed<span class="cursor">_</span></h1>
				<p class="subtitle">issues matched to your skills and goals</p>
			</div>
			{#if feed && !loading}
				<div class="header-stats">
					<div class="stat-pill">
						<span class="stat-num">{feed.total_count}</span>
						<span class="stat-label">issues</span>
					</div>
				</div>
			{/if}
		</div>

		<!-- Search -->
		<div class="search-row">
			<div class="search-field">
				<span class="search-icon">&#8981;</span>
				<input
					type="text"
					placeholder="search by keyword, repo, or language..."
					bind:value={searchQuery}
					onkeydown={(e) => e.key === 'Enter' && search()}
				/>
				{#if searchQuery}
					<button class="search-clear" onclick={clearSearch}>&times;</button>
				{/if}
			</div>
			<button class="search-btn" onclick={search}>search</button>
		</div>

		<!-- Import Repo -->
		<div class="import-row">
			<button class="import-toggle" onclick={() => showImport = !showImport}>
				{showImport ? '- hide import' : '+ import repo'}
			</button>
			{#if showImport}
				<div class="import-field">
					<input
						type="text"
						placeholder="github.com/owner/repo or owner/repo"
						bind:value={importUrl}
						onkeydown={(e) => e.key === 'Enter' && doImport()}
						disabled={importing}
					/>
					<button class="search-btn" onclick={doImport} disabled={importing}>
						{importing ? 'indexing...' : 'import'}
					</button>
				</div>
				{#if importResult}
					<p class="import-success">indexed {importResult}</p>
				{/if}
				{#if importError}
					<p class="import-error">{importError}</p>
				{/if}
			{/if}
		</div>

		{#if isSearching && !loading}
			<div class="search-status">
				showing results for "<span class="search-term">{searchQuery}</span>"
				<button class="clear-link" onclick={clearSearch}>clear</button>
			</div>
		{/if}
	</header>

	<!-- Filters -->
	{#if categories.length > 0}
		<div class="filters">
			<div class="filter-group">
				<span class="filter-label">category</span>
				<div class="filter-chips">
					{#each categories as cat}
						<button
							class="chip"
							class:chip-active={selectedCategory === cat.slug}
							onclick={() => toggleCategory(cat.slug)}
						>
							{#if cat.icon}<span class="chip-icon">{cat.icon}</span>{/if}
							{cat.name}
						</button>
					{/each}
				</div>
			</div>

			{#if Object.keys(languageFilters).length > 0}
				<div class="filter-group">
					<span class="filter-label">language</span>
					<div class="filter-chips">
						{#each Object.keys(languageFilters) as lang}
							<button
								class="chip"
								class:chip-active={languageFilters[lang]}
								onclick={() => toggleLanguage(lang)}
							>
								{lang}
							</button>
						{/each}
					</div>
				</div>
			{/if}

			<div class="filter-row-bottom">
				<div class="filter-group">
					<span class="filter-label">difficulty</span>
					<div class="filter-chips">
						<button class="chip chip-green" class:chip-active={selectedDifficulty === 1} onclick={() => toggleDifficulty(1)}>beginner</button>
						<button class="chip chip-amber" class:chip-active={selectedDifficulty === 2} onclick={() => toggleDifficulty(2)}>intermediate</button>
						<button class="chip chip-red" class:chip-active={selectedDifficulty === 3} onclick={() => toggleDifficulty(3)}>advanced</button>
					</div>
				</div>

				{#if hasActiveFilters}
					<button class="clear-filters" onclick={clearFilters}>clear filters</button>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Feed -->
	{#if loading}
		<div class="loading-state">
			<div class="loading-grid">
				{#each Array(6) as _, i}
					<div class="skeleton-card" style="animation-delay: {i * 80}ms"></div>
				{/each}
			</div>
		</div>
	{:else if feed?.issues?.length}
		<div class="issue-grid">
			{#each feed.issues as issue, i (issue.id)}
				<a href="/app/issues/{issue.id}" class="card" style="animation-delay: {i * 40}ms">
					<!-- Save button -->
					<button
						class="save-icon"
						class:save-icon-active={issue.is_saved}
						onclick={(e) => { e.preventDefault(); e.stopPropagation(); toggleSave(issue.id); }}
						title={issue.is_saved ? 'unsave' : 'save'}
					>
						{issue.is_saved ? '\u2605' : '\u2606'}
					</button>

					<!-- Top bar: repo + health + number -->
					<div class="card-top">
						<span class="card-repo">{issue.repo?.owner}/{issue.repo?.name}</span>
						{#if issue.repo?.health_score}
							<span class="health-badge" style="color: {healthColor(issue.repo.health_score)}" title="health: {Math.round(issue.repo.health_score * 100)}%">{healthLabel(issue.repo.health_score)}</span>
						{/if}
						<span class="card-num">#{issue.number}</span>
					</div>

					<!-- Title -->
					<h3 class="card-title">{issue.title}</h3>

					<!-- Summary if available -->
					{#if issue.summary}
						<p class="card-summary">{issue.summary}</p>
					{/if}

					<!-- Meta row -->
					<div class="card-meta">
						<span class="meta-difficulty" style="color: {difficultyColor(issue.difficulty)}">
							<span class="meta-dot" style="background: {difficultyColor(issue.difficulty)}"></span>
							{difficultyLabel(issue.difficulty)}
						</span>
						<span class="meta-item"><span class="meta-dot" style="background: #60a5fa"></span>{issue.time_estimate}</span>
						<span class="meta-item"><span class="meta-dot" style="background: #c084fc"></span>{issue.comment_count} comments</span>
					</div>

					<!-- Labels -->
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

					<!-- Footer: match score + reasons -->
					<div class="card-footer">
						{#if issue.match_score}
							<div class="match-badge" class:match-high={issue.match_score >= 0.7} class:match-mid={issue.match_score >= 0.4 && issue.match_score < 0.7}>
								<span class="match-grade">{matchGrade(issue.match_score)}</span>
								<span class="match-pct">{Math.round(issue.match_score * 100)}%</span>
							</div>
						{/if}
						{#if issue.match_reasons?.length}
							<div class="card-reasons">
								{#each issue.match_reasons.slice(0, 3) as reason}
									<span class="reason">{reason}</span>
								{/each}
							</div>
						{/if}
					</div>
				</a>
			{/each}
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<nav class="pagination">
				<button
					class="page-btn"
					disabled={page <= 1}
					onclick={() => { page--; loadFeed(); }}
				>
					&larr; prev
				</button>

				<div class="page-dots">
					{#each Array(Math.min(totalPages, 7)) as _, i}
						{@const p = totalPages <= 7 ? i + 1 : (page <= 4 ? i + 1 : (page >= totalPages - 3 ? totalPages - 6 + i : page - 3 + i))}
						<button
							class="page-dot"
							class:page-dot-active={p === page}
							onclick={() => { page = p; loadFeed(); }}
						>
							{p}
						</button>
					{/each}
				</div>

				<button
					class="page-btn"
					disabled={page >= totalPages}
					onclick={() => { page++; loadFeed(); }}
				>
					next &rarr;
				</button>
			</nav>
		{/if}
	{:else}
		<div class="empty-state">
			<div class="empty-icon">&#9675;</div>
			<p class="empty-title">no issues found</p>
			<p class="empty-sub">try adjusting your skills in onboarding or check back later</p>
			{#if isSearching}
				<button class="btn-ghost" onclick={clearSearch}>clear search</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* ── Page ── */
	.feed-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* ── Header ── */
	.header {
		margin-bottom: 1.75rem;
	}

	.header-top {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		margin-bottom: 1.25rem;
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

	.stat-num {
		color: var(--amber);
		font-weight: 700;
	}

	.stat-label {
		color: var(--text-dim);
	}

	/* ── Search ── */
	.search-row {
		display: flex;
		gap: 0.5rem;
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

	.search-field input::placeholder {
		color: var(--text-dim);
	}

	.search-field input:focus {
		border-color: var(--amber-dim);
	}

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
		transition: color 0.15s;
	}

	.search-clear:hover { color: var(--text); }

	.search-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 500;
		padding: 0.6rem 1.25rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.search-btn:hover {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
	}

	.search-status {
		margin-top: 0.75rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.search-term {
		color: var(--amber);
	}

	.clear-link {
		background: none;
		border: none;
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		cursor: pointer;
		text-decoration: underline;
		margin-left: 0.5rem;
	}

	.clear-link:hover { color: var(--text); }

	/* ── Import ── */
	.import-row {
		margin-top: 0.75rem;
	}

	.import-toggle {
		background: none;
		border: none;
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		cursor: pointer;
		padding: 0;
		transition: color 0.15s;
	}

	.import-toggle:hover { color: var(--amber); }

	.import-field {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.import-field input {
		flex: 1;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		outline: none;
		transition: border-color 0.15s;
	}

	.import-field input:focus { border-color: var(--amber-dim); }
	.import-field input::placeholder { color: var(--text-dim); }

	.import-success {
		font-size: 0.72rem;
		color: var(--green);
		margin-top: 0.35rem;
	}

	.import-error {
		font-size: 0.72rem;
		color: var(--red);
		margin-top: 0.35rem;
	}

	/* ── Filters ── */
	.filters {
		margin-bottom: 1.5rem;
		padding: 1rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.filter-group {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.filter-label {
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--text-dim);
		font-weight: 500;
		flex-shrink: 0;
		min-width: 60px;
	}

	.filter-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.filter-row-bottom {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.chip {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		padding: 0.2rem 0.55rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-muted);
		border-radius: 3px;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.chip:hover {
		border-color: var(--amber-dim);
		color: var(--text);
	}

	.chip-active {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		color: var(--amber);
		font-weight: 500;
	}

	.chip-icon {
		margin-right: 0.2rem;
	}

	.chip-green.chip-active {
		background: rgba(74, 222, 128, 0.1);
		border-color: rgba(74, 222, 128, 0.3);
		color: var(--green);
	}

	.chip-amber.chip-active {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	.chip-red.chip-active {
		background: rgba(248, 113, 113, 0.1);
		border-color: rgba(248, 113, 113, 0.3);
		color: var(--red);
	}

	.clear-filters {
		background: none;
		border: none;
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.68rem;
		cursor: pointer;
		text-decoration: underline;
		transition: color 0.15s;
	}

	.clear-filters:hover { color: var(--text); }

	/* ── Loading skeleton ── */
	.loading-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 700px) {
		.loading-grid { grid-template-columns: 1fr; }
	}

	.skeleton-card {
		height: 180px;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 0.8; }
	}

	/* ── Issue Grid ── */
	.issue-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (max-width: 700px) {
		.issue-grid { grid-template-columns: 1fr; }
	}

	/* ── Card ── */
	.card {
		display: flex;
		flex-direction: column;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.15rem 1.25rem;
		text-decoration: none;
		transition: all 0.2s;
		animation: cardIn 0.35s ease both;
		position: relative;
	}

	@keyframes cardIn {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
		transform: translateY(-1px);
	}

	.card:hover .card-title {
		color: var(--amber);
	}

	/* ── Save Icon ── */
	.save-icon {
		position: absolute;
		top: 0.6rem;
		right: 0.6rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-dim);
		font-size: 0.9rem;
		width: 26px;
		height: 26px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		z-index: 2;
		opacity: 0;
	}

	.card:hover .save-icon,
	.save-icon-active {
		opacity: 1;
	}

	.save-icon:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
		background: var(--amber-glow);
	}

	.save-icon-active {
		color: var(--amber);
		border-color: var(--amber-dim);
		background: var(--amber-glow);
	}

	.save-icon-active:hover {
		color: var(--red);
		border-color: var(--red);
		background: rgba(248, 113, 113, 0.08);
	}

	/* ── Card Top ── */
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

	.health-badge {
		font-size: 0.65rem;
		font-weight: 700;
		font-family: var(--font-mono);
		flex-shrink: 0;
		margin-left: 0.3rem;
	}

	.card-num {
		font-size: 0.7rem;
		color: var(--text-dim);
		flex-shrink: 0;
		margin-left: 0.5rem;
	}

	/* ── Card Title ── */
	.card-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
		line-height: 1.45;
		margin-bottom: 0.4rem;
		transition: color 0.15s;
		/* Clamp to 2 lines */
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	/* ── Card Summary ── */
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

	/* ── Card Meta ── */
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

	/* ── Card Labels ── */
	.card-labels {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin-bottom: 0.6rem;
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

	/* ── Card Footer ── */
	.card-footer {
		margin-top: auto;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}

	.match-badge {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.15rem 0.45rem;
		flex-shrink: 0;
	}

	.match-high {
		border-color: var(--amber-dim);
		background: var(--amber-glow);
	}

	.match-grade {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-dim);
	}

	.match-high .match-grade { color: var(--amber); }
	.match-mid .match-grade { color: var(--text-muted); }

	.match-pct {
		font-size: 0.65rem;
		font-weight: 600;
		color: var(--text-muted);
	}

	.match-high .match-pct { color: var(--amber); }

	.card-reasons {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		min-width: 0;
	}

	.reason {
		font-size: 0.6rem;
		padding: 0.1rem 0.4rem;
		background: rgba(74, 222, 128, 0.08);
		color: var(--green);
		border-radius: 2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 140px;
	}

	/* ── Pagination ── */
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

	.page-btn:disabled {
		opacity: 0.25;
		cursor: not-allowed;
	}

	.page-dots {
		display: flex;
		gap: 0.25rem;
	}

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

	/* ── Empty State ── */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 4rem 2rem;
		text-align: center;
	}

	.empty-icon {
		font-size: 2rem;
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
	}

	.btn-ghost:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
	}
</style>
