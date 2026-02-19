<script lang="ts">
	import { api, type ProfileStats, type Contribution, type ContributionFeed, type ProjectGroup, type PortfolioStats, type UserSkill, type GrowthProfile } from '$lib/api';
	import { onMount } from 'svelte';

	let profileStats = $state<ProfileStats | null>(null);
	let contributions = $state<Contribution[]>([]);
	let contribPage = $state(1);
	let contribTotal = $state(0);
	let projects = $state<ProjectGroup[]>([]);
	let portfolioStats = $state<PortfolioStats | null>(null);
	let activeTab = $state<'timeline' | 'projects' | 'stats'>('timeline');
	let loading = $state(true);
	let tabLoading = $state(false);
	let syncing = $state(false);
	let error = $state('');
	let growth = $state<GrowthProfile | null>(null);

	// Skills editing
	let editingSkills = $state(false);
	let editSkillsList = $state<string[]>([]);
	let newSkillInput = $state('');
	let savingSkills = $state(false);

	onMount(async () => {
		try {
			profileStats = await api.getProfileStats();
		} catch (e: any) {
			error = e.message || 'Failed to load profile';
		} finally {
			loading = false;
		}

		// Load these independently so failures don't block the page
		api.getContributions(1, 20)
			.then(res => { contributions = res.contributions || []; contribTotal = res.total_count; })
			.catch(() => {});
		api.getPortfolioStats()
			.then(res => { portfolioStats = res; })
			.catch(() => {});
		api.getGrowthProfile()
			.then(res => { growth = res; })
			.catch(() => {});
	});

	async function switchTab(tab: 'timeline' | 'projects' | 'stats') {
		activeTab = tab;
		if (tab === 'projects' && projects.length === 0) {
			tabLoading = true;
			try {
				projects = await api.getContributionProjects() || [];
			} catch { projects = []; }
			finally { tabLoading = false; }
		}
	}

	async function loadMoreContributions() {
		contribPage++;
		tabLoading = true;
		try {
			const result = await api.getContributions(contribPage, 20);
			contributions = [...contributions, ...(result.contributions || [])];
		} catch { /* ignore */ }
		finally { tabLoading = false; }
	}

	async function syncContributions() {
		syncing = true;
		try {
			await api.syncContributions();
			const [contribs, stats, projs] = await Promise.all([
				api.getContributions(1, 20),
				api.getPortfolioStats(),
				api.getContributionProjects()
			]);
			contributions = contribs.contributions || [];
			contribTotal = contribs.total_count;
			contribPage = 1;
			portfolioStats = stats;
			projects = projs || [];
		} catch (e: any) {
			console.error(e);
		} finally {
			syncing = false;
		}
	}

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

	function startEditSkills() {
		editSkillsList = profileStats?.user.skills?.map(s => s.language) || [];
		editingSkills = true;
	}

	function cancelEditSkills() {
		editingSkills = false;
		newSkillInput = '';
	}

	function removeEditSkill(lang: string) {
		editSkillsList = editSkillsList.filter(s => s !== lang);
	}

	function addEditSkill() {
		const trimmed = newSkillInput.trim();
		if (trimmed && !editSkillsList.includes(trimmed)) {
			editSkillsList = [...editSkillsList, trimmed];
		}
		newSkillInput = '';
	}

	async function saveSkills() {
		savingSkills = true;
		try {
			const result = await api.setSkills(editSkillsList);
			if (profileStats) {
				profileStats = {
					...profileStats,
					user: { ...profileStats.user, skills: result.skills }
				};
			}
			editingSkills = false;
		} catch (e: any) {
			console.error('Failed to save skills:', e);
		} finally {
			savingSkills = false;
		}
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

	// Growth radar helpers
	const radarDimensions = ['volume', 'breadth', 'consistency', 'depth', 'diversity', 'recency'] as const;
	const radarLabels: Record<string, string> = { volume: 'Volume', breadth: 'Breadth', consistency: 'Consistency', depth: 'Depth', diversity: 'Diversity', recency: 'Recency' };
	const levelColors: Record<string, string> = { explorer: '#6b9a7e', first_light: '#60a5fa', contributor: '#4ade80', regular: '#fbbf24', specialist: '#c084fc', luminary: '#f472b6' };

	function radarPoint(index: number, value: number, cx = 100, cy = 100, maxR = 80): { x: number; y: number } {
		const angle = (Math.PI * 2 * index) / 6 - Math.PI / 2;
		const r = (value / 100) * maxR;
		return { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
	}

	function radarPolygon(scale: number): string {
		return Array.from({ length: 6 }, (_, i) => {
			const p = radarPoint(i, scale);
			return `${p.x},${p.y}`;
		}).join(' ');
	}

	function radarDataPolygon(radar: GrowthProfile['radar']): string {
		const vals = radarDimensions.map(d => radar[d]);
		return vals.map((v, i) => {
			const p = radarPoint(i, v);
			return `${p.x},${p.y}`;
		}).join(' ');
	}

	function radarDataDots(radar: GrowthProfile['radar']): { x: number; y: number }[] {
		return radarDimensions.map((d, i) => radarPoint(i, radar[d]));
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

{#if loading}
	<div class="profile-loading">
		<div class="spinner"></div>
		<p>loading profile...</p>
	</div>
{:else if error}
	<div class="profile-error">
		<p class="error-text">{error}</p>
	</div>
{:else if profileStats}
	<div class="profile-page">

		<!-- Header -->
		<header class="profile-header">
			<div class="header-left">
				<img src={profileStats.user.avatar_url} alt={profileStats.user.github_username} class="profile-avatar" />
				<div class="header-info">
					<h1 class="profile-name">{profileStats.user.github_username}</h1>
					{#if profileStats.user.bio}
						<p class="profile-bio">{profileStats.user.bio}</p>
					{/if}
					<div class="profile-meta">
						<a href="https://github.com/{profileStats.user.github_username}" target="_blank" rel="noopener" class="github-link">
							<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
							github.com/{profileStats.user.github_username}
						</a>
						{#if profileStats.user.comfort_level}
							<span class="meta-tag">{profileStats.user.comfort_level}</span>
						{/if}
						{#if profileStats.user.time_commitment}
							<span class="meta-tag">{profileStats.user.time_commitment}</span>
						{/if}
					</div>
				</div>
			</div>
			<button class="sync-btn" onclick={syncContributions} disabled={syncing}>
				{syncing ? 'syncing...' : 'sync contributions'}
			</button>
		</header>

		<!-- Stats Cards -->
		<div class="stats-grid">
			<div class="stat-card">
				<span class="stat-value" style="color: #c084fc">{portfolioStats?.total_prs ?? profileStats.merged_pr_count}</span>
				<span class="stat-label">contributions</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #4ade80">{portfolioStats?.total_repos ?? 0}</span>
				<span class="stat-label">projects</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #fbbf24">{portfolioStats?.current_streak ?? 0}</span>
				<span class="stat-label">day streak</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #60a5fa">{profileStats.user.skills?.length || 0}</span>
				<span class="stat-label">languages</span>
			</div>
		</div>

		<div class="profile-columns">
			<!-- Left Column: Growth + Skills + Goals + illuminate card -->
			<div class="profile-sidebar">

				<!-- Growth Engine -->
				{#if growth}
					<div class="profile-section growth-section">
						<h3 class="section-heading">// growth</h3>

						<!-- Level badge -->
						<div class="growth-level">
							<div class="level-badge" style="border-color: {levelColors[growth.level] || 'var(--amber)'}">
								<span class="level-num" style="color: {levelColors[growth.level] || 'var(--amber)'}">{growth.level_index + 1}</span>
							</div>
							<div class="level-info">
								<span class="level-name" style="color: {levelColors[growth.level] || 'var(--amber)'}">{growth.level_name}</span>
								<span class="level-rank">level {growth.level_index + 1} of 6</span>
							</div>
						</div>

						<!-- Progress bar -->
						{#if growth.next_level}
							<div class="growth-progress">
								<div class="progress-label">
									<span class="progress-metric">{growth.progress.current_value}/{growth.progress.target_value} {growth.progress.metric}</span>
									<span class="progress-next">next: {growth.next_level_name}</span>
								</div>
								<div class="progress-bar-bg">
									<div class="progress-bar-fill" style="width: {growth.progress.percentage}%; background: {levelColors[growth.level] || 'var(--amber)'}"></div>
								</div>
							</div>
						{:else}
							<div class="growth-progress">
								<span class="progress-metric max-level">max level reached</span>
							</div>
						{/if}

						<!-- Radar chart -->
						<div class="radar-container">
							<svg viewBox="0 0 200 200" class="radar-svg">
								<!-- Background rings -->
								{#each [25, 50, 75, 100] as scale}
									<polygon points={radarPolygon(scale)} class="radar-ring" />
								{/each}
								<!-- Axis lines -->
								{#each Array.from({ length: 6 }, (_, i) => i) as i}
									{@const p = radarPoint(i, 100)}
									<line x1="100" y1="100" x2={p.x} y2={p.y} class="radar-axis" />
								{/each}
								<!-- Data polygon -->
								<polygon points={radarDataPolygon(growth.radar)} class="radar-data" style="stroke: {levelColors[growth.level] || 'var(--amber)'}; fill: {levelColors[growth.level] || 'var(--amber)'}" />
								<!-- Data dots -->
								{#each radarDataDots(growth.radar) as dot}
									<circle cx={dot.x} cy={dot.y} r="3" class="radar-dot" style="fill: {levelColors[growth.level] || 'var(--amber)'}" />
								{/each}
								<!-- Labels -->
								{#each radarDimensions as dim, i}
									{@const p = radarPoint(i, 125)}
									<text x={p.x} y={p.y} class="radar-label" text-anchor="middle" dominant-baseline="middle">{radarLabels[dim]}</text>
								{/each}
							</svg>
						</div>

						<!-- Next steps -->
						{#if growth.next_steps.length > 0}
							<div class="growth-steps">
								<h4 class="steps-heading">next steps</h4>
								{#each growth.next_steps as step}
									<div class="step-item">
										<span class="step-arrow">&gt;</span>
										<div class="step-content">
											<span class="step-title">{step.title}</span>
											<span class="step-desc">{step.description}</span>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Skills -->
				<div class="profile-section">
					<div class="section-header-row">
						<h3 class="section-heading">// skills</h3>
						{#if !editingSkills}
							<button class="edit-skills-btn" onclick={startEditSkills}>edit</button>
						{/if}
					</div>
					{#if editingSkills}
						<div class="skills-edit">
							<div class="edit-chips">
								{#each editSkillsList as lang}
									<span class="edit-chip">
										{lang}
										<button class="chip-remove" onclick={() => removeEditSkill(lang)}>&times;</button>
									</span>
								{/each}
							</div>
							<div class="edit-add-row">
								<input
									type="text"
									placeholder="add language..."
									bind:value={newSkillInput}
									onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addEditSkill())}
									class="edit-skill-input"
								/>
								{#if newSkillInput.trim()}
									<button class="edit-add-btn" onclick={addEditSkill}>add</button>
								{/if}
							</div>
							<div class="edit-actions">
								<button class="edit-save-btn" onclick={saveSkills} disabled={savingSkills}>
									{savingSkills ? 'saving...' : 'save'}
								</button>
								<button class="edit-cancel-btn" onclick={cancelEditSkills}>cancel</button>
							</div>
						</div>
					{:else if profileStats.user.skills?.length}
						<div class="skills-list">
							{#each profileStats.user.skills as skill}
								<div class="skill-row">
									<div class="skill-info">
										<span class="skill-name"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}</span>
										<span class="skill-level">{skillLevel(skill.proficiency)}{skill.source === 'manual' ? ' *' : ''}</span>
									</div>
									<div class="skill-bar-bg">
										<div class="skill-bar-fill" style="width: {skill.proficiency}%; background: {skillColor(skill.language)}"></div>
									</div>
								</div>
							{/each}
						</div>
					{:else}
						<p class="no-skills-text">no skills detected. <button class="edit-skills-link" onclick={startEditSkills}>add manually</button></p>
					{/if}
				</div>

				<!-- Goals -->
				{#if profileStats.user.goals?.length}
					<div class="profile-section">
						<h3 class="section-heading">// goals</h3>
						<div class="goals-list">
							{#each profileStats.user.goals as goal}
								<div class="goal-item">&gt; {goal}</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Illuminate Card -->
				<div class="profile-section illuminate-card">
					<div class="card-header">
						<span class="card-logo">illuminate<span class="blink">_</span></span>
						<span class="card-badge">contributor</span>
					</div>
					<div class="card-user">
						<img src={profileStats.user.avatar_url} alt="" class="card-avatar" />
						<div>
							<div class="card-username">{profileStats.user.github_username}</div>
							{#if profileStats.user.bio}
								<div class="card-bio">{profileStats.user.bio}</div>
							{/if}
						</div>
					</div>
					<div class="card-stats">
						<div class="card-stat">
							<span class="card-stat-val">{portfolioStats?.total_prs ?? 0}</span>
							<span class="card-stat-lbl">merged</span>
						</div>
						<div class="card-stat">
							<span class="card-stat-val">{portfolioStats?.total_repos ?? 0}</span>
							<span class="card-stat-lbl">repos</span>
						</div>
						<div class="card-stat">
							<span class="card-stat-val">{profileStats.user.skills?.length || 0}</span>
							<span class="card-stat-lbl">langs</span>
						</div>
					</div>
					{#if profileStats.user.skills?.length}
						<div class="card-skills">
							{#each profileStats.user.skills.slice(0, 5) as skill}
								<span class="card-skill-tag"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}</span>
							{/each}
						</div>
					{/if}
					<div class="card-footer">
						<span class="card-watermark">illuminate.dev</span>
					</div>
				</div>
			</div>

			<!-- Right Column: Portfolio Tabs -->
			<div class="profile-main">
				<div class="tab-bar">
					<button class="tab-btn" class:active={activeTab === 'timeline'} onclick={() => switchTab('timeline')}>timeline</button>
					<button class="tab-btn" class:active={activeTab === 'projects'} onclick={() => switchTab('projects')}>projects</button>
					<button class="tab-btn" class:active={activeTab === 'stats'} onclick={() => switchTab('stats')}>stats</button>
				</div>

				<!-- Timeline Tab -->
				{#if activeTab === 'timeline'}
					<div class="tab-content">
						{#if contributions.length === 0}
							<div class="tab-empty">
								<p>no contributions synced yet.</p>
								<p class="hint">click "sync contributions" above to fetch your merged PRs from GitHub.</p>
							</div>
						{:else}
							<div class="timeline-list">
								{#each contributions as c}
									<a href={c.pr_url} target="_blank" rel="noopener" class="timeline-item">
										<div class="tl-icon">
											<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M5 3.254V3.25v.005a.75.75 0 110-.005v.004zm.45 1.9a2.25 2.25 0 10-1.95.218v5.256a2.25 2.25 0 101.5 0V7.123A5.735 5.735 0 009.25 9h1.378a2.251 2.251 0 100-1.5H9.25a4.25 4.25 0 01-3.8-2.346zM12.75 9a.75.75 0 100-1.5.75.75 0 000 1.5zm-8.5 4.5a.75.75 0 100-1.5.75.75 0 000 1.5z"/></svg>
										</div>
										<div class="tl-content">
											<div class="tl-title">{c.pr_title}</div>
											<div class="tl-meta">
												<span class="tl-repo">{c.repo_owner}/{c.repo_name}</span>
												<span class="tl-sep">·</span>
												<span class="tl-number">#{c.pr_number}</span>
												<span class="tl-sep">·</span>
												<span class="tl-time">{timeAgo(c.merged_at || c.created_at)}</span>
											</div>
										</div>
										{#if c.language}
											<span class="tl-lang" style="color: {skillColor(c.language)}">{c.language}</span>
										{/if}
									</a>
								{/each}
							</div>
							{#if contributions.length < contribTotal}
								<div class="load-more">
									<button class="load-more-btn" onclick={loadMoreContributions} disabled={tabLoading}>
										{tabLoading ? 'loading...' : 'load more'}
									</button>
									<span class="load-count">{contributions.length} of {contribTotal}</span>
								</div>
							{/if}
						{/if}
					</div>
				{/if}

				<!-- Projects Tab -->
				{#if activeTab === 'projects'}
					<div class="tab-content">
						{#if tabLoading}
							<div class="tab-loading"><div class="spinner-sm"></div></div>
						{:else if projects.length === 0}
							<div class="tab-empty">
								<p>no projects yet.</p>
								<p class="hint">sync your contributions to see projects you've contributed to.</p>
							</div>
						{:else}
							<div class="projects-grid">
								{#each projects as project}
									<a href="https://github.com/{project.repo_owner}/{project.repo_name}" target="_blank" rel="noopener" class="project-card">
										<div class="project-header">
											<span class="project-name">{project.repo_owner}/<strong>{project.repo_name}</strong></span>
											{#if project.language}
												<span class="project-lang" style="color: {skillColor(project.language)}"><span class="skill-dot" style="background: {skillColor(project.language)}"></span>{project.language}</span>
											{/if}
										</div>
										<div class="project-stats">
											<span class="project-prs">{project.pr_count} PR{project.pr_count !== 1 ? 's' : ''}</span>
											{#if project.latest_at}
												<span class="project-time">last: {timeAgo(project.latest_at)}</span>
											{/if}
										</div>
									</a>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Stats Tab -->
				{#if activeTab === 'stats'}
					<div class="tab-content">
						{#if !portfolioStats || portfolioStats.total_prs === 0}
							<div class="tab-empty">
								<p>no stats available yet.</p>
								<p class="hint">sync your contributions to generate statistics.</p>
							</div>
						{:else}
							<div class="stats-section">
								<h3 class="section-heading">// overview</h3>
								<div class="overview-grid">
									<div class="overview-item">
										<span class="overview-val">{portfolioStats.total_prs}</span>
										<span class="overview-lbl">total PRs</span>
									</div>
									<div class="overview-item">
										<span class="overview-val">{portfolioStats.total_repos}</span>
										<span class="overview-lbl">repos</span>
									</div>
									<div class="overview-item">
										<span class="overview-val">{portfolioStats.current_streak}</span>
										<span class="overview-lbl">current streak</span>
									</div>
									<div class="overview-item">
										<span class="overview-val">{portfolioStats.longest_streak}</span>
										<span class="overview-lbl">longest streak</span>
									</div>
								</div>
								{#if portfolioStats.first_contribution}
									<div class="date-range">
										<span>first contribution: {formatDate(portfolioStats.first_contribution)}</span>
										<span>latest: {formatDate(portfolioStats.latest_contribution)}</span>
									</div>
								{/if}
							</div>

							{#if Object.keys(portfolioStats.languages).length}
								<div class="stats-section">
									<h3 class="section-heading">// languages</h3>
									<div class="lang-bar">
										{#each langPercent(portfolioStats.languages) as lang}
											{#if lang.pct >= 2}
												<div class="lang-bar-seg" style="width: {lang.pct}%; background: {lang.color}" title="{lang.name}: {lang.pct}%"></div>
											{/if}
										{/each}
									</div>
									<div class="lang-list">
										{#each langPercent(portfolioStats.languages) as lang}
											<div class="lang-item">
												<span class="lang-dot" style="background: {lang.color}"></span>
												<span class="lang-name">{lang.name}</span>
												<span class="lang-count">{lang.count}</span>
												<span class="lang-pct">{lang.pct}%</span>
											</div>
										{/each}
									</div>
								</div>
							{/if}
						{/if}
					</div>
				{/if}
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

	.sync-btn {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.4rem 1rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--amber);
		cursor: pointer;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.sync-btn:hover:not(:disabled) { background: var(--amber-glow); border-color: var(--amber-dim); }
	.sync-btn:disabled { opacity: 0.4; cursor: not-allowed; }

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

	.card-stat { text-align: center; }

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

	.card-footer { text-align: right; }

	.card-watermark {
		font-size: 0.6rem;
		color: var(--text-dim);
		letter-spacing: 0.06em;
	}

	/* Tabs */
	.tab-bar {
		display: flex;
		gap: 0;
		border-bottom: 1px solid var(--border);
		margin-bottom: 1rem;
	}

	.tab-btn {
		font-family: var(--font-mono);
		font-size: 0.78rem;
		padding: 0.5rem 1rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.tab-btn:hover { color: var(--text); }
	.tab-btn.active {
		color: var(--amber);
		border-bottom-color: var(--amber);
	}

	.tab-content {
		animation: fade-in 0.2s ease;
	}

	.tab-loading {
		display: flex;
		justify-content: center;
		padding: 2rem;
	}

	.tab-empty {
		text-align: center;
		padding: 2rem;
		color: var(--text-dim);
		font-size: 0.82rem;
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-dim);
		margin-top: 0.35rem;
	}

	/* Timeline */
	.timeline-list {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.timeline-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 0.65rem 0.5rem;
		border-bottom: 1px solid var(--border);
		text-decoration: none;
		transition: background 0.15s;
	}

	.timeline-item:hover { background: var(--amber-glow); }
	.timeline-item:last-child { border-bottom: none; }

	.tl-icon {
		width: 26px;
		height: 26px;
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

	.tl-content {
		flex: 1;
		min-width: 0;
	}

	.tl-title {
		font-size: 0.82rem;
		color: var(--text-bright);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tl-meta {
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-top: 0.15rem;
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.tl-repo { color: var(--text-muted); }
	.tl-sep { opacity: 0.4; }
	.tl-number { color: var(--amber-dim); }

	.tl-lang {
		font-size: 0.65rem;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.load-more {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.load-more-btn {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.3rem 0.8rem;
		background: none;
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--amber);
		cursor: pointer;
		transition: all 0.15s;
	}

	.load-more-btn:hover:not(:disabled) { border-color: var(--amber-dim); background: var(--amber-glow); }
	.load-more-btn:disabled { opacity: 0.4; cursor: not-allowed; }

	.load-count {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	/* Projects */
	.projects-grid {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.project-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.875rem 1rem;
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
		margin-bottom: 0.35rem;
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
		display: flex;
		align-items: center;
		gap: 0.25rem;
		flex-shrink: 0;
	}

	.project-stats {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.7rem;
	}

	.project-prs { color: var(--amber); font-weight: 600; }
	.project-time { color: var(--text-dim); }

	/* Stats tab */
	.stats-section {
		margin-bottom: 1.5rem;
	}

	.overview-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.overview-item {
		text-align: center;
		padding: 0.75rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.overview-val {
		display: block;
		font-size: 1.25rem;
		font-weight: 700;
		color: var(--text-bright);
	}

	.overview-lbl {
		font-size: 0.65rem;
		color: var(--text-dim);
		text-transform: lowercase;
	}

	.date-range {
		display: flex;
		justify-content: space-between;
		font-size: 0.7rem;
		color: var(--text-dim);
	}

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
		transition: width 0.6s ease;
	}

	.lang-list {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.lang-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.78rem;
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

	.lang-count {
		color: var(--text-dim);
		font-size: 0.7rem;
	}

	.lang-pct {
		color: var(--text-muted);
		font-size: 0.7rem;
		font-weight: 600;
		min-width: 32px;
		text-align: right;
	}

	/* Skills editing */
	.section-header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.section-header-row .section-heading {
		margin-bottom: 0;
	}

	.edit-skills-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		font-family: var(--font-mono);
		font-size: 0.65rem;
		cursor: pointer;
		transition: color 0.15s;
		padding: 0;
	}

	.edit-skills-btn:hover { color: var(--amber); }

	.skills-edit {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.edit-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.edit-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.72rem;
		padding: 0.2rem 0.45rem;
		background: var(--amber-glow);
		border: 1px solid var(--amber-dim);
		border-radius: 3px;
		color: var(--amber);
	}

	.chip-remove {
		background: none;
		border: none;
		color: var(--text-dim);
		font-size: 0.85rem;
		cursor: pointer;
		padding: 0;
		line-height: 1;
		transition: color 0.15s;
	}

	.chip-remove:hover { color: var(--red); }

	.edit-add-row {
		display: flex;
		gap: 0.4rem;
	}

	.edit-skill-input {
		flex: 1;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.35rem 0.5rem;
		border-radius: 4px;
		outline: none;
		transition: border-color 0.15s;
	}

	.edit-skill-input::placeholder { color: var(--text-dim); }
	.edit-skill-input:focus { border-color: var(--amber-dim); }

	.edit-add-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.35rem 0.5rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.edit-add-btn:hover { background: var(--amber-glow); border-color: var(--amber-dim); }

	.edit-actions {
		display: flex;
		gap: 0.4rem;
	}

	.edit-save-btn {
		flex: 1;
		background: var(--amber);
		color: var(--bg);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		font-weight: 600;
		padding: 0.35rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		transition: background 0.15s;
	}

	.edit-save-btn:hover { background: var(--amber-bright); }
	.edit-save-btn:disabled { opacity: 0.5; cursor: not-allowed; }

	.edit-cancel-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.35rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.edit-cancel-btn:hover { border-color: var(--text-dim); color: var(--text); }

	.no-skills-text {
		font-size: 0.78rem;
		color: var(--text-dim);
	}

	.edit-skills-link {
		background: none;
		border: none;
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.78rem;
		cursor: pointer;
		text-decoration: underline;
		padding: 0;
	}

	/* Growth Engine */
	.growth-section {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 1.25rem;
	}

	.growth-level {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.level-badge {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		border: 2px solid var(--amber);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.level-num {
		font-size: 1.1rem;
		font-weight: 700;
	}

	.level-info {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.level-name {
		font-size: 0.9rem;
		font-weight: 700;
	}

	.level-rank {
		font-size: 0.65rem;
		color: var(--text-dim);
	}

	.growth-progress {
		margin-bottom: 1rem;
	}

	.progress-label {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.35rem;
	}

	.progress-metric {
		font-size: 0.7rem;
		color: var(--text-muted);
	}

	.progress-metric.max-level {
		color: var(--amber);
		font-weight: 600;
		font-size: 0.72rem;
	}

	.progress-next {
		font-size: 0.65rem;
		color: var(--text-dim);
	}

	.progress-bar-bg {
		height: 4px;
		background: var(--border);
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-bar-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.6s ease;
	}

	.radar-container {
		margin: 0 auto 1rem;
		max-width: 220px;
	}

	.radar-svg {
		width: 100%;
		height: auto;
	}

	.radar-ring {
		fill: none;
		stroke: var(--border);
		stroke-width: 0.5;
	}

	.radar-axis {
		stroke: var(--border);
		stroke-width: 0.3;
		stroke-dasharray: 2 2;
	}

	.radar-data {
		fill-opacity: 0.15;
		stroke-width: 1.5;
	}

	.radar-dot {
		stroke: var(--bg-card);
		stroke-width: 1;
	}

	.radar-label {
		font-size: 8px;
		fill: var(--text-dim);
		font-family: var(--font-mono);
	}

	.growth-steps {
		border-top: 1px solid var(--border);
		padding-top: 0.75rem;
	}

	.steps-heading {
		font-size: 0.68rem;
		color: var(--text-dim);
		margin-bottom: 0.5rem;
		font-weight: 600;
		letter-spacing: 0.04em;
	}

	.step-item {
		display: flex;
		gap: 0.5rem;
		padding: 0.4rem 0;
	}

	.step-arrow {
		color: var(--amber);
		font-weight: 700;
		font-size: 0.75rem;
		flex-shrink: 0;
		margin-top: 1px;
	}

	.step-content {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.step-title {
		font-size: 0.75rem;
		color: var(--text);
		font-weight: 500;
	}

	.step-desc {
		font-size: 0.68rem;
		color: var(--text-dim);
		line-height: 1.35;
	}

	/* Responsive */
	@media (max-width: 800px) {
		.stats-grid { grid-template-columns: repeat(2, 1fr); }
		.overview-grid { grid-template-columns: repeat(2, 1fr); }
		.profile-columns { grid-template-columns: 1fr; }
		.profile-header { flex-direction: column; gap: 1rem; }
		.header-left { flex-direction: column; align-items: center; text-align: center; }
		.profile-meta { justify-content: center; }
		.sync-btn { align-self: center; }
		.date-range { flex-direction: column; gap: 0.25rem; }
	}
</style>
