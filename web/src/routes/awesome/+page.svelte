<script lang="ts">
	import { categories, allLanguages, type Category, type AwesomeRepo } from '$lib/data/awesome';
	import { reveal } from '$lib/actions/reveal';

	let search = $state('');
	let activeCategory = $state('all');
	let activeLanguage = $state('all');

	let filtered = $derived.by(() => {
		let cats = categories;

		if (activeCategory !== 'all') {
			cats = cats.filter(c => c.id === activeCategory);
		}

		return cats.map(cat => ({
			...cat,
			repos: cat.repos.filter(repo => {
				const matchesSearch = search === '' ||
					repo.name.toLowerCase().includes(search.toLowerCase()) ||
					repo.description.toLowerCase().includes(search.toLowerCase()) ||
					repo.owner.toLowerCase().includes(search.toLowerCase()) ||
					repo.tags.some(t => t.toLowerCase().includes(search.toLowerCase()));

				const matchesLang = activeLanguage === 'all' || repo.language === activeLanguage;

				return matchesSearch && matchesLang;
			})
		})).filter(cat => cat.repos.length > 0);
	});

	let totalCount = $derived(filtered.reduce((sum, cat) => sum + cat.repos.length, 0));
</script>

<svelte:head>
	<title>awesome repos — illuminate</title>
	<meta name="description" content="Curated collection of awesome open source repositories for developers. Browse by category, language, and search." />
</svelte:head>

<div class="awesome-page">
	<nav class="top-bar">
		<a href="/" class="logo">illuminate<span class="cursor">_</span></a>
		<div class="top-links">
			<a href="/#features">features</a>
			<a href="/awesome" class="active">awesome</a>
			<a href="/login" class="cta-link">get started</a>
		</div>
	</nav>

	<header class="page-header" use:reveal>
		<div class="terminal-tag">$ illuminate awesome --list</div>
		<h1>awesome repos<span class="cursor">_</span></h1>
		<p class="subtitle">
			a curated collection of <span class="highlight">{totalCount}</span> open source repositories worth contributing to
		</p>
	</header>

	<div class="controls" use:reveal>
		<div class="search-wrap">
			<span class="search-prefix">~/</span>
			<input
				type="text"
				placeholder="search repos, tags, languages..."
				bind:value={search}
			/>
		</div>

		<div class="filter-row">
			<div class="filter-group">
				<span class="filter-label">cat:</span>
				<button class="chip" class:active={activeCategory === 'all'} onclick={() => activeCategory = 'all'}>all</button>
				{#each categories as cat}
					<button class="chip" class:active={activeCategory === cat.id} onclick={() => activeCategory = cat.id}>
						<span class="chip-icon">{cat.icon}</span> {cat.label}
					</button>
				{/each}
			</div>

			<div class="filter-group">
				<span class="filter-label">lang:</span>
				<button class="chip" class:active={activeLanguage === 'all'} onclick={() => activeLanguage = 'all'}>all</button>
				{#each allLanguages as lang}
					<button class="chip" class:active={activeLanguage === lang} onclick={() => activeLanguage = lang}>{lang}</button>
				{/each}
			</div>
		</div>
	</div>

	<div class="results">
		{#each filtered as cat, i}
			<section class="category-section" use:reveal>
				<div class="category-header">
					<span class="cat-icon">{cat.icon}</span>
					<h2>{cat.label}</h2>
					<span class="cat-count">{cat.repos.length}</span>
				</div>

				<div class="repo-grid">
					{#each cat.repos as repo}
						<a
							href="https://github.com/{repo.owner}/{repo.name}"
							target="_blank"
							rel="noopener noreferrer"
							class="repo-card"
						>
							<div class="repo-top">
								<span class="repo-owner">{repo.owner}/</span>
								<span class="repo-name">{repo.name}</span>
							</div>
							<p class="repo-desc">{repo.description}</p>
							<div class="repo-meta">
								<span class="stars">★ {repo.stars}</span>
								<span class="lang">
									<span class="lang-dot" style="background: {langColor(repo.language)}"></span>
									{repo.language}
								</span>
							</div>
							<div class="repo-tags">
								{#each repo.tags as tag}
									<span class="tag">{tag}</span>
								{/each}
							</div>
						</a>
					{/each}
				</div>
			</section>
		{/each}

		{#if filtered.length === 0}
			<div class="empty" use:reveal>
				<p class="empty-cmd">$ illuminate search --query "{search}"</p>
				<p class="empty-result">no repos found. try different filters.</p>
			</div>
		{/if}
	</div>

	<footer class="page-footer">
		<p>
			want to suggest a repo? <a href="https://github.com/rohansx/illuminate" target="_blank" rel="noopener">open a PR</a>
		</p>
	</footer>
</div>

<script lang="ts" module>
	function langColor(lang: string): string {
		const colors: Record<string, string> = {
			'JavaScript': '#f1e05a',
			'TypeScript': '#3178c6',
			'Python': '#3572A5',
			'Go': '#00ADD8',
			'Rust': '#dea584',
			'C++': '#f34b7d',
			'C': '#555555',
			'Java': '#b07219',
			'CSS': '#563d7c',
			'SCSS': '#c6538c',
			'Svelte': '#ff3e00',
			'Zig': '#ec915c',
			'Vim Script': '#199f4b',
			'Markdown': '#083fa1',
		};
		return colors[lang] || '#8b8b8b';
	}
</script>

<style>
	.awesome-page {
		min-height: 100vh;
		max-width: var(--content-width);
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	/* ── Top bar ── */
	.top-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 0;
		border-bottom: 1px solid var(--border);
		margin-bottom: 3rem;
	}

	.logo {
		color: var(--amber);
		text-decoration: none;
		font-weight: 700;
		font-size: 1rem;
	}

	.cursor {
		animation: blink 1s step-end infinite;
	}

	@keyframes blink { 50% { opacity: 0; } }

	.top-links {
		display: flex;
		gap: 1.5rem;
		align-items: center;
	}

	.top-links a {
		color: var(--text-muted);
		text-decoration: none;
		font-size: 0.85rem;
		transition: color 0.15s;
	}

	.top-links a:hover,
	.top-links a.active { color: var(--amber); }

	.cta-link {
		border: 1px solid var(--amber-dim) !important;
		color: var(--amber) !important;
		padding: 0.3rem 0.75rem;
		border-radius: 4px;
	}

	/* ── Header ── */
	.page-header {
		margin-bottom: 2.5rem;
	}

	.terminal-tag {
		font-size: 0.75rem;
		color: var(--text-dim);
		margin-bottom: 0.75rem;
	}

	.page-header h1 {
		font-size: 2rem;
		font-weight: 700;
		color: var(--text-bright);
		line-height: 1.2;
		margin-bottom: 0.5rem;
	}

	.subtitle {
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.highlight {
		color: var(--amber);
		font-weight: 600;
	}

	/* ── Controls ── */
	.controls {
		margin-bottom: 2.5rem;
	}

	.search-wrap {
		display: flex;
		align-items: center;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0 1rem;
		margin-bottom: 1rem;
		transition: border-color 0.15s;
	}

	.search-wrap:focus-within {
		border-color: var(--amber);
	}

	.search-prefix {
		color: var(--amber);
		font-weight: 600;
		font-size: 0.9rem;
		margin-right: 0.5rem;
		user-select: none;
	}

	.search-wrap input {
		flex: 1;
		background: none;
		border: none;
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 0.7rem 0;
		outline: none;
	}

	.search-wrap input::placeholder {
		color: var(--text-dim);
	}

	.filter-row {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.filter-group {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.375rem;
	}

	.filter-label {
		color: var(--text-dim);
		font-size: 0.75rem;
		min-width: 3rem;
	}

	.chip {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		padding: 0.25rem 0.6rem;
		border-radius: 3px;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.chip:hover {
		border-color: var(--border-bright);
		color: var(--text);
	}

	.chip.active {
		border-color: var(--amber);
		color: var(--amber);
		background: var(--amber-glow);
	}

	.chip-icon {
		opacity: 0.6;
	}

	/* ── Results ── */
	.category-section {
		margin-bottom: 3rem;
	}

	.category-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	.cat-icon {
		color: var(--amber);
		font-weight: 700;
		font-size: 0.85rem;
		width: 2rem;
		text-align: center;
	}

	.category-header h2 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.cat-count {
		font-size: 0.7rem;
		color: var(--text-dim);
		background: var(--bg-raised);
		padding: 0.1rem 0.5rem;
		border-radius: 3px;
		border: 1px solid var(--border);
	}

	.repo-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 0.75rem;
	}

	.repo-card {
		display: block;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem 1.25rem;
		text-decoration: none;
		transition: all 0.2s;
	}

	.repo-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
		transform: translateY(-1px);
	}

	.repo-top {
		margin-bottom: 0.375rem;
	}

	.repo-owner {
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.repo-name {
		color: var(--amber);
		font-size: 0.8rem;
		font-weight: 600;
	}

	.repo-desc {
		color: var(--text);
		font-size: 0.8rem;
		line-height: 1.5;
		margin-bottom: 0.75rem;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.repo-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.stars {
		color: var(--amber);
	}

	.lang {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.lang-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		display: inline-block;
	}

	.repo-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}

	.tag {
		font-size: 0.65rem;
		padding: 0.1rem 0.4rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-dim);
		border-radius: 2px;
	}

	/* ── Empty ── */
	.empty {
		text-align: center;
		padding: 4rem 1rem;
	}

	.empty-cmd {
		color: var(--text-dim);
		font-size: 0.85rem;
		margin-bottom: 0.5rem;
	}

	.empty-result {
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	/* ── Footer ── */
	.page-footer {
		text-align: center;
		padding: 3rem 0;
		border-top: 1px solid var(--border);
		margin-top: 2rem;
	}

	.page-footer p {
		color: var(--text-dim);
		font-size: 0.8rem;
	}

	.page-footer a {
		color: var(--amber);
		text-decoration: none;
	}

	/* ── Responsive ── */
	@media (max-width: 640px) {
		.page-header h1 { font-size: 1.5rem; }
		.repo-grid { grid-template-columns: 1fr; }
		.top-links { gap: 0.75rem; }
	}
</style>
