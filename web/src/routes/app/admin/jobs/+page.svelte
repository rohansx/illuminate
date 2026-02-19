<script lang="ts">
	import { api, type JobStatus } from '$lib/api';
	import { onMount } from 'svelte';

	let jobs = $state<JobStatus[]>([]);
	let loading = $state(true);
	let pollTimer = $state<ReturnType<typeof setInterval> | null>(null);
	let expandedJob = $state<string | null>(null);

	onMount(() => {
		loadJobs();
		return () => {
			if (pollTimer) clearInterval(pollTimer);
		};
	});

	async function loadJobs() {
		try {
			jobs = await api.adminGetJobs();
			updatePolling();
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}

	function updatePolling() {
		const hasRunning = jobs.some((j) => j.status === 'running');
		if (hasRunning && !pollTimer) {
			pollTimer = setInterval(loadJobs, 2000);
		} else if (!hasRunning && pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
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

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
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

	function toggleExpand(id: string) {
		expandedJob = expandedJob === id ? null : id;
	}

	const runningJobs = $derived(jobs.filter(j => j.status === 'running'));
	const completedJobs = $derived(jobs.filter(j => j.status !== 'running'));
</script>

<div class="jobs-page">
	<header>
		<a href="/app/admin" class="back">&larr; admin</a>
		<h1>jobs<span class="cursor">_</span></h1>
		<p class="subtitle">
			background operations
			{#if pollTimer}<span class="live-badge"><span class="live-dot"></span> live</span>{/if}
		</p>
	</header>

	{#if loading}
		<div class="loading"><div class="spinner"></div></div>
	{:else if jobs.length}
		<!-- Running jobs section -->
		{#if runningJobs.length}
			<div class="job-section">
				<h2 class="section-label"><span class="section-dot running"></span> running ({runningJobs.length})</h2>
				<div class="job-list">
					{#each runningJobs as job (job.id)}
						<button class="job-card active" onclick={() => toggleExpand(job.id)}>
							<div class="job-header">
								<span class="job-type">{job.type}</span>
								<span class="job-status-badge" style="color: {statusColor(job.status)}; background: {statusBg(job.status)}">
									<span class="pulse"></span>
									{job.status}
								</span>
							</div>
							<div class="job-progress-bar">
								<div class="job-progress-fill running-fill"></div>
							</div>
							<div class="job-body">
								<span class="job-progress">{job.progress}</span>
								<span class="job-time">started {timeSince(job.started_at)}</span>
							</div>
							{#if expandedJob === job.id}
								<div class="job-detail">
									<div class="detail-row">
										<span class="detail-label">job id</span>
										<span class="detail-value">{job.id}</span>
									</div>
									<div class="detail-row">
										<span class="detail-label">started</span>
										<span class="detail-value">{formatDate(job.started_at)}</span>
									</div>
								</div>
							{/if}
							{#if job.error}
								<div class="job-error">{job.error}</div>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Completed / failed jobs -->
		{#if completedJobs.length}
			<div class="job-section">
				<h2 class="section-label"><span class="section-dot"></span> history ({completedJobs.length})</h2>
				<div class="job-list">
					{#each completedJobs as job (job.id)}
						<button class="job-card" onclick={() => toggleExpand(job.id)}>
							<div class="job-header">
								<span class="job-type">{job.type}</span>
								<span class="job-status-badge" style="color: {statusColor(job.status)}; background: {statusBg(job.status)}">
									{job.status}
								</span>
							</div>
							<div class="job-body">
								<span class="job-progress">{job.progress}</span>
								<span class="job-time">{timeSince(job.started_at)}</span>
							</div>
							{#if expandedJob === job.id}
								<div class="job-detail">
									<div class="detail-row">
										<span class="detail-label">job id</span>
										<span class="detail-value">{job.id}</span>
									</div>
									<div class="detail-row">
										<span class="detail-label">started</span>
										<span class="detail-value">{formatDate(job.started_at)}</span>
									</div>
									<div class="detail-row">
										<span class="detail-label">status</span>
										<span class="detail-value" style="color: {statusColor(job.status)}">{job.status}</span>
									</div>
								</div>
							{/if}
							{#if job.error}
								<div class="job-error">{job.error}</div>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}
	{:else}
		<div class="empty">
			<p>no jobs have been run yet.</p>
			<p class="hint">trigger a seed or index from the <a href="/app/admin">dashboard</a>.</p>
		</div>
	{/if}
</div>

<style>
	.jobs-page { max-width: 800px; }

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
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.live-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.7rem;
		color: var(--amber);
		background: var(--amber-glow);
		padding: 0.1rem 0.5rem;
		border-radius: 99px;
		border: 1px solid var(--amber-dim);
	}

	.live-dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--amber);
		animation: pulse-dot 1.5s ease-in-out infinite;
	}

	@keyframes pulse-dot {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
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

	/* ── Job sections ── */
	.job-section {
		margin-bottom: 2rem;
	}

	.section-label {
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: 0.75rem;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-weight: 500;
	}

	.section-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-dim);
	}

	.section-dot.running {
		background: #fbbf24;
		animation: pulse-dot 1.5s ease-in-out infinite;
	}

	.job-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.job-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.875rem 1.125rem;
		cursor: pointer;
		transition: all 0.15s;
		text-align: left;
		width: 100%;
		font-family: var(--font-mono);
		color: inherit;
	}

	.job-card:hover {
		border-color: var(--amber-dim);
		background: var(--bg-card);
	}

	.job-card.active {
		border-color: var(--amber-dim);
	}

	.job-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.4rem;
		gap: 0.5rem;
	}

	.job-type {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.job-status-badge {
		font-size: 0.68rem;
		padding: 0.1rem 0.5rem;
		border-radius: 99px;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-weight: 500;
	}

	.pulse {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: currentColor;
		animation: pulse-dot 1s ease-in-out infinite;
	}

	.job-progress-bar {
		height: 2px;
		background: var(--border);
		border-radius: 1px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.job-progress-fill {
		height: 100%;
		width: 40%;
		background: var(--amber);
		border-radius: 1px;
	}

	.running-fill {
		animation: loadSlide 2s ease-in-out infinite;
	}

	@keyframes loadSlide {
		0% { transform: translateX(-100%); }
		50% { transform: translateX(200%); }
		100% { transform: translateX(-100%); }
	}

	.job-body {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.job-progress {
		font-size: 0.78rem;
		color: var(--text-muted);
	}

	.job-time {
		font-size: 0.7rem;
		color: var(--text-dim);
		flex-shrink: 0;
	}

	/* ── Expandable detail ── */
	.job-detail {
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		animation: fadeIn 0.15s ease;
	}

	@keyframes fadeIn {
		from { opacity: 0; transform: translateY(-4px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.detail-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.detail-label {
		font-size: 0.68rem;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.detail-value {
		font-size: 0.75rem;
		color: var(--text);
		text-align: right;
		word-break: break-all;
	}

	.job-error {
		margin-top: 0.5rem;
		font-size: 0.78rem;
		color: var(--red);
		padding: 0.5rem;
		background: rgba(248, 113, 113, 0.05);
		border-radius: 4px;
		border: 1px solid rgba(248, 113, 113, 0.15);
	}

	.empty {
		text-align: center;
		padding: 3rem;
		color: var(--text-muted);
	}

	.hint {
		font-size: 0.85rem;
		margin-top: 0.5rem;
	}

	.hint a {
		color: var(--amber);
		text-decoration: none;
	}

	.hint a:hover { text-decoration: underline; }

	/* ── Responsive ── */
	@media (max-width: 600px) {
		.job-card {
			padding: 0.75rem;
		}
		.detail-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.15rem;
		}
		.detail-value {
			text-align: left;
		}
	}
</style>
