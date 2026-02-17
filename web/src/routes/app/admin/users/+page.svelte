<script lang="ts">
	import { api, type AdminUserList } from '$lib/api';
	import { onMount } from 'svelte';

	let data = $state<AdminUserList | null>(null);
	let loading = $state(true);
	let page = $state(1);

	onMount(() => loadUsers());

	async function loadUsers() {
		loading = true;
		try {
			data = await api.adminListUsers(page);
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function toggleRole(userId: string, currentRole: string) {
		const newRole = currentRole === 'admin' ? 'user' : 'admin';
		try {
			await api.adminUpdateRole(userId, newRole);
			await loadUsers();
		} catch (e: any) {
			alert(e.message);
		}
	}
</script>

<div class="users-page">
	<header>
		<a href="/app/admin" class="back">&larr; admin</a>
		<h1>users<span class="cursor">_</span></h1>
		{#if data}
			<p class="subtitle">{data.total_count} total users</p>
		{/if}
	</header>

	{#if loading}
		<div class="loading"><div class="spinner"></div></div>
	{:else if data?.users?.length}
		<div class="user-list">
			{#each data.users as user (user.id)}
				<div class="user-row">
					<img src={user.avatar_url} alt={user.github_username} class="avatar" />
					<div class="user-info">
						<span class="username">{user.github_username}</span>
						<span class="meta">joined {new Date(user.created_at).toLocaleDateString()}</span>
					</div>
					<span class="role-badge" class:admin={user.role === 'admin'}>{user.role}</span>
					<button class="role-btn" onclick={() => toggleRole(user.id, user.role)}>
						{user.role === 'admin' ? 'demote' : 'promote'}
					</button>
				</div>
			{/each}
		</div>

		{#if data.total_count > data.per_page}
			<div class="pagination">
				<button disabled={page <= 1} onclick={() => { page--; loadUsers(); }}>prev</button>
				<span class="page-info">page {data.page} of {Math.ceil(data.total_count / data.per_page)}</span>
				<button disabled={page * data.per_page >= data.total_count} onclick={() => { page++; loadUsers(); }}>next</button>
			</div>
		{/if}
	{:else}
		<div class="empty"><p>no users found.</p></div>
	{/if}
</div>

<style>
	.users-page { max-width: 800px; }

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

	.user-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.user-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		border: 1px solid var(--border);
	}

	.user-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.username {
		font-size: 0.9rem;
		color: var(--text-bright);
		font-weight: 500;
	}

	.meta {
		font-size: 0.7rem;
		color: var(--text-dim);
	}

	.role-badge {
		font-size: 0.7rem;
		padding: 0.125rem 0.5rem;
		border-radius: 3px;
		background: rgba(255, 255, 255, 0.05);
		color: var(--text-muted);
	}

	.role-badge.admin {
		background: var(--amber-glow);
		color: var(--amber);
	}

	.role-btn {
		background: none;
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		padding: 0.25rem 0.5rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.15s;
	}

	.role-btn:hover {
		border-color: var(--amber);
		color: var(--amber);
	}

	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		margin-top: 1.5rem;
	}

	.pagination button {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		padding: 0.375rem 0.75rem;
		cursor: pointer;
		border-radius: 4px;
	}

	.pagination button:disabled { opacity: 0.3; cursor: not-allowed; }

	.page-info {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.empty {
		text-align: center;
		padding: 3rem;
		color: var(--text-muted);
	}
</style>
