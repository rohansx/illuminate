const API_BASE = import.meta.env.VITE_API_BASE || '';

class ApiError extends Error {
	status: number;
	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		credentials: 'include',
		headers: { 'Content-Type': 'application/json', ...options.headers },
		...options
	});

	if (res.status === 401) {
		// Try refresh
		const refreshRes = await fetch(`${API_BASE}/auth/refresh`, {
			method: 'POST',
			credentials: 'include'
		});

		if (refreshRes.ok) {
			// Retry original request
			const retryRes = await fetch(`${API_BASE}${path}`, {
				credentials: 'include',
				headers: { 'Content-Type': 'application/json', ...options.headers },
				...options
			});
			if (!retryRes.ok) {
				throw new ApiError(retryRes.status, 'Request failed after refresh');
			}
			return retryRes.json();
		}

		// Redirect to login
		window.location.href = '/login';
		throw new ApiError(401, 'Unauthorized');
	}

	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Unknown error' }));
		throw new ApiError(res.status, body.error || 'Request failed');
	}

	return res.json();
}

export interface User {
	id: string;
	github_id: number;
	github_username: string;
	avatar_url: string;
	bio: string;
	role: string;
	comfort_level: string;
	time_commitment: string;
	goals: string[];
	onboarding_done: boolean;
	skills: UserSkill[];
}

export interface UserSkill {
	language: string;
	proficiency: number;
	source: string;
}

export interface UserProfile {
	comfort_level: string;
	time_commitment: string;
	goals: string[];
}

export interface Issue {
	id: string;
	github_id: number;
	repo_id: string;
	number: number;
	title: string;
	body: string;
	summary: string;
	labels: string[];
	difficulty: number;
	time_estimate: string;
	status: string;
	comment_count: number;
	freshness_score: number;
	match_score?: number;
	match_reasons?: string[];
	is_saved?: boolean;
	repo?: Repository;
	skills?: IssueSkill[];
}

export interface Repository {
	id: string;
	owner: string;
	name: string;
	description: string;
	stars: number;
	primary_language: string;
	topics: string[];
	health_score: number;
}

export interface IssueSkill {
	language: string;
	framework?: string;
}

export interface DeepDive {
	id: string;
	issue_id: string;
	user_id: string;
	project_overview: string;
	issue_context: string;
	suggested_approach: string;
	questions_to_ask: string;
	red_flags: string;
	model_used: string;
	prompt_tokens: number;
	completion_tokens: number;
	created_at: string;
}

export interface IssueFeed {
	issues: Issue[];
	total_count: number;
	page: number;
	per_page: number;
}

export interface AdminStats {
	user_count: number;
	repo_count: number;
	issue_count: number;
}

export interface AdminUserListItem {
	id: string;
	github_username: string;
	avatar_url: string;
	role: string;
	onboarding_done: boolean;
	created_at: string;
}

export interface AdminUserList {
	users: AdminUserListItem[];
	total_count: number;
	page: number;
	per_page: number;
}

export interface Category {
	id: string;
	name: string;
	slug: string;
	description: string;
	icon: string;
}

export interface AdminRepoListItem {
	id: string;
	owner: string;
	name: string;
	stars: number;
	primary_language: string;
	issue_count: number;
	indexed_at: string;
	tags: string[];
	difficulty_level: string;
	activity_status: string;
	categories?: Category[];
}

export interface AdminRepoList {
	repos: AdminRepoListItem[];
	total_count: number;
	page: number;
	per_page: number;
}

export interface GitHubComment {
	id: number;
	body: string;
	created_at: string;
	updated_at: string;
	user: {
		login: string;
		avatar_url: string;
	};
}

export interface SavedStatus {
	saved: boolean;
}

export interface JobStatus {
	id: string;
	type: string;
	status: string;
	progress: string;
	started_at: string;
	error?: string;
}

export interface ProfileStats {
	user: User;
	merged_pr_count: number;
	open_pr_count: number;
	saved_count: number;
}

export interface GitHubPR {
	id: number;
	number: number;
	title: string;
	state: string;
	html_url: string;
	created_at: string;
	updated_at: string;
	closed_at: string;
	pull_request?: {
		merged_at: string;
	};
	repository_url: string;
	labels: { name: string }[];
}

export interface PRList {
	total_count: number;
	items: GitHubPR[];
}

export const api = {
	getMe: () => request<User>('/api/users/me'),

	updateProfile: (profile: UserProfile) =>
		request('/api/users/me/profile', {
			method: 'PATCH',
			body: JSON.stringify(profile)
		}),

	getProfileStats: () => request<ProfileStats>('/api/users/me/stats'),

	getMyPRs: (type: 'merged' | 'open' = 'merged', page = 1, perPage = 20) => {
		const params = new URLSearchParams({ type, page: String(page), per_page: String(perPage) });
		return request<PRList>(`/api/users/me/prs?${params}`);
	},

	getFeed: (page = 1, perPage = 20, filters?: { languages?: string[]; difficulty?: number; category?: string }) => {
		const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
		if (filters?.languages?.length) params.set('languages', filters.languages.join(','));
		if (filters?.difficulty) params.set('difficulty', String(filters.difficulty));
		if (filters?.category) params.set('category', filters.category);
		return request<IssueFeed>(`/api/issues/feed?${params}`);
	},

	getCategories: () => request<Category[]>('/api/categories'),

	getIssue: (id: string) => request<Issue>(`/api/issues/${id}`),

	getDeepDive: (issueId: string) => request<DeepDive>(`/api/issues/${issueId}/deep-dive`),

	searchIssues: (query: string, page = 1, perPage = 20) => {
		const params = new URLSearchParams({ q: query, page: String(page), per_page: String(perPage) });
		return request<IssueFeed>(`/api/issues/search?${params}`);
	},

	getIssueComments: (issueId: string) =>
		request<GitHubComment[]>(`/api/issues/${issueId}/comments`),

	saveIssue: (issueId: string) =>
		request('/api/issues/' + issueId + '/save', { method: 'POST' }),

	unsaveIssue: (issueId: string) =>
		request('/api/issues/' + issueId + '/save', { method: 'DELETE' }),

	isIssueSaved: (issueId: string) =>
		request<SavedStatus>('/api/issues/' + issueId + '/saved'),

	getSavedIssues: (page = 1, perPage = 20) => {
		const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
		return request<IssueFeed>(`/api/issues/saved?${params}`);
	},

	logout: () =>
		request('/auth/logout', { method: 'POST' }),

	loginURL: `${API_BASE}/auth/github/login`,

	// Admin endpoints
	adminGetStats: () => request<AdminStats>('/admin/stats'),

	adminListUsers: (page = 1, perPage = 50) => {
		const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
		return request<AdminUserList>(`/admin/users?${params}`);
	},

	adminUpdateRole: (userId: string, role: string) =>
		request('/admin/users/' + userId + '/role', {
			method: 'PATCH',
			body: JSON.stringify({ role })
		}),

	adminTriggerSeed: () =>
		request<JobStatus>('/admin/seed', { method: 'POST' }),

	adminTriggerIndex: () =>
		request<JobStatus>('/admin/index', { method: 'POST' }),

	adminGetJobs: () =>
		request<JobStatus[]>('/admin/jobs'),

	adminListRepos: (page = 1, perPage = 50) => {
		const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
		return request<AdminRepoList>(`/admin/repos?${params}`);
	},

	adminDeleteRepo: (repoId: string) =>
		request('/admin/repos/' + repoId, { method: 'DELETE' }),

	adminUpdateRepoMetadata: (repoId: string, metadata: { tags: string[]; difficulty_level: string; activity_status: string }) =>
		request('/admin/repos/' + repoId + '/metadata', {
			method: 'PATCH',
			body: JSON.stringify(metadata)
		}),

	adminGetCategories: () =>
		request<Category[]>('/admin/categories'),

	adminAssignCategory: (repoId: string, categoryId: string) =>
		request('/admin/repos/' + repoId + '/categories', {
			method: 'POST',
			body: JSON.stringify({ category_id: categoryId })
		}),

	adminRemoveCategory: (repoId: string, categoryId: string) =>
		request('/admin/repos/' + repoId + '/categories/' + categoryId, {
			method: 'DELETE'
		}),
};
