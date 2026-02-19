<script lang="ts">
	import { api, type AdminStats, type JobStatus } from '$lib/api';
	import { onMount } from 'svelte';

	let stats = $state<AdminStats | null>(null);
	let jobs = $state<JobStatus[]>([]);
	let loading = $state(true);
	let seeding = $state(false);
	let indexing = $state(false);

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
				<h2>recent jobs</h2>
				{#each jobs.slice(0, 3) as job}
					<div class="job-row">
						<span class="job-type">{job.type}</span>
						<span class="job-status" class:running={job.status === 'running'} class:completed={job.status === 'completed'} class:failed={job.status === 'failed'}>
							<span class="job-dot"></span>{job.status}
						</span>
						<span class="job-progress">{job.progress}</span>
					</div>
				{/each}
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

	.actions h2, .recent-jobs h2 {
		font-size: 0.85rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.75rem;
	}

	.action-grid {
		display: flex;
		gap: 0.75rem;
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

	.recent-jobs { margin-top: 1.5rem; }

	.job-row {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--border);
		font-size: 0.85rem;
	}

	.job-type {
		color: var(--text);
		font-weight: 500;
		min-width: 60px;
	}

	.job-status {
		font-size: 0.75rem;
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}
	.job-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-dim);
	}
	.job-status.running { color: var(--amber); }
	.job-status.running .job-dot { background: var(--amber); animation: pulse-dot 1.5s ease-in-out infinite; }
	.job-status.completed { color: var(--green); }
	.job-status.completed .job-dot { background: var(--green); }
	.job-status.failed { color: var(--red); }
	.job-status.failed .job-dot { background: var(--red); }

	@keyframes pulse-dot {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}

	.job-progress {
		color: var(--text-muted);
		font-size: 0.75rem;
		margin-left: auto;
	}
</style>
