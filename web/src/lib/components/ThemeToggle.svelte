<script lang="ts">
	import { THEMES, getTheme, setTheme, type ThemeId } from '$lib/theme';
	import { onMount } from 'svelte';

	let current = $state<ThemeId>('forest');
	let open = $state(false);

	onMount(() => {
		current = getTheme();
	});

	function pick(id: ThemeId) {
		current = id;
		setTheme(id);
		open = false;
	}

	function handleClickOutside(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (!target.closest('.theme-toggle')) {
			open = false;
		}
	}
</script>

<svelte:window onclick={handleClickOutside} />

<div class="theme-toggle">
	<button class="toggle-btn" onclick={() => open = !open} title="Switch theme">
		<span class="swatch" style="background: {THEMES.find(t => t.id === current)?.accent}"></span>
	</button>

	{#if open}
		<div class="dropdown">
			<div class="dropdown-label">theme</div>
			{#each THEMES as theme}
				<button
					class="theme-option"
					class:active={current === theme.id}
					onclick={() => pick(theme.id)}
				>
					<span class="option-swatch" style="background: {theme.accent}"></span>
					<span class="option-label">{theme.label}</span>
					{#if current === theme.id}
						<span class="check">*</span>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.theme-toggle {
		position: relative;
	}

	.toggle-btn {
		background: none;
		border: 1px solid var(--border);
		padding: 0.3rem;
		cursor: pointer;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-color 0.15s;
	}

	.toggle-btn:hover {
		border-color: var(--border-bright);
	}

	.swatch {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		display: block;
	}

	.dropdown {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.375rem;
		min-width: 150px;
		z-index: 200;
		animation: fade-in 0.12s ease;
	}

	@keyframes fade-in {
		from { opacity: 0; transform: translateY(-4px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.dropdown-label {
		font-size: 0.65rem;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		padding: 0.25rem 0.5rem 0.375rem;
	}

	.theme-option {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		background: none;
		border: none;
		padding: 0.375rem 0.5rem;
		cursor: pointer;
		border-radius: 4px;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--text-muted);
		transition: all 0.1s;
	}

	.theme-option:hover {
		background: var(--amber-glow);
		color: var(--text);
	}

	.theme-option.active {
		color: var(--amber);
	}

	.option-swatch {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.option-label {
		flex: 1;
		text-align: left;
	}

	.check {
		color: var(--amber);
		font-weight: 700;
	}
</style>
