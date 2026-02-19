<script lang="ts">
	import { api, type User, type Notification } from '$lib/api';
	import { onMount } from 'svelte';
	import { initTheme } from '$lib/theme';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { children } = $props();
	let user = $state<User | null>(null);
	let loading = $state(true);
	let error = $state('');

	// Notifications
	let unreadCount = $state(0);
	let showNotifDropdown = $state(false);
	let notifications = $state<Notification[]>([]);
	let notifLoading = $state(false);

	async function pollUnread() {
		try {
			const res = await api.getUnreadCount();
			unreadCount = res.count;
		} catch { /* silent */ }
	}

	async function toggleNotifDropdown() {
		showNotifDropdown = !showNotifDropdown;
		if (showNotifDropdown && notifications.length === 0) {
			notifLoading = true;
			try {
				const res = await api.getNotifications(1, 10);
				notifications = res.notifications;
			} catch { /* silent */ }
			notifLoading = false;
		}
	}

	async function markAllRead() {
		try {
			await api.markAllNotificationsRead();
			unreadCount = 0;
			notifications = notifications.map(n => ({ ...n, read: true }));
		} catch { /* silent */ }
	}

	async function markRead(id: string) {
		try {
			await api.markNotificationRead(id);
			unreadCount = Math.max(0, unreadCount - 1);
			notifications = notifications.map(n => n.id === id ? { ...n, read: true } : n);
		} catch { /* silent */ }
	}

	onMount(() => {
		initTheme();
		let timer: ReturnType<typeof setInterval> | undefined;

		(async () => {
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
				// Start notification polling
				pollUnread();
				timer = setInterval(pollUnread, 30000);
			} catch (e: any) {
				if (e.status === 401) {
					window.location.href = '/login';
					return;
				}
				error = e.message;
			} finally {
				loading = false;
			}
		})();

		return () => {
			if (timer) clearInterval(timer);
		};
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
				<a href="/app/awesome" class="nav-link">awesome</a>
				<a href="/app/hiring" class="nav-link">hiring</a>
				<a href="/app/profile" class="nav-link">profile</a>
				{#if user.role === 'admin'}
					<a href="/app/admin" class="nav-link admin-link">admin</a>
				{/if}
			</div>
			<div class="nav-user">
				<div class="notif-wrapper">
					<button class="notif-bell" onclick={toggleNotifDropdown}>
						&#9993;
						{#if unreadCount > 0}
							<span class="notif-badge">{unreadCount > 9 ? '9+' : unreadCount}</span>
						{/if}
					</button>
					{#if showNotifDropdown}
						<div class="notif-dropdown">
							<div class="notif-header">
								<span class="notif-title">notifications</span>
								{#if unreadCount > 0}
									<button class="notif-mark-all" onclick={markAllRead}>mark all read</button>
								{/if}
							</div>
							{#if notifLoading}
								<p class="notif-empty">loading...</p>
							{:else if notifications.length === 0}
								<p class="notif-empty">no notifications yet</p>
							{:else}
								{#each notifications as notif (notif.id)}
									<a
										href={notif.link || '#'}
										class="notif-item"
										class:notif-unread={!notif.read}
										onclick={() => { if (!notif.read) markRead(notif.id); showNotifDropdown = false; }}
									>
										<span class="notif-item-title">{notif.title}</span>
										<span class="notif-item-msg">{notif.message}</span>
									</a>
								{/each}
							{/if}
						</div>
					{/if}
				</div>
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

	/* Notifications */
	.notif-wrapper {
		position: relative;
	}

	.notif-bell {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 1.1rem;
		cursor: pointer;
		position: relative;
		padding: 0.25rem;
		transition: color 0.15s;
	}

	.notif-bell:hover {
		color: var(--text-bright);
	}

	.notif-badge {
		position: absolute;
		top: -4px;
		right: -6px;
		background: var(--amber);
		color: var(--bg);
		font-size: 0.6rem;
		font-weight: 700;
		min-width: 16px;
		height: 16px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
		padding: 0 3px;
	}

	.notif-dropdown {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		width: 320px;
		max-height: 400px;
		overflow-y: auto;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 6px;
		z-index: 200;
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
	}

	.notif-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border);
	}

	.notif-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text);
	}

	.notif-mark-all {
		background: none;
		border: none;
		color: var(--amber);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		cursor: pointer;
		padding: 0;
	}

	.notif-mark-all:hover {
		text-decoration: underline;
	}

	.notif-empty {
		padding: 1.5rem 1rem;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.notif-item {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border);
		text-decoration: none;
		transition: background 0.15s;
	}

	.notif-item:last-child {
		border-bottom: none;
	}

	.notif-item:hover {
		background: var(--bg-hover);
	}

	.notif-unread {
		border-left: 2px solid var(--amber);
	}

	.notif-item-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text);
	}

	.notif-item-msg {
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.4;
	}
</style>
