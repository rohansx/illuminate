<script lang="ts">
	import { page } from '$app/stores';
	import { api, type PublicProfile } from '$lib/api';
	import { onMount } from 'svelte';

	let profile = $state<PublicProfile | null>(null);
	let loading = $state(true);
	let error = $state('');

	const username = $derived($page.params.username);

	onMount(async () => {
		try {
			profile = await api.getPublicProfile(username!);
		} catch (e: any) {
			if (e.status === 404) {
				error = 'User not found';
			} else {
				error = e.message || 'Failed to load profile';
			}
		} finally {
			loading = false;
		}
	});

	function timeAgo(dateStr: string | null): string {
		if (!dateStr) return '';
		const now = Date.now();
		const then = new Date(dateStr).getTime();
		const diff = Math.floor((now - then) / 1000);
		if (diff < 60) return 'just now';
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		if (diff < 2592000) return `${Math.floor(diff / 86400)}d ago`;
		return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return '';
		return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}

	const skillPalette = ['#4ade80', '#60a5fa', '#c084fc', '#f472b6', '#fbbf24', '#67e8f9', '#fb923c', '#a78bfa'];
	function skillColor(name: string): string {
		let hash = 0;
		for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
		return skillPalette[Math.abs(hash) % skillPalette.length];
	}

	function langPercent(languages: Record<string, number>): { name: string; count: number; pct: number; color: string }[] {
		const total = Object.values(languages).reduce((a, b) => a + b, 0);
		if (total === 0) return [];
		return Object.entries(languages)
			.sort(([, a], [, b]) => b - a)
			.map(([name, count]) => ({
				name, count,
				pct: Math.round((count / total) * 100),
				color: skillColor(name)
			}));
	}
</script>

<svelte:head>
	<title>{profile ? `${profile.user.github_username} — illuminate` : 'illuminate'}</title>
</svelte:head>

<div class="public-profile">
	{#if loading}
		<div class="center-state">
			<div class="spinner"></div>
			<p>loading profile...</p>
		</div>
	{:else if error}
		<div class="center-state">
			<h2 class="error-heading">{error}</h2>
			<p class="error-hint">this user may not have an illuminate account yet.</p>
			<a href="/" class="home-link">go to illuminate &rarr;</a>
		</div>
	{:else if profile}
		<nav class="pub-nav">
			<a href="/" class="nav-logo">illuminate<span class="blink">_</span></a>
		</nav>

		<!-- Header -->
		<header class="pub-header">
			<img src={profile.user.avatar_url} alt={profile.user.github_username} class="pub-avatar" />
			<div class="pub-info">
				<h1 class="pub-name">{profile.user.github_username}</h1>
				{#if profile.user.bio}
					<p class="pub-bio">{profile.user.bio}</p>
				{/if}
				<div class="pub-meta">
					<a href="https://github.com/{profile.user.github_username}" target="_blank" rel="noopener" class="github-link">
						<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
						github
					</a>
					<span class="meta-sep">·</span>
					<span class="meta-joined">member since {formatDate(profile.user.created_at)}</span>
				</div>
			</div>
		</header>

		<!-- Stats Row -->
		<div class="pub-stats-row">
			<div class="pub-stat">
				<span class="pub-stat-val" style="color: #c084fc">{profile.stats.total_prs}</span>
				<span class="pub-stat-lbl">contributions</span>
			</div>
			<div class="pub-stat">
				<span class="pub-stat-val" style="color: #4ade80">{profile.stats.total_repos}</span>
				<span class="pub-stat-lbl">projects</span>
			</div>
			<div class="pub-stat">
				<span class="pub-stat-val" style="color: #fbbf24">{profile.stats.current_streak}</span>
				<span class="pub-stat-lbl">day streak</span>
			</div>
			<div class="pub-stat">
				<span class="pub-stat-val" style="color: #60a5fa">{profile.user.skills?.length || 0}</span>
				<span class="pub-stat-lbl">languages</span>
			</div>
		</div>

		<div class="pub-columns">
			<!-- Left: Skills + Language Breakdown -->
			<div class="pub-sidebar">
				{#if profile.user.skills?.length}
					<div class="pub-section">
						<h3 class="sec-heading">// skills</h3>
						<div class="skills-list">
							{#each profile.user.skills as skill}
								<div class="skill-row">
									<div class="skill-info">
										<span class="skill-name"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}</span>
									</div>
									<div class="skill-bar-bg">
										<div class="skill-bar-fill" style="width: {skill.proficiency}%; background: {skillColor(skill.language)}"></div>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				{#if Object.keys(profile.stats.languages).length}
					<div class="pub-section">
						<h3 class="sec-heading">// languages</h3>
						<div class="lang-bar">
							{#each langPercent(profile.stats.languages) as lang}
								{#if lang.pct >= 2}
									<div class="lang-bar-seg" style="width: {lang.pct}%; background: {lang.color}" title="{lang.name}: {lang.pct}%"></div>
								{/if}
							{/each}
						</div>
						<div class="lang-list">
							{#each langPercent(profile.stats.languages) as lang}
								<div class="lang-item">
									<span class="lang-dot" style="background: {lang.color}"></span>
									<span class="lang-name">{lang.name}</span>
									<span class="lang-pct">{lang.pct}%</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			</div>

			<!-- Right: Projects + Recent PRs -->
			<div class="pub-main">
				{#if profile.top_projects?.length}
					<div class="pub-section">
						<h3 class="sec-heading">// top projects</h3>
						<div class="project-list">
							{#each profile.top_projects as project}
								<a href="https://github.com/{project.repo_owner}/{project.repo_name}" target="_blank" rel="noopener" class="project-card">
									<div class="project-header">
										<span class="project-name">{project.repo_owner}/<strong>{project.repo_name}</strong></span>
										{#if project.language}
											<span class="project-lang" style="color: {skillColor(project.language)}">{project.language}</span>
										{/if}
									</div>
									<div class="project-meta">
										<span class="project-prs">{project.pr_count} PR{project.pr_count !== 1 ? 's' : ''}</span>
									</div>
								</a>
							{/each}
						</div>
					</div>
				{/if}

				{#if profile.recent_prs?.length}
					<div class="pub-section">
						<h3 class="sec-heading">// recent contributions</h3>
						<div class="pr-list">
							{#each profile.recent_prs as pr}
								<a href={pr.pr_url} target="_blank" rel="noopener" class="pr-item">
									<div class="pr-icon">
										<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M5 3.254V3.25v.005a.75.75 0 110-.005v.004zm.45 1.9a2.25 2.25 0 10-1.95.218v5.256a2.25 2.25 0 101.5 0V7.123A5.735 5.735 0 009.25 9h1.378a2.251 2.251 0 100-1.5H9.25a4.25 4.25 0 01-3.8-2.346zM12.75 9a.75.75 0 100-1.5.75.75 0 000 1.5zm-8.5 4.5a.75.75 0 100-1.5.75.75 0 000 1.5z"/></svg>
									</div>
									<div class="pr-content">
										<div class="pr-title">{pr.pr_title}</div>
										<div class="pr-meta">
											<span class="pr-repo">{pr.repo_owner}/{pr.repo_name}</span>
											<span class="pr-sep">·</span>
											<span class="pr-time">{timeAgo(pr.merged_at || pr.created_at)}</span>
										</div>
									</div>
								</a>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		</div>

		<footer class="pub-footer">
			<span>powered by <a href="/">illuminate</a></span>
		</footer>
	{/if}
</div>

<style>
	.public-profile {
		max-width: 900px;
		margin: 0 auto;
		padding: 2rem 1.5rem;
		font-family: 'JetBrains Mono', monospace;
		color: var(--text);
		animation: fade-in 0.4s ease;
	}

	@keyframes fade-in {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.center-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 60vh;
		gap: 0.75rem;
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

	@keyframes spin { to { transform: rotate(360deg); } }

	.error-heading {
		font-size: 1.2rem;
		color: var(--text-bright);
		font-weight: 600;
	}

	.error-hint {
		font-size: 0.8rem;
		color: var(--text-dim);
	}

	.home-link {
		margin-top: 0.5rem;
		font-size: 0.8rem;
		color: var(--amber);
		text-decoration: none;
	}

	.home-link:hover { text-decoration: underline; }

	/* Nav */
	.pub-nav {
		margin-bottom: 2rem;
	}

	.nav-logo {
		font-size: 1rem;
		font-weight: 700;
		color: var(--amber);
		text-decoration: none;
	}

	.blink { animation: blink-cursor 1s step-end infinite; }
	@keyframes blink-cursor { 50% { opacity: 0; } }

	/* Header */
	.pub-header {
		display: flex;
		gap: 1.25rem;
		align-items: flex-start;
		margin-bottom: 2rem;
		padding-bottom: 2rem;
		border-bottom: 1px solid var(--border);
	}

	.pub-avatar {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		border: 2px solid var(--border-bright);
		flex-shrink: 0;
	}

	.pub-name {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-bright);
	}

	.pub-bio {
		color: var(--text-muted);
		font-size: 0.82rem;
		margin-top: 0.25rem;
	}

	.pub-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.github-link {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		color: var(--text-muted);
		text-decoration: none;
		transition: color 0.15s;
	}

	.github-link:hover { color: var(--amber); }

	.meta-sep { opacity: 0.4; }

	/* Stats Row */
	.pub-stats-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
		margin-bottom: 2rem;
	}

	.pub-stat {
		text-align: center;
		padding: 1rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.pub-stat-val {
		display: block;
		font-size: 1.5rem;
		font-weight: 700;
	}

	.pub-stat-lbl {
		font-size: 0.68rem;
		color: var(--text-dim);
		text-transform: lowercase;
		margin-top: 0.2rem;
		display: block;
	}

	/* Columns */
	.pub-columns {
		display: grid;
		grid-template-columns: 280px 1fr;
		gap: 2rem;
	}

	.pub-section { margin-bottom: 1.5rem; }

	.sec-heading {
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
		gap: 0.5rem;
	}

	.skill-row {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.skill-info {
		display: flex;
		justify-content: space-between;
	}

	.skill-name {
		font-size: 0.78rem;
		color: var(--text);
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

	.skill-bar-bg {
		height: 4px;
		background: var(--border);
		border-radius: 2px;
		overflow: hidden;
	}

	.skill-bar-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.6s ease;
	}

	/* Languages */
	.lang-bar {
		display: flex;
		height: 8px;
		border-radius: 4px;
		overflow: hidden;
		margin-bottom: 0.75rem;
		gap: 2px;
	}

	.lang-bar-seg {
		border-radius: 2px;
	}

	.lang-list {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.lang-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
	}

	.lang-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.lang-name {
		color: var(--text);
		flex: 1;
	}

	.lang-pct {
		color: var(--text-muted);
		font-weight: 600;
	}

	/* Projects */
	.project-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.project-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem 1rem;
		text-decoration: none;
		transition: all 0.15s;
	}

	.project-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-raised);
	}

	.project-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		margin-bottom: 0.25rem;
	}

	.project-name {
		font-size: 0.82rem;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.project-name strong { color: var(--text-bright); }

	.project-lang {
		font-size: 0.68rem;
		flex-shrink: 0;
	}

	.project-meta {
		font-size: 0.7rem;
	}

	.project-prs {
		color: var(--amber);
		font-weight: 600;
	}

	/* Recent PRs */
	.pr-list {
		display: flex;
		flex-direction: column;
	}

	.pr-item {
		display: flex;
		align-items: flex-start;
		gap: 0.65rem;
		padding: 0.6rem 0.25rem;
		border-bottom: 1px solid var(--border);
		text-decoration: none;
		transition: background 0.15s;
	}

	.pr-item:hover { background: var(--amber-glow); }
	.pr-item:last-child { border-bottom: none; }

	.pr-icon {
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: var(--bg-card);
		border: 1px solid var(--amber-dim);
		color: var(--amber);
		flex-shrink: 0;
		margin-top: 2px;
	}

	.pr-content {
		flex: 1;
		min-width: 0;
	}

	.pr-title {
		font-size: 0.8rem;
		color: var(--text-bright);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pr-meta {
		font-size: 0.68rem;
		color: var(--text-dim);
		margin-top: 0.15rem;
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.pr-repo { color: var(--text-muted); }
	.pr-sep { opacity: 0.4; }

	/* Footer */
	.pub-footer {
		margin-top: 3rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border);
		text-align: center;
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.pub-footer a {
		color: var(--amber);
		text-decoration: none;
	}

	.pub-footer a:hover { text-decoration: underline; }

	/* Responsive */
	@media (max-width: 700px) {
		.public-profile { padding: 1.5rem 1rem; }
		.pub-header { flex-direction: column; align-items: center; text-align: center; }
		.pub-meta { justify-content: center; }
		.pub-stats-row { grid-template-columns: repeat(2, 1fr); }
		.pub-columns { grid-template-columns: 1fr; }
	}
</style>
