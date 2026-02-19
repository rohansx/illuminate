<script lang="ts">
	import { api, type User, type ProfileStats, type GitHubPR } from '$lib/api';
	import { onMount } from 'svelte';

	let stats = $state<ProfileStats | null>(null);
	let prs = $state<GitHubPR[]>([]);
	let prTab = $state<'merged' | 'open'>('merged');
	let prPage = $state(1);
	let prTotal = $state(0);
	let loading = $state(true);
	let prLoading = $state(false);
	let error = $state('');

	onMount(async () => {
		try {
			stats = await api.getProfileStats();
			await loadPRs();
		} catch (e: any) {
			error = e.message || 'Failed to load profile';
		} finally {
			loading = false;
		}
	});

	async function loadPRs() {
		prLoading = true;
		try {
			const result = await api.getMyPRs(prTab, prPage, 10);
			prs = result.items || [];
			prTotal = result.total_count;
		} catch {
			prs = [];
			prTotal = 0;
		} finally {
			prLoading = false;
		}
	}

	async function switchTab(tab: 'merged' | 'open') {
		prTab = tab;
		prPage = 1;
		await loadPRs();
	}

	async function nextPage() {
		prPage++;
		await loadPRs();
	}

	async function prevPage() {
		if (prPage > 1) {
			prPage--;
			await loadPRs();
		}
	}

	function repoFromURL(url: string): string {
		const parts = url.replace('https://api.github.com/repos/', '').split('/');
		return parts.slice(0, 2).join('/');
	}

	function timeAgo(dateStr: string): string {
		const now = Date.now();
		const then = new Date(dateStr).getTime();
		const diff = Math.floor((now - then) / 1000);
		if (diff < 60) return 'just now';
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		if (diff < 2592000) return `${Math.floor(diff / 86400)}d ago`;
		return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}

	function skillLevel(proficiency: number): string {
		if (proficiency >= 80) return 'expert';
		if (proficiency >= 60) return 'advanced';
		if (proficiency >= 40) return 'intermediate';
		return 'beginner';
	}

	const skillPalette = ['#4ade80', '#60a5fa', '#c084fc', '#f472b6', '#fbbf24', '#67e8f9', '#fb923c', '#a78bfa'];
	function skillColor(name: string): string {
		let hash = 0;
		for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
		return skillPalette[Math.abs(hash) % skillPalette.length];
	}

	function getLabelColor(label: string): { bg: string; fg: string } {
		const l = label.toLowerCase();
		if (l.includes('good first issue') || l.includes('beginner') || l.includes('easy')) return { bg: 'rgba(74, 222, 128, 0.12)', fg: '#4ade80' };
		if (l.includes('help wanted') || l.includes('contributions')) return { bg: 'rgba(103, 232, 249, 0.12)', fg: '#67e8f9' };
		if (l.includes('bug') || l.includes('fix')) return { bg: 'rgba(248, 113, 113, 0.12)', fg: '#f87171' };
		if (l.includes('type:') || l.includes('documentation') || l.includes('docs') || l.includes('cleanup')) return { bg: 'rgba(96, 165, 250, 0.12)', fg: '#60a5fa' };
		if (l.includes('team-') || l.includes('team:')) return { bg: 'rgba(192, 132, 252, 0.12)', fg: '#c084fc' };
		return { bg: 'var(--amber-glow)', fg: 'var(--amber)' };
	}
</script>

{#if loading}
	<div class="profile-loading">
		<div class="spinner"></div>
		<p>loading profile...</p>
	</div>
{:else if error}
	<div class="profile-error">
		<p class="error-text">{error}</p>
	</div>
{:else if stats}
	<div class="profile-page">

		<!-- Header -->
		<header class="profile-header">
			<div class="header-left">
				<img src={stats.user.avatar_url} alt={stats.user.github_username} class="profile-avatar" />
				<div class="header-info">
					<h1 class="profile-name">{stats.user.github_username}</h1>
					{#if stats.user.bio}
						<p class="profile-bio">{stats.user.bio}</p>
					{/if}
					<div class="profile-meta">
						<a href="https://github.com/{stats.user.github_username}" target="_blank" rel="noopener" class="github-link">
							<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
							github.com/{stats.user.github_username}
						</a>
						{#if stats.user.comfort_level}
							<span class="meta-tag">{stats.user.comfort_level}</span>
						{/if}
						{#if stats.user.time_commitment}
							<span class="meta-tag">{stats.user.time_commitment}</span>
						{/if}
					</div>
				</div>
			</div>
		</header>

		<!-- Stats Cards -->
		<div class="stats-grid">
			<div class="stat-card">
				<span class="stat-value" style="color: #c084fc">{stats.merged_pr_count}</span>
				<span class="stat-label">merged PRs</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #4ade80">{stats.open_pr_count}</span>
				<span class="stat-label">open PRs</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #fbbf24">{stats.saved_count}</span>
				<span class="stat-label">saved issues</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #60a5fa">{stats.user.skills?.length || 0}</span>
				<span class="stat-label">languages</span>
			</div>
		</div>

		<div class="profile-columns">
			<!-- Left Column: Skills + Goals + illuminate card -->
			<div class="profile-sidebar">

				<!-- Skills -->
				{#if stats.user.skills?.length}
					<div class="profile-section">
						<h3 class="section-heading">// skills</h3>
						<div class="skills-list">
							{#each stats.user.skills as skill}
								<div class="skill-row">
									<div class="skill-info">
										<span class="skill-name"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}</span>
										<span class="skill-level">{skillLevel(skill.proficiency)}</span>
									</div>
									<div class="skill-bar-bg">
										<div class="skill-bar-fill" style="width: {skill.proficiency}%; background: {skillColor(skill.language)}"></div>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Goals -->
				{#if stats.user.goals?.length}
					<div class="profile-section">
						<h3 class="section-heading">// goals</h3>
						<div class="goals-list">
							{#each stats.user.goals as goal}
								<div class="goal-item">&gt; {goal}</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Illuminate Card / Widget -->
				<div class="profile-section illuminate-card">
					<div class="card-header">
						<span class="card-logo">illuminate<span class="blink">_</span></span>
						<span class="card-badge">contributor</span>
					</div>
					<div class="card-user">
						<img src={stats.user.avatar_url} alt="" class="card-avatar" />
						<div>
							<div class="card-username">{stats.user.github_username}</div>
							{#if stats.user.bio}
								<div class="card-bio">{stats.user.bio}</div>
							{/if}
						</div>
					</div>
					<div class="card-stats">
						<div class="card-stat">
							<span class="card-stat-val">{stats.merged_pr_count}</span>
							<span class="card-stat-lbl">merged</span>
						</div>
						<div class="card-stat">
							<span class="card-stat-val">{stats.open_pr_count}</span>
							<span class="card-stat-lbl">open</span>
						</div>
						<div class="card-stat">
							<span class="card-stat-val">{stats.user.skills?.length || 0}</span>
							<span class="card-stat-lbl">langs</span>
						</div>
					</div>
					{#if stats.user.skills?.length}
						<div class="card-skills">
							{#each stats.user.skills.slice(0, 5) as skill}
								<span class="card-skill-tag"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}</span>
							{/each}
						</div>
					{/if}
					<div class="card-footer">
						<span class="card-watermark">illuminate.dev</span>
					</div>
				</div>
			</div>

			<!-- Right Column: PRs -->
			<div class="profile-main">
				<div class="profile-section">
					<div class="pr-header">
						<h3 class="section-heading">// pull requests</h3>
						<div class="pr-tabs">
							<button
								class="pr-tab"
								class:active={prTab === 'merged'}
								onclick={() => switchTab('merged')}
							>
								merged ({stats.merged_pr_count})
							</button>
							<button
								class="pr-tab"
								class:active={prTab === 'open'}
								onclick={() => switchTab('open')}
							>
								open ({stats.open_pr_count})
							</button>
						</div>
					</div>

					{#if prLoading}
						<div class="pr-loading">
							<div class="spinner-sm"></div>
						</div>
					{:else if prs.length === 0}
						<div class="pr-empty">
							<p>no {prTab} pull requests found</p>
						</div>
					{:else}
						<div class="pr-list">
							{#each prs as pr}
								<a href={pr.html_url} target="_blank" rel="noopener" class="pr-item">
									<div class="pr-icon" class:merged={prTab === 'merged'}>
										{#if prTab === 'merged'}
											<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M5 3.254V3.25v.005a.75.75 0 110-.005v.004zm.45 1.9a2.25 2.25 0 10-1.95.218v5.256a2.25 2.25 0 101.5 0V7.123A5.735 5.735 0 009.25 9h1.378a2.251 2.251 0 100-1.5H9.25a4.25 4.25 0 01-3.8-2.346zM12.75 9a.75.75 0 100-1.5.75.75 0 000 1.5zm-8.5 4.5a.75.75 0 100-1.5.75.75 0 000 1.5z"/></svg>
										{:else}
											<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 3.25a2.25 2.25 0 113 2.122v5.256a2.251 2.251 0 11-1.5 0V5.372A2.25 2.25 0 011.5 3.25zm5.677-.177L9.573.677A.25.25 0 0110 .854v2.792a.25.25 0 01-.427.177L7.177 1.427a.25.25 0 010-.354zM10 4.646l-2.396 2.396a.25.25 0 01-.427-.177V4.073a.25.25 0 01.427-.177L10 6.292V13.5a.75.75 0 01-.75.75h-5.5a.75.75 0 010-1.5H8.5V5.793L6.104 3.396a.25.25 0 01.177-.427h2.792a.25.25 0 01.177.427L10 4.646z"/></svg>
										{/if}
									</div>
									<div class="pr-info">
										<div class="pr-title">{pr.title}</div>
										<div class="pr-meta">
											<span class="pr-repo">{repoFromURL(pr.repository_url)}</span>
											<span class="pr-sep">·</span>
											<span class="pr-number">#{pr.number}</span>
											<span class="pr-sep">·</span>
											<span class="pr-time">{timeAgo(pr.updated_at)}</span>
										</div>
									</div>
									{#if pr.labels?.length}
										<div class="pr-labels">
											{#each pr.labels.slice(0, 3) as label}
												<span class="pr-label" style="background: {getLabelColor(label.name).bg}; color: {getLabelColor(label.name).fg}; border-color: transparent">{label.name}</span>
											{/each}
										</div>
									{/if}
								</a>
							{/each}
						</div>

						<!-- Pagination -->
						{#if prTotal > 10}
							<div class="pr-pagination">
								<button class="pr-page-btn" disabled={prPage === 1} onclick={prevPage}>&larr; prev</button>
								<span class="pr-page-info">page {prPage} of {Math.ceil(prTotal / 10)}</span>
								<button class="pr-page-btn" disabled={prPage * 10 >= prTotal} onclick={nextPage}>next &rarr;</button>
							</div>
						{/if}
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.profile-loading, .profile-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 50vh;
		gap: 1rem;
		color: var(--text-muted);
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--border);
		border-top-color: var(--amber);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	.spinner-sm {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border);
		border-top-color: var(--amber);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }

	.error-text { color: var(--red); }

	.profile-page {
		animation: fade-in 0.4s ease;
	}

	@keyframes fade-in {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}

	/* Header */
	.profile-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		margin-bottom: 2rem;
		padding-bottom: 2rem;
		border-bottom: 1px solid var(--border);
	}

	.header-left {
		display: flex;
		gap: 1.25rem;
		align-items: flex-start;
	}

	.profile-avatar {
		width: 72px;
		height: 72px;
		border-radius: 50%;
		border: 2px solid var(--border-bright);
		flex-shrink: 0;
	}

	.profile-name {
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--text-bright);
		line-height: 1.2;
	}

	.profile-bio {
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 0.25rem;
		max-width: 400px;
	}

	.profile-meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-top: 0.5rem;
		flex-wrap: wrap;
	}

	.github-link {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		transition: color 0.15s;
	}

	.github-link:hover { color: var(--amber); }

	.meta-tag {
		font-size: 0.65rem;
		padding: 0.15rem 0.5rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--text-dim);
		text-transform: lowercase;
	}

	/* Stats Grid */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem;
		text-align: center;
		transition: border-color 0.2s;
	}

	.stat-card:hover { border-color: var(--border-bright); }

	.stat-value {
		display: block;
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--amber);
		line-height: 1.2;
	}

	.stat-label {
		font-size: 0.7rem;
		color: var(--text-dim);
		text-transform: lowercase;
		margin-top: 0.25rem;
		display: block;
	}

	/* Columns */
	.profile-columns {
		display: grid;
		grid-template-columns: 320px 1fr;
		gap: 2rem;
	}

	/* Section */
	.profile-section {
		margin-bottom: 1.5rem;
	}

	.section-heading {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-dim);
		margin-bottom: 0.75rem;
		letter-spacing: 0.04em;
	}

	/* Skills */
	.skills-list {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.skill-row {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.skill-info {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.skill-name {
		font-size: 0.78rem;
		color: var(--text);
		font-weight: 500;
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.skill-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.skill-level {
		font-size: 0.65rem;
		color: var(--text-dim);
	}

	.skill-bar-bg {
		height: 4px;
		background: var(--border);
		border-radius: 2px;
		overflow: hidden;
	}

	.skill-bar-fill {
		height: 100%;
		background: var(--amber);
		border-radius: 2px;
		transition: width 0.6s ease;
	}

	/* Goals */
	.goals-list {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.goal-item {
		font-size: 0.78rem;
		color: var(--text-muted);
		padding: 0.35rem 0;
		border-bottom: 1px solid var(--border);
	}

	.goal-item:last-child { border-bottom: none; }

	/* Illuminate Card */
	.illuminate-card {
		background: var(--bg-card);
		border: 1px solid var(--border-bright);
		border-radius: 8px;
		padding: 1.25rem;
		position: relative;
		overflow: hidden;
	}

	.illuminate-card::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 2px;
		background: linear-gradient(90deg, var(--amber), var(--amber-dim), transparent);
	}

	.card-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.card-logo {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--amber);
	}

	.blink { animation: blink-cursor 1s step-end infinite; }
	@keyframes blink-cursor { 50% { opacity: 0; } }

	.card-badge {
		font-size: 0.6rem;
		padding: 0.15rem 0.5rem;
		border: 1px solid var(--amber-dim);
		border-radius: 3px;
		color: var(--amber);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.card-user {
		display: flex;
		gap: 0.75rem;
		align-items: center;
		margin-bottom: 1rem;
	}

	.card-avatar {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		border: 1px solid var(--border);
	}

	.card-username {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.card-bio {
		font-size: 0.7rem;
		color: var(--text-muted);
		margin-top: 0.1rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 180px;
	}

	.card-stats {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.5rem;
		margin-bottom: 0.75rem;
		padding: 0.6rem 0;
		border-top: 1px solid var(--border);
		border-bottom: 1px solid var(--border);
	}

	.card-stat {
		text-align: center;
	}

	.card-stat-val {
		display: block;
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-bright);
	}

	.card-stat-lbl {
		font-size: 0.6rem;
		color: var(--text-dim);
	}

	.card-skills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
		margin-bottom: 0.75rem;
	}

	.card-skill-tag {
		font-size: 0.6rem;
		padding: 0.15rem 0.4rem;
		background: var(--amber-glow);
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--amber);
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.card-footer {
		text-align: right;
	}

	.card-watermark {
		font-size: 0.6rem;
		color: var(--text-dim);
		letter-spacing: 0.06em;
	}

	/* PRs */
	.pr-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.pr-tabs {
		display: flex;
		gap: 0.5rem;
	}

	.pr-tab {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.3rem 0.6rem;
		background: none;
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.pr-tab:hover { border-color: var(--border-bright); color: var(--text); }
	.pr-tab.active {
		border-color: var(--amber-dim);
		color: var(--amber);
		background: var(--amber-glow);
	}

	.pr-loading {
		display: flex;
		justify-content: center;
		padding: 2rem;
	}

	.pr-empty {
		text-align: center;
		padding: 2rem;
		color: var(--text-dim);
		font-size: 0.8rem;
	}

	.pr-list {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.pr-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 0.75rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-decoration: none;
		transition: background 0.15s;
	}

	.pr-item:hover { background: var(--amber-glow); }
	.pr-item:last-child { border-bottom: none; }

	.pr-icon {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--green);
		flex-shrink: 0;
		margin-top: 2px;
	}

	.pr-icon.merged {
		color: var(--amber);
		border-color: var(--amber-dim);
	}

	.pr-info {
		flex: 1;
		min-width: 0;
	}

	.pr-title {
		font-size: 0.82rem;
		color: var(--text-bright);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pr-meta {
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-top: 0.2rem;
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.pr-repo { color: var(--text-muted); }
	.pr-sep { opacity: 0.4; }
	.pr-number { color: var(--amber-dim); }

	.pr-labels {
		display: flex;
		gap: 0.25rem;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.pr-label {
		font-size: 0.6rem;
		padding: 0.1rem 0.35rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--text-dim);
	}

	/* Pagination */
	.pr-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.pr-page-btn {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.3rem 0.6rem;
		background: none;
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.pr-page-btn:hover:not(:disabled) {
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	.pr-page-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.pr-page-info {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	/* Responsive */
	@media (max-width: 800px) {
		.stats-grid { grid-template-columns: repeat(2, 1fr); }
		.profile-columns {
			grid-template-columns: 1fr;
		}
		.profile-header { flex-direction: column; }
		.header-left { flex-direction: column; align-items: center; text-align: center; }
		.profile-meta { justify-content: center; }
		.pr-header { flex-direction: column; gap: 0.5rem; }
	}
</style>
