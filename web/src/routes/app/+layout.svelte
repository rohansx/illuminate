<script lang="ts">
	import { api, type User } from '$lib/api';
	import { onMount } from 'svelte';
	import { initTheme } from '$lib/theme';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { children } = $props();
	let user = $state<User | null>(null);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		initTheme();
		try {
			user = await api.getMe();
			if (!user.onboarding_done && !window.location.pathname.includes('/onboarding')) {
				window.location.href = '/app/onboarding';
				return;
			}
			if (user.onboarding_done && window.location.pathname.includes('/onboarding')) {
				window.location.href = '/app/feed';
				return;
			}
		} catch (e: any) {
			if (e.status === 401) {
				window.location.href = '/login';
				return;
			}
			error = e.message;
		} finally {
			loading = false;
		}
	});
</script>

{#if loading}
	<div class="app-loading">
		<div class="spinner"></div>
		<p>loading...</p>
	</div>
{:else if error}
	<div class="app-error">
		<p class="error-text">{error}</p>
		<a href="/" class="back-link">&larr; back to home</a>
	</div>
{:else if user}
	<div class="app-shell">
		<nav class="app-nav">
			<a href="/app/feed" class="nav-logo">illuminate<span class="cursor">_</span></a>
			<div class="nav-links">
				<a href="/app/feed" class="nav-link">feed</a>
				<a href="/app/saved" class="nav-link">saved</a>
				<a href="/app/profile" class="nav-link">profile</a>
				{#if user.role === 'admin'}
					<a href="/app/admin" class="nav-link admin-link">admin</a>
				{/if}
			</div>
			<div class="nav-user">
				<ThemeToggle />
				<img src={user.avatar_url} alt={user.github_username} class="avatar" />
				<span class="username">{user.github_username}</span>
				<button class="logout-btn" onclick={() => { api.logout(); window.location.href = '/'; }}>
					logout
				</button>
			</div>
		</nav>
		<main class="app-main">
			{@render children()}
		</main>
	</div>
{/if}

<style>
	.app-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
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

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.app-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 1rem;
	}

	.error-text { color: var(--red); }

	.back-link {
		color: var(--amber);
		text-decoration: none;
	}

	.app-shell {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.app-nav {
		display: flex;
		align-items: center;
		padding: 0.75rem 1.5rem;
		border-bottom: 1px solid var(--border);
		background: var(--bg);
		position: sticky;
		top: 0;
		z-index: 100;
	}

	.nav-logo {
		color: var(--amber);
		text-decoration: none;
		font-weight: 700;
		font-size: 1rem;
		margin-right: 2rem;
	}

	.cursor {
		animation: blink 1s step-end infinite;
	}

	@keyframes blink {
		50% { opacity: 0; }
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
		flex: 1;
	}

	.nav-link {
		color: var(--text-muted);
		text-decoration: none;
		font-size: 0.85rem;
		transition: color 0.15s;
	}

	.nav-link:hover { color: var(--text-bright); }
	.admin-link { color: var(--amber); }

	.nav-user {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.avatar {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		border: 1px solid var(--border);
	}

	.username {
		color: var(--text);
		font-size: 0.85rem;
	}

	.logout-btn {
		background: none;
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.logout-btn:hover {
		border-color: var(--red);
		color: var(--red);
	}

	.app-main {
		flex: 1;
		max-width: 1280px;
		width: 100%;
		margin: 0 auto;
		padding: 2rem 2rem;
	}
</style>
