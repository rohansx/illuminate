<script lang="ts">
	import { api, type AdminStats, type JobStatus } from '$lib/api';
	import { onMount } from 'svelte';

	let stats = $state<AdminStats | null>(null);
	let jobs = $state<JobStatus[]>([]);
	let loading = $state(true);
	let seeding = $state(false);
	let indexing = $state(false);
	let syncingContribs = $state(false);

	onMount(async () => {
		try {
			[stats, jobs] = await Promise.all([
				api.adminGetStats(),
				api.adminGetJobs()
			]);
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	});

	async function triggerSeed() {
		seeding = true;
		try {
			await api.adminTriggerSeed();
			jobs = await api.adminGetJobs();
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
			jobs = await api.adminGetJobs();
		} catch (e: any) {
			alert(e.message);
		} finally {
			indexing = false;
		}
	}

	async function triggerContribSync() {
		syncingContribs = true;
		try {
			await api.adminTriggerContributionSync();
			jobs = await api.adminGetJobs();
		} catch (e: any) {
			alert(e.message);
		} finally {
			syncingContribs = false;
		}
	}

	function timeSince(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return `${seconds}s ago`;
		const minutes = Math.floor(seconds / 60);
		if (minutes < 60) return `${minutes}m ago`;
		const hours = Math.floor(minutes / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}

	function statusColor(status: string): string {
		if (status === 'running') return '#fbbf24';
		if (status === 'completed') return '#4ade80';
		if (status === 'failed') return '#f87171';
		return 'var(--text-dim)';
	}

	function statusBg(status: string): string {
		if (status === 'running') return 'rgba(251, 191, 36, 0.1)';
		if (status === 'completed') return 'rgba(74, 222, 128, 0.1)';
		if (status === 'failed') return 'rgba(248, 113, 113, 0.1)';
		return 'rgba(255,255,255,0.05)';
	}
</script>

<div class="admin-page">
	<header class="admin-header">
		<h1>admin dashboard<span class="cursor">_</span></h1>
		<p class="subtitle">system overview and management</p>
	</header>

	{#if loading}
		<div class="loading"><div class="spinner"></div></div>
	{:else if stats}
		<div class="stat-grid">
			<div class="stat-card">
				<span class="stat-value" style="color: #60a5fa">{stats.user_count}</span>
				<span class="stat-label">users</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #c084fc">{stats.repo_count}</span>
				<span class="stat-label">repositories</span>
			</div>
			<div class="stat-card">
				<span class="stat-value" style="color: #4ade80">{stats.issue_count}</span>
				<span class="stat-label">issues indexed</span>
			</div>
		</div>

		<div class="actions">
			<h2>quick actions</h2>
			<div class="action-grid">
				<button class="action-btn" onclick={triggerSeed} disabled={seeding}>
					{seeding ? 'seeding...' : 'seed repos'}
				</button>
				<button class="action-btn" onclick={triggerIndex} disabled={indexing}>
					{indexing ? 'indexing...' : 'index issues'}
				</button>
				<button class="action-btn" onclick={triggerContribSync} disabled={syncingContribs}>
					{syncingContribs ? 'syncing...' : 'sync contributions'}
				</button>
			</div>
		</div>

		<div class="nav-grid">
			<a href="/app/admin/users" class="nav-card" style="border-top: 2px solid #60a5fa">
				<span class="nav-icon" style="color: #60a5fa">&#9679;</span>
				<span class="nav-title">users</span>
				<span class="nav-desc">manage users and roles</span>
			</a>
			<a href="/app/admin/repos" class="nav-card" style="border-top: 2px solid #c084fc">
				<span class="nav-icon" style="color: #c084fc">&#9679;</span>
				<span class="nav-title">repositories</span>
				<span class="nav-desc">view and manage indexed repos</span>
			</a>
			<a href="/app/admin/jobs" class="nav-card" style="border-top: 2px solid #4ade80">
				<span class="nav-icon" style="color: #4ade80">&#9679;</span>
				<span class="nav-title">jobs</span>
				<span class="nav-desc">view running operations</span>
			</a>
		</div>

		{#if jobs?.length}
			<div class="recent-jobs">
				<div class="rj-header">
					<h2>recent jobs</h2>
					<a href="/app/admin/jobs" class="rj-view-all">view all &rarr;</a>
				</div>
				<div class="rj-list">
					{#each jobs.slice(0, 5) as job}
						<div class="rj-card" style="border-left: 3px solid {statusColor(job.status)}">
							<div class="rj-top">
								<span class="rj-type">{job.type}</span>
								<span class="rj-status" style="color: {statusColor(job.status)}; background: {statusBg(job.status)}">
									{#if job.status === 'running'}<span class="rj-pulse"></span>{/if}
									{job.status}
								</span>
							</div>
							<div class="rj-details">
								<span class="rj-progress">{job.progress}</span>
								<span class="rj-time">{timeSince(job.started_at)}</span>
							</div>
							{#if job.error}
								<div class="rj-error">{job.error}</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.admin-page { max-width: 800px; }

	.admin-header { margin-bottom: 2rem; }

	.admin-header h1 {
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

	.stat-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1rem;
		margin-bottom: 2rem;
	}

	@media (max-width: 600px) {
		.stat-grid {
			grid-template-columns: 1fr;
		}
		.nav-grid {
			grid-template-columns: 1fr !important;
		}
	}

	.stat-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.stat-value {
		font-size: 2rem;
		font-weight: 700;
		color: var(--amber);
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.actions { margin-bottom: 2rem; }

	.actions h2 {
		font-size: 0.85rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.75rem;
	}

	.action-grid {
		display: flex;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.action-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.85rem;
		padding: 0.5rem 1.25rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.action-btn:hover:not(:disabled) { background: var(--amber-glow); }
	.action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

	.nav-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
		margin-bottom: 2rem;
	}

	.nav-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem 1.25rem;
		text-decoration: none;
		transition: all 0.15s;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.nav-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
	}

	.nav-icon {
		font-size: 0.5rem;
		line-height: 1;
	}

	.nav-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.nav-desc {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	/* ── Recent Jobs ── */
	.recent-jobs { margin-top: 0.5rem; }

	.rj-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.rj-header h2 {
		font-size: 0.85rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.rj-view-all {
		font-size: 0.75rem;
		color: var(--amber);
		text-decoration: none;
		transition: color 0.15s;
	}

	.rj-view-all:hover {
		color: var(--amber-bright);
	}

	.rj-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.rj-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem 1rem;
	}

	.rj-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		margin-bottom: 0.35rem;
	}

	.rj-type {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.rj-status {
		font-size: 0.7rem;
		padding: 0.1rem 0.5rem;
		border-radius: 99px;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-weight: 500;
	}

	.rj-pulse {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: currentColor;
		animation: pulse-dot 1.5s ease-in-out infinite;
	}

	@keyframes pulse-dot {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}

	.rj-details {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.rj-progress {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.rj-time {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.rj-error {
		margin-top: 0.4rem;
		font-size: 0.75rem;
		color: var(--red);
		padding: 0.35rem 0.5rem;
		background: rgba(248, 113, 113, 0.05);
		border-radius: 4px;
		border: 1px solid rgba(248, 113, 113, 0.15);
	}
</style>
