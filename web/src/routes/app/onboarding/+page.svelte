<script lang="ts">
	import { api } from '$lib/api';

	let comfortLevel = $state('beginner');
	let timeCommitment = $state('1-3 hours/week');
	let goals = $state<string[]>([]);
	let submitting = $state(false);
	let error = $state('');

	const goalOptions = [
		'Learn a new language',
		'Build my portfolio',
		'Give back to OSS',
		'Get hired',
		'Level up my skills',
		'Meet other developers'
	];

	function toggleGoal(goal: string) {
		if (goals.includes(goal)) {
			goals = goals.filter(g => g !== goal);
		} else {
			goals = [...goals, goal];
		}
	}

	async function submit() {
		if (goals.length === 0) {
			error = 'select at least one goal';
			return;
		}
		submitting = true;
		error = '';
		try {
			await api.updateProfile({
				comfort_level: comfortLevel,
				time_commitment: timeCommitment,
				goals
			});
			window.location.href = '/app/feed';
		} catch (e: any) {
			error = e.message || 'Something went wrong';
		} finally {
			submitting = false;
		}
	}
</script>

<div class="onboarding">
	<div class="terminal-header">
		<span class="dot red"></span>
		<span class="dot yellow"></span>
		<span class="dot green"></span>
		<span class="title">illuminate --setup</span>
	</div>

	<div class="content">
		<h1>let's personalize your feed<span class="cursor">_</span></h1>
		<p class="subtitle">answer a few questions so we can match you with the right issues</p>

		<div class="field">
			<label>comfort level with open source</label>
			<div class="radio-group">
				{#each ['beginner', 'intermediate', 'advanced'] as level}
					<button
						class="radio-btn"
						class:active={comfortLevel === level}
						onclick={() => comfortLevel = level}
					>
						{level}
					</button>
				{/each}
			</div>
		</div>

		<div class="field">
			<label>time you can commit</label>
			<div class="radio-group">
				{#each ['1-3 hours/week', '3-6 hours/week', '6+ hours/week'] as time}
					<button
						class="radio-btn"
						class:active={timeCommitment === time}
						onclick={() => timeCommitment = time}
					>
						{time}
					</button>
				{/each}
			</div>
		</div>

		<div class="field">
			<label>what are your goals? (select all that apply)</label>
			<div class="goal-grid">
				{#each goalOptions as goal}
					<button
						class="goal-btn"
						class:active={goals.includes(goal)}
						onclick={() => toggleGoal(goal)}
					>
						{goal}
					</button>
				{/each}
			</div>
		</div>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<button class="submit-btn" onclick={submit} disabled={submitting}>
			{submitting ? 'saving...' : 'start exploring →'}
		</button>
	</div>
</div>

<style>
	.onboarding {
		max-width: 640px;
		margin: 0 auto;
	}

	.terminal-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-bottom: none;
		border-radius: 8px 8px 0 0;
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
	}

	.dot.red { background: #f87171; }
	.dot.yellow { background: #fbbf24; }
	.dot.green { background: #4ade80; }

	.title {
		color: var(--text-muted);
		font-size: 0.75rem;
		margin-left: 0.5rem;
	}

	.content {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 0 0 8px 8px;
		padding: 2rem;
	}

	h1 {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 0.5rem;
	}

	.cursor { animation: blink 1s step-end infinite; color: var(--amber); }
	@keyframes blink { 50% { opacity: 0; } }

	.subtitle {
		color: var(--text-muted);
		font-size: 0.85rem;
		margin-bottom: 2rem;
	}

	.field {
		margin-bottom: 1.5rem;
	}

	label {
		display: block;
		color: var(--text);
		font-size: 0.85rem;
		margin-bottom: 0.75rem;
	}

	.radio-group {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.radio-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.5rem 1rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.radio-btn:hover {
		border-color: var(--border-bright);
		color: var(--text);
	}

	.radio-btn.active {
		border-color: var(--amber);
		color: var(--amber);
		background: var(--amber-glow);
	}

	.goal-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}

	.goal-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.6rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		text-align: left;
		transition: all 0.15s;
	}

	.goal-btn:hover {
		border-color: var(--border-bright);
		color: var(--text);
	}

	.goal-btn.active {
		border-color: var(--amber);
		color: var(--amber);
		background: var(--amber-glow);
	}

	.error {
		color: var(--red);
		font-size: 0.85rem;
		margin-bottom: 1rem;
	}

	.submit-btn {
		width: 100%;
		background: var(--amber);
		color: var(--bg);
		font-family: var(--font-mono);
		font-size: 0.9rem;
		font-weight: 600;
		padding: 0.75rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		margin-top: 0.5rem;
	}

	.submit-btn:hover { background: var(--amber-bright); }
	.submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

	@media (max-width: 500px) {
		.goal-grid { grid-template-columns: 1fr; }
	}
</style>
