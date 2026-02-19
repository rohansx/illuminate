<script lang="ts">
	import { api, type Issue, type DeepDive, type GitHubComment } from '$lib/api';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { marked } from 'marked';

	let issue = $state<Issue | null>(null);
	let loading = $state(true);
	let error = $state('');

	let deepDive = $state<DeepDive | null>(null);
	let deepDiveLoading = $state(false);
	let deepDiveError = $state('');

	let isSaved = $state(false);
	let savingInProgress = $state(false);

	let comments = $state<GitHubComment[]>([]);
	let commentsLoading = $state(false);
	let showAllSkills = $state(false);
	const MAX_SKILLS = 6;

	// Configure marked for safe rendering
	marked.setOptions({
		breaks: true,
		gfm: true
	});

	function md(text: string): string {
		return marked.parse(text, { async: false }) as string;
	}

	function timeAgo(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return 'just now';
		const minutes = Math.floor(seconds / 60);
		if (minutes < 60) return `${minutes}m ago`;
		const hours = Math.floor(minutes / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		if (days < 30) return `${days}d ago`;
		const months = Math.floor(days / 30);
		return `${months}mo ago`;
	}

	onMount(async () => {
		try {
			issue = await api.getIssue(page.params.id!);

			// Check saved status
			api.isIssueSaved(page.params.id!).then(res => { isSaved = res.saved; }).catch(() => {});

			// Load comments if there are any
			if (issue && issue.comment_count > 0) {
				commentsLoading = true;
				api.getIssueComments(issue.id)
					.then(c => { comments = c; })
					.catch(() => {})
					.finally(() => { commentsLoading = false; });
			}
		} catch (e: any) {
			error = e.message || 'Failed to load issue';
		} finally {
			loading = false;
		}
	});

	async function toggleSave() {
		if (!issue) return;
		savingInProgress = true;
		try {
			if (isSaved) {
				await api.unsaveIssue(issue.id);
				isSaved = false;
			} else {
				await api.saveIssue(issue.id);
				isSaved = true;
			}
		} catch (e) {
			console.error('Save toggle failed:', e);
		} finally {
			savingInProgress = false;
		}
	}

	async function generateDeepDive() {
		if (!issue) return;
		deepDiveLoading = true;
		deepDiveError = '';
		try {
			deepDive = await api.getDeepDive(issue.id);
		} catch (e: any) {
			deepDiveError = e.message || 'Failed to generate deep dive';
		} finally {
			deepDiveLoading = false;
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

	function freshnessColor(score: number): string {
		if (score >= 0.7) return '#4ade80';
		if (score >= 0.4) return '#fbbf24';
		return '#f87171';
	}

	const labelColors: Record<string, { bg: string; fg: string }> = {
		green: { bg: 'rgba(74, 222, 128, 0.12)', fg: '#4ade80' },
		blue: { bg: 'rgba(96, 165, 250, 0.12)', fg: '#60a5fa' },
		purple: { bg: 'rgba(192, 132, 252, 0.12)', fg: '#c084fc' },
		red: { bg: 'rgba(248, 113, 113, 0.12)', fg: '#f87171' },
		yellow: { bg: 'rgba(251, 191, 36, 0.12)', fg: '#fbbf24' },
		cyan: { bg: 'rgba(103, 232, 249, 0.12)', fg: '#67e8f9' },
		default: { bg: 'var(--amber-glow)', fg: 'var(--amber)' },
	};

	function getLabelColor(label: string): { bg: string; fg: string } {
		const l = label.toLowerCase();
		if (l.includes('good first issue') || l.includes('beginner') || l.includes('easy')) return labelColors.green;
		if (l.includes('help wanted') || l.includes('contributions')) return labelColors.cyan;
		if (l.includes('bug') || l.includes('fix')) return labelColors.red;
		if (l.includes('type:') || l.includes('documentation') || l.includes('docs') || l.includes('cleanup')) return labelColors.blue;
		if (l.includes('team-') || l.includes('team:')) return labelColors.purple;
		if (l.startsWith('p0') || l.startsWith('p1')) return labelColors.red;
		if (l.startsWith('p2') || l.startsWith('p3')) return labelColors.yellow;
		if (l.includes('feature') || l.includes('enhancement')) return labelColors.green;
		return labelColors.default;
	}

	const skillPalette = ['#4ade80', '#60a5fa', '#c084fc', '#f472b6', '#fbbf24', '#67e8f9', '#fb923c', '#a78bfa'];
	function skillColor(name: string): string {
		let hash = 0;
		for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
		return skillPalette[Math.abs(hash) % skillPalette.length];
	}

	const ddSections = $derived(deepDive ? [
		{ num: '01', title: 'project overview', content: deepDive.project_overview, variant: '' },
		{ num: '02', title: 'issue context', content: deepDive.issue_context, variant: '' },
		{ num: '03', title: 'suggested approach', content: deepDive.suggested_approach, variant: '' },
		{ num: '04', title: 'questions to ask', content: deepDive.questions_to_ask, variant: '' },
		{ num: '05', title: 'red flags', content: deepDive.red_flags, variant: 'red' },
	] : []);
</script>

{#if loading}
	<div class="loading">
		<div class="spinner"></div>
	</div>
{:else if error}
	<div class="error-state">
		<p>{error}</p>
		<a href="/app/feed">&larr; back to feed</a>
	</div>
{:else if issue}
	<div class="page">
		<!-- Breadcrumb -->
		<nav class="breadcrumb">
			<a href="/app/feed">feed</a>
			<span class="sep">/</span>
			<span class="repo-path">{issue.repo?.owner}/{issue.repo?.name}</span>
			<span class="sep">/</span>
			<span class="current">#{issue.number}</span>
		</nav>

		<!-- Title block -->
		<header class="issue-header">
			<span class="status-badge"><span class="status-dot"></span>open</span>
			<h1>{issue.title}</h1>
			<div class="issue-number">#{issue.number}</div>
		</header>

		<!-- Two-column layout -->
		<div class="columns">
			<!-- Main content -->
			<div class="main-col">
				<!-- Description + Deep Dive Trigger/Loading/Error -->
				<section class="card">
					<div class="card-label">description</div>
					<div class="prose">
						{@html md(issue.body || '*No description provided.*')}
					</div>

					{#if !deepDive && !deepDiveLoading && !deepDiveError}
						<div class="dd-trigger-wrap">
							<button class="dd-trigger" onclick={generateDeepDive}>
								<span class="dd-trigger-icon">&#9672;</span>
								<span class="dd-trigger-text">
									<span class="dd-trigger-title">generate deep dive</span>
									<span class="dd-trigger-sub">ai-powered analysis tailored to your skill level</span>
								</span>
								<span class="dd-trigger-arrow">&rarr;</span>
							</button>
						</div>
					{:else if deepDiveLoading}
						<div class="dd-inline-loading">
							<div class="dd-loading-bar">
								<div class="dd-loading-progress"></div>
							</div>
							<div class="dd-loading-steps">
								<span class="dd-step active">
									<span class="dd-step-dot"></span>
									fetching context
								</span>
								<span class="dd-step">
									<span class="dd-step-dot"></span>
									building prompt
								</span>
								<span class="dd-step">
									<span class="dd-step-dot"></span>
									generating analysis
								</span>
							</div>
							<p class="dd-loading-hint">this usually takes 10-20 seconds</p>
						</div>
					{:else if deepDiveError}
						<div class="dd-inline-error">
							<span class="dd-error-icon">&#9888;</span>
							<span class="dd-error-msg">{deepDiveError}</span>
							<button class="btn btn-ghost" onclick={generateDeepDive}>retry</button>
						</div>
					{/if}
				</section>

				<!-- Deep Dive Results -->
				{#if deepDive}
					<section class="deep-dive-section">
						<div class="dd-header-bar">
							<span class="dd-title">deep dive<span class="cursor">_</span></span>
							<span class="dd-meta">{deepDive.model_used} &middot; {deepDive.prompt_tokens + deepDive.completion_tokens} tokens</span>
						</div>

						<div class="dd-grid">
							{#each ddSections as section, i}
								<div class="dd-card" class:dd-card-red={section.variant === 'red'} style="animation-delay: {i * 60}ms">
									<div class="dd-card-num" class:dd-num-red={section.variant === 'red'}>{section.num}</div>
									<h3 class="dd-card-title" class:dd-title-red={section.variant === 'red'}>{section.title}</h3>
									<div class="prose prose-sm">
										{@html md(section.content)}
									</div>
								</div>
							{/each}
						</div>
					</section>
				{/if}

				<!-- Comments -->
				{#if issue.comment_count > 0}
					<section class="card">
						<div class="card-label">discussion ({issue.comment_count})</div>
						{#if commentsLoading}
							<div class="comments-loading">
								<div class="spinner"></div>
								<span>loading comments...</span>
							</div>
						{:else if comments.length}
							<div class="comment-list">
								{#each comments as comment}
									<div class="comment">
										<div class="comment-header">
											<img src={comment.user.avatar_url} alt={comment.user.login} class="comment-avatar" />
											<span class="comment-author">{comment.user.login}</span>
											<span class="comment-time">{timeAgo(comment.created_at)}</span>
										</div>
										<div class="prose prose-sm comment-body">
											{@html md(comment.body)}
										</div>
									</div>
								{/each}
							</div>
						{:else}
							<p class="no-comments">failed to load comments</p>
						{/if}
					</section>
				{/if}
			</div>

			<!-- Sidebar -->
			<aside class="side-col">
				<!-- Actions -->
				<div class="sidebar-card sidebar-actions">
					<a
						href="https://github.com/{issue.repo?.owner}/{issue.repo?.name}/issues/{issue.number}"
						target="_blank"
						rel="noopener noreferrer"
						class="btn btn-primary"
					>
						open on github &rarr;
					</a>
					<button
						class="btn btn-save"
						class:btn-saved={isSaved}
						onclick={toggleSave}
						disabled={savingInProgress}
					>
						{#if savingInProgress}
							<span class="save-spinner"></span>
						{:else if isSaved}
							&#9733; saved
						{:else}
							&#9734; save issue
						{/if}
					</button>
				</div>

				<!-- At a glance -->
				<div class="sidebar-card">
					<div class="sidebar-label">at a glance</div>
					<div class="stat-grid">
						<div class="stat">
							<span class="stat-value"><span class="stat-dot" style="background: {difficultyColor(issue.difficulty)}"></span>{difficultyLabel(issue.difficulty)}</span>
							<span class="stat-key">difficulty</span>
						</div>
						<div class="stat">
							<span class="stat-value"><span class="stat-dot" style="background: #60a5fa"></span>{issue.time_estimate}</span>
							<span class="stat-key">estimate</span>
						</div>
						<div class="stat">
							<span class="stat-value"><span class="stat-dot" style="background: #c084fc"></span>{issue.comment_count}</span>
							<span class="stat-key">comments</span>
						</div>
						<div class="stat">
							<span class="stat-value"><span class="stat-dot" style="background: {freshnessColor(issue.freshness_score)}"></span>{Math.round(issue.freshness_score * 100)}%</span>
							<span class="stat-key">freshness</span>
						</div>
					</div>
				</div>

				<!-- Labels -->
				{#if issue.labels?.length}
					<div class="sidebar-card">
						<div class="sidebar-label">labels</div>
						<div class="tag-list">
							{#each issue.labels as label}
								<span class="tag" style="background: {getLabelColor(label).bg}; color: {getLabelColor(label).fg}">{label}</span>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Skills -->
				{#if issue.skills?.length}
					<div class="sidebar-card">
						<div class="sidebar-label">skills needed</div>
						<div class="tag-list">
							{#each (showAllSkills ? issue.skills : issue.skills.slice(0, MAX_SKILLS)) as skill}
								<span class="tag tag-outline"><span class="skill-dot" style="background: {skillColor(skill.language)}"></span>{skill.language}{skill.framework ? ` / ${skill.framework}` : ''}</span>
							{/each}
							{#if issue.skills.length > MAX_SKILLS}
								<button class="tag tag-more" onclick={() => showAllSkills = !showAllSkills}>
									{showAllSkills ? 'show less' : `+${issue.skills.length - MAX_SKILLS} more`}
								</button>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Match reasons -->
				{#if issue.match_reasons?.length}
					<div class="sidebar-card">
						<div class="sidebar-label">why this matches you</div>
						<ul class="reason-list">
							{#each issue.match_reasons as reason}
								<li>{reason}</li>
							{/each}
						</ul>
					</div>
				{/if}

				<!-- Repo info -->
				{#if issue.repo}
					<div class="sidebar-card">
						<div class="sidebar-label">repository</div>
						<div class="repo-info">
							<div class="repo-name">{issue.repo.owner}/{issue.repo.name}</div>
							{#if issue.repo.description}
								<p class="repo-desc">{issue.repo.description}</p>
							{/if}
							<div class="repo-stats">
								{#if issue.repo.stars}
									<span class="repo-stat-item"><span class="star-icon">&#9733;</span> {issue.repo.stars.toLocaleString()}</span>
								{/if}
								{#if issue.repo.primary_language}
									<span class="repo-stat-item"><span class="lang-dot" style="background: {skillColor(issue.repo.primary_language)}"></span>{issue.repo.primary_language}</span>
								{/if}
							</div>
						</div>
					</div>
				{/if}
			</aside>
		</div>
	</div>
{/if}

<style>
	/* ── Page ── */
	.page {
		max-width: 1200px;
		margin: 0 auto;
		animation: fadeIn 0.3s ease;
	}

	@keyframes fadeIn {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}

	/* ── Loading / Error ── */
	.loading {
		display: flex;
		justify-content: center;
		padding: 4rem;
	}

	.spinner {
		width: 20px;
		height: 20px;
		border: 2px solid var(--border);
		border-top-color: var(--amber);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }

	.error-state {
		text-align: center;
		padding: 4rem;
		color: var(--red);
	}

	.error-state a {
		color: var(--amber);
		display: inline-block;
		margin-top: 1rem;
	}

	/* ── Breadcrumb ── */
	.breadcrumb {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-dim);
		margin-bottom: 1.25rem;
		flex-wrap: wrap;
	}

	.breadcrumb a {
		color: var(--text-muted);
		text-decoration: none;
		transition: color 0.15s;
	}

	.breadcrumb a:hover { color: var(--amber); }
	.sep { opacity: 0.4; }
	.repo-path { color: var(--text-muted); word-break: break-all; }
	.current { color: var(--text); }

	/* ── Header ── */
	.issue-header {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		margin-bottom: 1.75rem;
		padding-bottom: 1.25rem;
		border-bottom: 1px solid var(--border);
		flex-wrap: wrap;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.7rem;
		font-weight: 600;
		color: #4ade80;
		background: rgba(74, 222, 128, 0.1);
		border: 1px solid rgba(74, 222, 128, 0.2);
		padding: 0.2rem 0.6rem;
		border-radius: 99px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		flex-shrink: 0;
		margin-top: 0.25rem;
	}

	.status-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #4ade80;
		animation: pulse-dot 2s ease-in-out infinite;
	}

	@keyframes pulse-dot {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.4; }
	}

	.issue-header h1 {
		font-size: 1.3rem;
		font-weight: 600;
		color: var(--text-bright);
		line-height: 1.4;
		flex: 1;
		min-width: 200px;
		word-break: break-word;
	}

	.issue-number {
		font-size: 1.3rem;
		color: var(--text-dim);
		font-weight: 400;
		flex-shrink: 0;
	}

	/* ── Columns ── */
	.columns {
		display: grid;
		grid-template-columns: 1fr 300px;
		gap: 1.75rem;
		align-items: start;
	}

	@media (max-width: 900px) {
		.columns {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
		.side-col {
			order: -1;
		}
	}

	/* ── Main column ── */
	.main-col {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	/* ── Card ── */
	.card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.5rem;
		overflow: hidden;
	}

	.card-label {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.15em;
		color: var(--text-dim);
		margin-bottom: 1rem;
		font-weight: 500;
	}

	/* ── Prose (markdown) ── */
	.prose {
		overflow-wrap: break-word;
		word-break: break-word;
		min-width: 0;
	}

	.prose :global(h1),
	.prose :global(h2),
	.prose :global(h3),
	.prose :global(h4) {
		color: var(--text-bright);
		font-weight: 600;
		margin-top: 1.25rem;
		margin-bottom: 0.5rem;
		line-height: 1.4;
	}

	.prose :global(h1) { font-size: 1.1rem; }
	.prose :global(h2) { font-size: 1rem; }
	.prose :global(h3) { font-size: 0.9rem; }
	.prose :global(h4) { font-size: 0.85rem; }

	.prose :global(p) {
		font-size: 0.85rem;
		color: var(--text);
		line-height: 1.75;
		margin-bottom: 0.75rem;
	}

	.prose :global(ul),
	.prose :global(ol) {
		font-size: 0.85rem;
		color: var(--text);
		line-height: 1.75;
		margin-bottom: 0.75rem;
		padding-left: 1.25rem;
	}

	.prose :global(li) {
		margin-bottom: 0.25rem;
	}

	.prose :global(li)::marker {
		color: var(--text-dim);
	}

	.prose :global(code) {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		padding: 0.1rem 0.35rem;
		border-radius: 3px;
		color: var(--amber);
		word-break: break-all;
	}

	.prose :global(pre) {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 1rem;
		overflow-x: auto;
		margin-bottom: 0.75rem;
	}

	.prose :global(pre code) {
		background: none;
		border: none;
		padding: 0;
		font-size: 0.8rem;
		color: var(--text);
		word-break: normal;
	}

	.prose :global(blockquote) {
		border-left: 3px solid var(--amber-dim);
		padding-left: 1rem;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}

	.prose :global(a) {
		color: var(--amber);
		text-decoration: underline;
		text-decoration-color: var(--amber-dim);
		text-underline-offset: 2px;
		word-break: break-all;
	}

	.prose :global(a:hover) {
		color: var(--amber-bright);
	}

	.prose :global(hr) {
		border: none;
		border-top: 1px solid var(--border);
		margin: 1rem 0;
	}

	.prose :global(img) {
		max-width: 100%;
		border-radius: 4px;
		border: 1px solid var(--border);
		height: auto;
	}

	/* Table overflow wrapper */
	.prose {
		overflow-x: auto;
	}

	.prose :global(table) {
		width: max-content;
		min-width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
		margin-bottom: 0.75rem;
	}

	.prose :global(th),
	.prose :global(td) {
		border: 1px solid var(--border);
		padding: 0.4rem 0.6rem;
		text-align: left;
		white-space: nowrap;
	}

	.prose :global(th) {
		background: var(--bg-card);
		color: var(--text-bright);
		font-weight: 600;
	}

	.prose-sm :global(p),
	.prose-sm :global(ul),
	.prose-sm :global(ol) {
		font-size: 0.82rem;
	}

	.prose :global(*:first-child) {
		margin-top: 0;
	}

	.prose :global(*:last-child) {
		margin-bottom: 0;
	}

	/* ── Sidebar ── */
	.side-col {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		position: sticky;
		top: 60px;
	}

	@media (max-width: 900px) {
		.side-col {
			position: static;
		}
	}

	.sidebar-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem;
	}

	.sidebar-label {
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 0.15em;
		color: var(--text-dim);
		margin-bottom: 0.75rem;
		font-weight: 500;
	}

	.sidebar-actions {
		background: transparent;
		border: none;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.btn-save {
		width: 100%;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		color: var(--text-muted);
		padding: 0.6rem 1rem;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.btn-save:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	.btn-save:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn-saved {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	.btn-saved:hover {
		border-color: var(--red);
		color: var(--red);
	}

	.save-spinner {
		display: inline-block;
		width: 12px;
		height: 12px;
		border: 2px solid var(--border);
		border-top-color: var(--amber);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	/* ── Buttons ── */
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 600;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		text-decoration: none;
		gap: 0.5rem;
	}

	.btn-primary {
		width: 100%;
		background: var(--amber);
		color: var(--bg);
		padding: 0.6rem 1rem;
		border: none;
	}

	.btn-primary:hover {
		background: var(--amber-bright);
		color: var(--bg);
	}

	.btn-ghost {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		padding: 0.375rem 0.75rem;
	}

	.btn-ghost:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	/* ── Stat grid ── */
	.stat-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.stat-value {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-bright);
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.stat-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.stat-key {
		font-size: 0.65rem;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	/* ── Tags ── */
	.tag-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.tag {
		font-size: 0.7rem;
		padding: 0.2rem 0.5rem;
		border-radius: 3px;
	}

	.tag-amber {
		background: var(--amber-glow);
		color: var(--amber);
	}

	.tag-outline {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
	}

	.skill-dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tag-more {
		background: transparent;
		border: 1px dashed var(--border);
		color: var(--text-dim);
		cursor: pointer;
		font-family: var(--font-mono);
		transition: all 0.15s;
	}

	.tag-more:hover {
		border-color: var(--amber-dim);
		color: var(--amber);
	}

	/* ── Reason list ── */
	.reason-list {
		list-style: none;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.reason-list li {
		font-size: 0.75rem;
		color: var(--green);
		padding: 0.25rem 0.5rem;
		background: rgba(74, 222, 128, 0.08);
		border-radius: 3px;
		line-height: 1.5;
	}

	/* ── Repo info ── */
	.repo-info {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.repo-name {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-bright);
		word-break: break-all;
	}

	.repo-desc {
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.6;
	}

	.repo-stats {
		display: flex;
		gap: 0.75rem;
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-top: 0.2rem;
	}

	.repo-stat-item {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.star-icon {
		color: #fbbf24;
	}

	.lang-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	/* ── Deep Dive Trigger (inside description card) ── */
	.dd-trigger-wrap {
		margin-top: 1.25rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
	}

	.dd-trigger {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		background: transparent;
		border: 1px dashed var(--amber-dim);
		border-radius: 4px;
		padding: 0.6rem 0.875rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		font-family: var(--font-mono);
	}

	.dd-trigger:hover {
		background: var(--amber-glow);
		border-style: solid;
		border-color: var(--amber);
	}

	.dd-trigger-icon {
		font-size: 1.1rem;
		color: var(--amber);
		flex-shrink: 0;
		line-height: 1;
	}

	.dd-trigger-text {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		flex: 1;
	}

	.dd-trigger-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--amber);
	}

	.dd-trigger-sub {
		font-size: 0.68rem;
		color: var(--text-dim);
	}

	.dd-trigger-arrow {
		font-size: 1rem;
		color: var(--amber-dim);
		transition: transform 0.2s;
	}

	.dd-trigger:hover .dd-trigger-arrow {
		transform: translateX(3px);
		color: var(--amber);
	}

	/* ── Deep Dive Inline Loading (inside description card) ── */
	.dd-inline-loading {
		margin-top: 1.25rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.dd-loading-steps {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.dd-step {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.72rem;
		color: var(--text-dim);
	}

	.dd-step.active {
		color: var(--amber);
	}

	.dd-step-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--border);
		flex-shrink: 0;
	}

	.dd-step.active .dd-step-dot {
		background: var(--amber);
		animation: pulse-dot 1.5s ease-in-out infinite;
	}

	.dd-loading-hint {
		font-size: 0.65rem;
		color: var(--text-dim);
		opacity: 0.7;
	}

	.dd-loading-bar {
		height: 2px;
		background: var(--border);
		border-radius: 1px;
		overflow: hidden;
	}

	.dd-loading-progress {
		height: 100%;
		width: 40%;
		background: var(--amber);
		border-radius: 1px;
		animation: loadSlide 2s ease-in-out infinite;
	}

	@keyframes loadSlide {
		0% { transform: translateX(-100%); }
		50% { transform: translateX(200%); }
		100% { transform: translateX(-100%); }
	}

	/* ── Deep Dive Inline Error (inside description card) ── */
	.dd-inline-error {
		margin-top: 1.25rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.dd-error-icon {
		color: var(--red);
		font-size: 0.9rem;
		flex-shrink: 0;
	}

	.dd-error-msg {
		font-size: 0.8rem;
		color: var(--red);
		flex: 1;
	}

	/* ── Deep Dive Header ── */
	.dd-header-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.dd-title {
		font-size: 1rem;
		font-weight: 700;
		color: var(--amber);
	}

	.cursor {
		animation: blink 1s step-end infinite;
	}

	@keyframes blink { 50% { opacity: 0; } }

	.dd-meta {
		font-size: 0.65rem;
		color: var(--text-dim);
	}

	/* ── Deep Dive Grid ── */
	.dd-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	/* Make full-width cards for approach (longer content) and red flags */
	.dd-grid .dd-card:nth-child(3) {
		grid-column: 1 / -1;
	}

	.dd-grid .dd-card:nth-child(5) {
		grid-column: 1 / -1;
	}

	@media (max-width: 700px) {
		.dd-grid {
			grid-template-columns: 1fr;
		}
	}

	.dd-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.25rem;
		animation: cardIn 0.3s ease both;
		overflow: hidden;
	}

	@keyframes cardIn {
		from { opacity: 0; transform: translateY(6px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.dd-card-red {
		border-color: rgba(248, 113, 113, 0.25);
		background: rgba(248, 113, 113, 0.04);
	}

	.dd-card-num {
		font-size: 0.6rem;
		font-weight: 700;
		color: var(--amber);
		background: var(--amber-glow);
		display: inline-block;
		padding: 0.15rem 0.4rem;
		border-radius: 3px;
		margin-bottom: 0.5rem;
		letter-spacing: 0.05em;
	}

	.dd-num-red {
		color: var(--red);
		background: rgba(248, 113, 113, 0.1);
	}

	.dd-card-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.75rem;
		text-transform: lowercase;
	}

	.dd-title-red {
		color: var(--red);
	}

	/* ── Comments ── */
	.comments-loading {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem 0;
		color: var(--text-dim);
		font-size: 0.8rem;
	}

	.comment-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.comment {
		padding: 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-card);
		overflow: hidden;
	}

	.comment-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.comment-avatar {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		border: 1px solid var(--border);
	}

	.comment-author {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.comment-time {
		font-size: 0.7rem;
		color: var(--text-dim);
		margin-left: auto;
	}

	.comment-body {
		font-size: 0.82rem;
	}

	.no-comments {
		font-size: 0.8rem;
		color: var(--text-dim);
		text-align: center;
		padding: 1rem;
	}

	/* ── Mobile tweaks ── */
	@media (max-width: 600px) {
		.card {
			padding: 1rem;
		}
		.issue-header h1 {
			font-size: 1.1rem;
			min-width: 150px;
		}
		.issue-number {
			display: none;
		}
		.dd-loading-steps {
			flex-direction: column;
			gap: 0.4rem;
		}
	}
</style>
