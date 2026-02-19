<script lang="ts">
	import { api } from '$lib/api';

	let comfortLevel = $state('beginner');
	let timeCommitment = $state('1-3 hours/week');
	let goals = $state<string[]>([]);
	let selectedSkills = $state<string[]>([]);
	let customSkill = $state('');
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

	const popularLanguages = [
		'Go', 'Python', 'JavaScript', 'TypeScript', 'Rust', 'Java',
		'C++', 'C#', 'Ruby', 'PHP', 'Swift', 'Kotlin', 'C',
		'Scala', 'Dart', 'Elixir', 'Haskell', 'Lua', 'R', 'Shell'
	];

	function toggleGoal(goal: string) {
		if (goals.includes(goal)) {
			goals = goals.filter(g => g !== goal);
		} else {
			goals = [...goals, goal];
		}
	}

	function toggleSkill(lang: string) {
		if (selectedSkills.includes(lang)) {
			selectedSkills = selectedSkills.filter(s => s !== lang);
		} else {
			selectedSkills = [...selectedSkills, lang];
		}
	}

	function addCustomSkill() {
		const trimmed = customSkill.trim();
		if (trimmed && !selectedSkills.includes(trimmed)) {
			selectedSkills = [...selectedSkills, trimmed];
		}
		customSkill = '';
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
			if (selectedSkills.length > 0) {
				await api.setSkills(selectedSkills);
			}
			// Trigger skill analysis in background (won't overwrite manual skills)
			api.analyzeSkills().catch(() => {});
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

		<div class="field">
			<label>what languages do you know? <span class="optional">(optional, improves your feed)</span></label>
			<div class="goal-grid skill-grid">
				{#each popularLanguages as lang}
					<button
						class="goal-btn"
						class:active={selectedSkills.includes(lang)}
						onclick={() => toggleSkill(lang)}
					>
						{lang}
					</button>
				{/each}
			</div>
			<div class="custom-skill-row">
				<input
					type="text"
					placeholder="add another language..."
					bind:value={customSkill}
					onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addCustomSkill())}
					class="custom-skill-input"
				/>
				{#if customSkill.trim()}
					<button class="add-skill-btn" onclick={addCustomSkill}>add</button>
				{/if}
			</div>
			{#if selectedSkills.filter(s => !popularLanguages.includes(s)).length > 0}
				<div class="custom-skills-list">
					{#each selectedSkills.filter(s => !popularLanguages.includes(s)) as skill}
						<span class="skill-chip">
							{skill}
							<button class="chip-remove" onclick={() => toggleSkill(skill)}>&times;</button>
						</span>
					{/each}
				</div>
			{/if}
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
	.dot.yellow { background: #95EDBE; }
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

	.optional {
		color: var(--text-dim);
		font-weight: 400;
	}

	.skill-grid {
		grid-template-columns: repeat(4, 1fr);
	}

	.custom-skill-row {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.custom-skill-input {
		flex: 1;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		outline: none;
		transition: border-color 0.15s;
	}

	.custom-skill-input::placeholder { color: var(--text-dim); }
	.custom-skill-input:focus { border-color: var(--amber-dim); }

	.add-skill-btn {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.5rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.add-skill-btn:hover {
		background: var(--amber-glow);
		border-color: var(--amber-dim);
	}

	.custom-skills-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
		margin-top: 0.5rem;
	}

	.skill-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		background: var(--amber-glow);
		border: 1px solid var(--amber-dim);
		border-radius: 3px;
		color: var(--amber);
	}

	.chip-remove {
		background: none;
		border: none;
		color: var(--text-dim);
		font-size: 0.9rem;
		cursor: pointer;
		padding: 0;
		line-height: 1;
		transition: color 0.15s;
	}

	.chip-remove:hover { color: var(--red); }

	@media (max-width: 500px) {
		.goal-grid { grid-template-columns: 1fr; }
		.skill-grid { grid-template-columns: repeat(3, 1fr); }
	}
</style>
