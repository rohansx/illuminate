<script lang="ts">
	import { api, type JobStatus } from '$lib/api';
	import { onMount } from 'svelte';

	let jobs = $state<JobStatus[]>([]);
	let loading = $state(true);
	let pollTimer = $state<ReturnType<typeof setInterval> | null>(null);

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
		return `${hours}h ago`;
	}
</script>

<div class="jobs-page">
	<header>
		<a href="/app/admin" class="back">&larr; admin</a>
		<h1>jobs<span class="cursor">_</span></h1>
		<p class="subtitle">background operations {#if pollTimer}<span class="live-dot"></span> live{/if}</p>
	</header>

	{#if loading}
		<div class="loading"><div class="spinner"></div></div>
	{:else if jobs.length}
		<div class="job-list">
			{#each jobs as job (job.id)}
				<div class="job-card" class:running={job.status === 'running'}>
					<div class="job-header">
						<span class="job-type">{job.type}</span>
						<span
							class="job-status"
							class:status-running={job.status === 'running'}
							class:status-completed={job.status === 'completed'}
							class:status-failed={job.status === 'failed'}
						>
							{#if job.status === 'running'}<span class="pulse"></span>{/if}
							{job.status}
						</span>
					</div>
					<div class="job-body">
						<span class="job-progress">{job.progress}</span>
						<span class="job-time">{timeSince(job.started_at)}</span>
					</div>
					{#if job.error}
						<div class="job-error">{job.error}</div>
					{/if}
				</div>
			{/each}
		</div>
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

	.live-dot {
		width: 6px;
		height: 6px;
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

	.job-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.job-card {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 1rem 1.25rem;
	}

	.job-card.running {
		border-color: var(--amber-dim);
	}

	.job-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}

	.job-type {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-bright);
	}

	.job-status {
		font-size: 0.75rem;
		padding: 0.125rem 0.5rem;
		border-radius: 3px;
		display: flex;
		align-items: center;
		gap: 0.375rem;
		background: rgba(255, 255, 255, 0.05);
		color: var(--text-muted);
	}

	.status-running {
		background: var(--amber-glow);
		color: var(--amber);
	}

	.status-completed {
		background: rgba(74, 222, 128, 0.1);
		color: var(--green);
	}

	.status-failed {
		background: rgba(248, 113, 113, 0.1);
		color: var(--red);
	}

	.pulse {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
		animation: pulse-dot 1s ease-in-out infinite;
	}

	.job-body {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.job-progress {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.job-time {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.job-error {
		margin-top: 0.5rem;
		font-size: 0.8rem;
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
</style>
